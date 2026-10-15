//! Bounded snapshots and a separate output owner with monotonic timeout safety.
use crate::protocol::ControllerState;
use tokio::{sync::watch, time::Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AcceptedState {
    pub state: ControllerState,
    pub received_at: Instant,
}

/// Copy snapshots immediately; no watch borrow may cross backend work or await.
pub type LatestReceiver = watch::Receiver<Option<AcceptedState>>;
pub struct LatestState(watch::Sender<Option<AcceptedState>>);
impl LatestState {
    pub fn channel() -> (Self, LatestReceiver) {
        let (tx, rx) = watch::channel(None);
        (Self(tx), rx)
    }
    pub(crate) fn publish(&self, state: ControllerState) {
        self.0.send_replace(Some(AcceptedState {
            state,
            received_at: Instant::now(),
        }));
    }
    pub(crate) fn clear(&self) {
        self.0.send_replace(None);
    }
}

pub const INPUT_TIMEOUT: std::time::Duration = std::time::Duration::from_millis(250);
pub const SESSION_IDLE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(3);

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum InputPhase {
    #[default]
    Unpaired,
    Active,
    TimedOut,
    BackendFault,
    ShuttingDown,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct OutputCounters {
    pub updates: u64,
    pub failures: u64,
    pub timeouts: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SessionRequest {
    pub session: crate::protocol::SessionId,
    pub expires_at: Instant,
    pub armed: bool,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct OutputHealth {
    pub session: Option<crate::protocol::SessionId>,
    pub ready: bool,
    pub attached: bool,
    pub timed_out: bool,
    pub fault: bool,
    pub diagnostic: Option<crate::virtual_gamepad::BackendDiagnostic>,
}

pub fn supported_controls(state: &ControllerState) -> bool {
    state.buttons & !0x3fff == 0
        && state.buttons & 0x0c00 != 0x0c00
        && state.buttons & 0x3000 != 0x3000
}

/// Called after validation/admission. Axes share signed range and positive-up Y.
/// Wire button order differs from XInput; keep this translation in one pure function.
pub fn xbox_report(state: &ControllerState) -> crate::virtual_gamepad::XboxReport {
    const XBOX_BITS: [u16; 14] = [
        0x1000, 0x2000, 0x4000, 0x8000, 0x0100, 0x0200, 0x0020, 0x0010, 0x0040, 0x0080, 0x0001,
        0x0002, 0x0004, 0x0008,
    ];
    let buttons = XBOX_BITS.iter().enumerate().fold(0, |mask, (bit, xbox)| {
        if state.buttons & (1 << bit) != 0 {
            mask | xbox
        } else {
            mask
        }
    });
    crate::virtual_gamepad::XboxReport {
        buttons,
        left_x: state.left_x,
        left_y: state.left_y,
        right_x: state.right_x,
        right_y: state.right_y,
        left_trigger: state.left_trigger,
        right_trigger: state.right_trigger,
    }
}

/// Deterministic owner step, with externally supplied monotonic time.
pub struct OutputOwner<B: crate::virtual_gamepad::VirtualGamepadBackend> {
    pad: crate::virtual_gamepad::Gamepad<B>,
    request: Option<SessionRequest>,
    last: Option<AcceptedState>,
    pub health: OutputHealth,
    pub phase: InputPhase,
    pub counters: OutputCounters,
    timeout: crate::config::InputTimeout,
}
impl<B: crate::virtual_gamepad::VirtualGamepadBackend> OutputOwner<B> {
    pub fn new(backend: B) -> Self {
        Self::with_timeout(backend, crate::config::InputTimeout::default())
    }

    pub fn with_timeout(backend: B, timeout: crate::config::InputTimeout) -> Self {
        Self {
            pad: crate::virtual_gamepad::Gamepad::new(backend),
            request: None,
            last: None,
            health: OutputHealth::default(),
            phase: InputPhase::Unpaired,
            counters: OutputCounters::default(),
            timeout,
        }
    }

    fn close(&mut self) {
        let was_faulted = self.health.fault;
        self.pad.shutdown();
        self.health.ready = false;
        self.health.attached = false;
        self.health.fault |= self.pad.health() == crate::virtual_gamepad::BackendHealth::Fault;
        if let Some(error) = self.pad.cleanup_errors().first() {
            let diagnostic = self
                .health
                .diagnostic
                .get_or_insert_with(|| error.diagnostic());
            diagnostic.cleanup_failed = true;
        }
        if self.health.fault {
            self.phase = InputPhase::BackendFault;
            if !was_faulted {
                self.counters.failures = self.counters.failures.saturating_add(1);
            }
        }
    }

    pub fn step(
        &mut self,
        request: Option<SessionRequest>,
        latest: Option<AcceptedState>,
        now: Instant,
    ) {
        self.step_checked(request, latest, now, &|| true);
    }

    fn step_checked(
        &mut self,
        request: Option<SessionRequest>,
        latest: Option<AcceptedState>,
        now: Instant,
        current: &dyn Fn() -> bool,
    ) {
        let new_id = request.map(|r| r.session);
        if new_id != self.request.map(|r| r.session) {
            self.close();
            self.request = request;
            self.last = None;
            self.health.session = new_id;
            self.health.timed_out = false;
            if request.is_some_and(|r| now < r.expires_at) && !self.health.fault {
                match self.pad.attach_checked(current) {
                    Ok(true) => {
                        self.phase = InputPhase::Active;
                        self.health.ready = true;
                        self.health.attached = true;
                    }
                    Ok(false) => {
                        self.close();
                        self.request = None;
                        self.health.session = None;
                        return;
                    }
                    Err(error) => {
                        tracing::error!(%error, recovery = %error.diagnostic().message(), "Backend unavailable");
                        self.health.diagnostic = Some(error.diagnostic());
                        self.health.fault = true;
                        self.phase = InputPhase::BackendFault;
                        self.counters.failures = self.counters.failures.saturating_add(1);
                        self.close();
                    }
                }
            }
        } else {
            self.request = request;
        }
        let Some(request) = request else {
            if !self.health.fault {
                self.phase = InputPhase::Unpaired;
            }
            return;
        };
        if !self.health.ready {
            return;
        }
        self.phase = if request.armed {
            InputPhase::Active
        } else {
            InputPhase::Unpaired
        };
        let latest = latest.filter(|s| {
            request.armed && s.state.session == request.session && supported_controls(&s.state)
        });
        if latest.is_none() && self.last.is_some() {
            // A published state stays in the slot until explicit revocation or
            // receiver shutdown. Clearing it is not expiry of the original
            // session grant deadline, even if the request channel lags the clear.
            self.close();
            if !self.health.fault {
                self.phase = InputPhase::Unpaired;
            }
            return;
        }
        let deadline = latest.map_or(request.expires_at, |s| {
            s.received_at + self.timeout.duration()
        });
        let session_deadline = if request.armed {
            deadline + (SESSION_IDLE_TIMEOUT - self.timeout.duration())
        } else {
            deadline
        };
        if now >= session_deadline {
            if request.armed && !self.health.timed_out {
                self.counters.timeouts = self.counters.timeouts.saturating_add(1);
            }
            self.close();
            self.health.timed_out = true;
            if !self.health.fault {
                self.phase = InputPhase::TimedOut;
            }
            tracing::warn!("Session idle deadline expired; neutral and detach requested");
            return;
        }
        if now >= deadline {
            self.phase = InputPhase::TimedOut;
            if self.health.timed_out {
                return;
            }
            let deadline_lateness_ms = now.duration_since(deadline).as_millis();
            let cleanup_started = Instant::now();
            if let Err(error) = self.pad.submit(crate::virtual_gamepad::XboxReport::NEUTRAL) {
                tracing::error!(%error, "Timeout neutral failed; backend closed");
                self.health.diagnostic = Some(error.diagnostic());
                self.close();
                return;
            }
            let cleanup_ms = cleanup_started.elapsed().as_millis();
            self.health.timed_out = true;
            if !self.health.fault {
                self.phase = InputPhase::TimedOut;
            }
            self.counters.timeouts = self.counters.timeouts.saturating_add(1);
            tracing::warn!(
                deadline_lateness_ms,
                cleanup_ms,
                "Valid input deadline expired; neutral retained for recovery"
            );
            return;
        }
        if let Some(state) = latest.filter(|state| Some(*state) != self.last) {
            if let Err(error) = self.pad.submit(xbox_report(&state.state)) {
                tracing::error!(%error, "Controller update failed; input refused until restart");
                self.health.diagnostic = Some(error.diagnostic());
                self.health.fault = true;
                self.counters.failures = self.counters.failures.saturating_add(1);
                self.close();
            } else {
                self.counters.updates = self.counters.updates.saturating_add(1);
                self.health.timed_out = false;
                self.last = Some(state);
            }
        }
    }

    pub fn shutdown(&mut self) -> Result<(), String> {
        self.close();
        self.phase = InputPhase::ShuttingDown;
        if self.pad.cleanup_errors().is_empty() {
            Ok(())
        } else {
            Err(self
                .pad
                .cleanup_errors()
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join("; "))
        }
    }
}

/// Production calls this on a single spawn_blocking worker, joined on shutdown.
pub async fn run_output<B: crate::virtual_gamepad::VirtualGamepadBackend>(
    backend: B,
    requests: watch::Receiver<Option<SessionRequest>>,
    latest: LatestReceiver,
    health: watch::Sender<OutputHealth>,
) -> Result<(), String> {
    run_output_with_timeout(
        backend,
        requests,
        latest,
        health,
        crate::config::InputTimeout::default(),
    )
    .await
}

pub async fn run_output_with_timeout<B: crate::virtual_gamepad::VirtualGamepadBackend>(
    backend: B,
    mut requests: watch::Receiver<Option<SessionRequest>>,
    mut latest: LatestReceiver,
    health: watch::Sender<OutputHealth>,
    timeout: crate::config::InputTimeout,
) -> Result<(), String> {
    let mut owner = OutputOwner::with_timeout(backend, timeout);
    let mut report_at = Instant::now() + std::time::Duration::from_secs(1);
    let mut reported_updates = 0;
    let mut tick = tokio::time::interval(std::time::Duration::from_millis(5));
    tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    loop {
        if requests.has_changed().is_err() || latest.has_changed().is_err() {
            break;
        }
        // Never retain a watch borrow during backend work or an await.
        let request = *requests.borrow_and_update();
        let state = *latest.borrow_and_update();
        owner.step_checked(request, state, Instant::now(), &|| {
            // The watch borrows here are short and never cross a driver call.
            requests.has_changed().is_ok()
                && latest.has_changed().is_ok()
                && request.is_some_and(|original| {
                    requests
                        .borrow()
                        .is_some_and(|live| live.session == original.session)
                        && Instant::now() < original.expires_at
                })
        });
        if Instant::now() >= report_at {
            if owner.counters.updates != reported_updates {
                tracing::info!(target: "virtual_gamepad_agent::diagnostics", phase = ?owner.phase, updates = owner.counters.updates,
                failures = owner.counters.failures, timeouts = owner.counters.timeouts, "Output diagnostics");
            }
            reported_updates = owner.counters.updates;
            report_at = Instant::now() + std::time::Duration::from_secs(1);
        }
        health.send_if_modified(|value| {
            if *value == owner.health {
                false
            } else {
                *value = owner.health;
                true
            }
        });
        tokio::select! {
            biased;
            changed = requests.changed() => if changed.is_err() { break; },
            _ = tick.tick() => {},
            changed = latest.changed() => if changed.is_err() { break; },
        }
    }
    let result = owner.shutdown();
    tracing::info!(
        updates = owner.counters.updates,
        failures = owner.counters.failures,
        timeouts = owner.counters.timeouts,
        "Output stopped"
    );
    health.send_replace(owner.health);
    result
}

#[cfg(test)]
mod safety_tests {
    use super::*;
    use crate::{
        protocol::SessionId,
        virtual_gamepad::{
            XboxReport,
            fake::{Call, RecordingBackend},
        },
    };
    use std::{
        num::NonZeroU64,
        sync::{Arc, Mutex},
        time::Duration,
    };

    fn id(value: u64) -> SessionId {
        SessionId(NonZeroU64::new(value).unwrap())
    }
    fn snapshot(session: u64, sequence: u32, now: Instant) -> AcceptedState {
        AcceptedState {
            state: ControllerState {
                session: id(session),
                sequence,
                client_ms: 0,
                buttons: 1,
                left_x: 0,
                left_y: 0,
                right_x: 0,
                right_y: 0,
                left_trigger: 0,
                right_trigger: 0,
            },
            received_at: now,
        }
    }

    #[test]
    fn clearing_latest_during_shutdown_is_not_an_input_timeout() {
        let record = Arc::new(Mutex::new(RecordingBackend::default()));
        let mut owner = OutputOwner::new(record.clone());
        let start = Instant::now();
        let request = Some(SessionRequest {
            session: id(1),
            expires_at: start + INPUT_TIMEOUT,
            armed: true,
        });
        owner.step(request, Some(snapshot(1, 1, start)), start);
        let now = start + Duration::from_secs(1);
        owner.step(request, Some(snapshot(1, 2, now)), now);
        // Receiver shutdown clears the slot before channel destruction. The
        // output worker may observe that clear while requests still holds the ID.
        owner.step(request, None, now);
        assert!(
            !owner.health.timed_out,
            "explicit clearing is not deadline expiry"
        );
        assert!(!owner.health.ready);
        assert!(!owner.health.attached);
        assert_eq!(
            &record.lock().unwrap().calls[4..],
            &[Call::Neutral, Call::Detach]
        );
        owner.shutdown().unwrap();
        assert_eq!(record.lock().unwrap().calls.len(), 6);
    }

    #[test]
    fn startup_neutral_failure_retains_diagnostic_and_refuses_readiness() {
        let record = Arc::new(Mutex::new(RecordingBackend::default()));
        record.lock().unwrap().failures.push(Call::Neutral);
        let mut owner = OutputOwner::new(record.clone());
        let now = Instant::now();
        owner.step(
            Some(SessionRequest {
                session: id(1),
                expires_at: now + INPUT_TIMEOUT,
                armed: true,
            }),
            Some(snapshot(1, 1, now)),
            now,
        );
        assert!(!owner.health.ready && !owner.health.attached && owner.health.fault);
        let diagnostic = owner.health.diagnostic.unwrap();
        assert_eq!(diagnostic.operation, "neutral");
        assert!(diagnostic.cleanup_failed);
        assert_eq!(
            record.lock().unwrap().calls,
            vec![Call::Attach, Call::Neutral, Call::Neutral, Call::Detach]
        );
        assert!(owner.shutdown().is_err());
    }

    #[test]
    fn startup_cancelled_after_initial_neutral_never_activates_or_submits() {
        let record = Arc::new(Mutex::new(RecordingBackend::default()));
        let mut owner = OutputOwner::new(record.clone());
        let now = Instant::now();
        owner.step_checked(
            Some(SessionRequest {
                session: id(1),
                expires_at: now + INPUT_TIMEOUT,
                armed: true,
            }),
            Some(snapshot(1, 1, now)),
            now,
            &|| record.lock().unwrap().calls.len() < 2,
        );
        assert!(!owner.health.ready && !owner.health.attached && !owner.health.fault);
        assert_eq!(
            record.lock().unwrap().calls,
            vec![Call::Attach, Call::Neutral, Call::Neutral, Call::Detach]
        );
        owner.shutdown().unwrap();
        let fresh = Some(SessionRequest {
            session: id(2),
            expires_at: now + INPUT_TIMEOUT,
            armed: true,
        });
        owner.step(fresh, None, now);
        assert!(owner.health.ready);
    }

    #[tokio::test(start_paused = true)]
    async fn startup_blocking_call_keeps_runtime_responsive_and_cannot_revive_cancelled_or_expired_grant()
     {
        use crate::virtual_gamepad::{BackendError, BackendHealth, VirtualGamepadBackend};
        struct Blocked {
            record: Arc<Mutex<RecordingBackend>>,
            entered: Option<tokio::sync::oneshot::Sender<()>>,
            release: std::sync::mpsc::Receiver<()>,
        }
        impl VirtualGamepadBackend for Blocked {
            fn attach(&mut self) -> Result<(), BackendError> {
                self.record.attach()?;
                self.entered.take().unwrap().send(()).unwrap();
                self.release.recv_timeout(Duration::from_secs(5)).unwrap();
                Ok(())
            }
            fn submit(&mut self, r: XboxReport) -> Result<(), BackendError> {
                self.record.submit(r)
            }
            fn neutral(&mut self) -> Result<(), BackendError> {
                self.record.neutral()
            }
            fn detach(&mut self) -> Result<(), BackendError> {
                self.record.detach()
            }
            fn health(&self) -> BackendHealth {
                self.record.health()
            }
        }
        for mode in ["revoke", "expire", "shutdown"] {
            let record = Arc::new(Mutex::new(RecordingBackend::default()));
            let (entered, waiting) = tokio::sync::oneshot::channel();
            let (release, blocked) = std::sync::mpsc::channel();
            let backend = Blocked {
                record: record.clone(),
                entered: Some(entered),
                release: blocked,
            };
            let (requests, rx) = watch::channel(Some(SessionRequest {
                session: id(1),
                expires_at: Instant::now() + INPUT_TIMEOUT,
                armed: true,
            }));
            let (latest, states) = LatestState::channel();
            latest.publish(snapshot(1, 1, Instant::now()).state);
            let (health, status) = watch::channel(OutputHealth::default());
            let runtime = tokio::runtime::Handle::current();
            let task = tokio::task::spawn_blocking(move || {
                runtime.block_on(run_output(backend, rx, states, health))
            });
            waiting.await.unwrap();
            assert!(!status.borrow().ready);
            match mode {
                "revoke" => {
                    requests.send_replace(None);
                }
                "expire" => tokio::time::advance(INPUT_TIMEOUT).await,
                _ => {}
            }
            // The runtime has processed revocation/time while attach is blocked.
            // Closing requests is the same output shutdown signal as service Exit.
            if mode == "shutdown" {
                drop(requests);
            } else {
                release.send(()).unwrap();
                // Wait for cleanup from the invalidated grant before closing.
                let cleanup_started = std::time::Instant::now();
                while record.lock().unwrap().calls.last() != Some(&Call::Detach) {
                    assert!(
                        cleanup_started.elapsed() < Duration::from_secs(5),
                        "fake backend did not clean up"
                    );
                    tokio::task::yield_now().await;
                }
                assert!(!status.borrow().ready);
                drop(requests);
            }
            if mode == "shutdown" {
                assert!(
                    !task.is_finished(),
                    "cleanup must join the uncancellable call"
                );
                release.send(()).unwrap();
            }
            task.await.unwrap().unwrap();
            assert!(!status.borrow().ready && !status.borrow().attached);
            assert_eq!(
                record.lock().unwrap().calls,
                vec![Call::Attach, Call::Neutral, Call::Detach]
            );
        }
    }

    #[test]
    fn hard_idle_expiry_detaches_once_and_requires_new_session() {
        let record = Arc::new(Mutex::new(RecordingBackend::default()));
        let mut owner = OutputOwner::new(record.clone());
        let now = Instant::now();
        let request = Some(SessionRequest {
            session: id(1),
            expires_at: now + SESSION_IDLE_TIMEOUT,
            armed: true,
        });
        let state = Some(snapshot(1, 1, now));
        owner.step(request, state, now);
        owner.step(
            request,
            state,
            now + SESSION_IDLE_TIMEOUT - Duration::from_nanos(1),
        );
        assert!(owner.health.ready);
        owner.step(request, state, now + SESSION_IDLE_TIMEOUT);
        assert!(owner.health.timed_out);
        assert!(!owner.health.attached);
        let count = record.lock().unwrap().calls.len();
        owner.step(
            request,
            Some(snapshot(1, 2, now + SESSION_IDLE_TIMEOUT)),
            now + SESSION_IDLE_TIMEOUT,
        );
        assert_eq!(record.lock().unwrap().calls.len(), count);
        let fresh = Some(SessionRequest {
            session: id(2),
            expires_at: now + SESSION_IDLE_TIMEOUT + INPUT_TIMEOUT,
            armed: true,
        });
        owner.step(fresh, state, now + SESSION_IDLE_TIMEOUT);
        assert!(owner.health.ready);
        assert_eq!(
            &record.lock().unwrap().calls[count..],
            &[Call::Attach, Call::Neutral]
        );
    }

    #[test]
    fn stale_snapshot_never_presses_and_disconnect_clears_before_replacement() {
        let record = Arc::new(Mutex::new(RecordingBackend::default()));
        let mut owner = OutputOwner::new(record.clone());
        let now = Instant::now();
        let request = Some(SessionRequest {
            session: id(1),
            expires_at: now + INPUT_TIMEOUT,
            armed: true,
        });
        owner.step(request, Some(snapshot(1, 1, now - INPUT_TIMEOUT)), now);
        assert!(owner.health.timed_out);
        assert!(
            !record
                .lock()
                .unwrap()
                .calls
                .iter()
                .any(|call| matches!(call, Call::Submit(value) if *value != XboxReport::NEUTRAL))
        );
        owner.step(None, None, now);
        let request = Some(SessionRequest {
            session: id(2),
            expires_at: now + INPUT_TIMEOUT,
            armed: true,
        });
        owner.step(request, Some(snapshot(2, 2, now)), now);
        owner.step(None, None, now);
        assert!(!owner.health.attached);
        let calls = &record.lock().unwrap().calls;
        assert_eq!(
            &calls[calls.len() - 3..],
            &[
                Call::Submit(XboxReport {
                    buttons: 0x1000,
                    ..XboxReport::NEUTRAL
                }),
                Call::Neutral,
                Call::Detach
            ]
        );
    }

    #[tokio::test(start_paused = true)]
    async fn latest_overwrite_timeout_and_shutdown_with_fake_time() {
        let record = Arc::new(Mutex::new(RecordingBackend::default()));
        let (requests, rx) = watch::channel(None);
        let (latest, states) = LatestState::channel();
        let (health, mut status) = watch::channel(OutputHealth::default());
        let task = tokio::spawn(run_output(record.clone(), rx, states, health));
        let now = Instant::now();
        requests.send_replace(Some(SessionRequest {
            session: id(1),
            expires_at: now + INPUT_TIMEOUT,
            armed: true,
        }));
        // No consumer runs between these updates. Only the final released state may apply.
        latest.publish(snapshot(1, 1, now).state);
        let mut released = snapshot(1, 2, now).state;
        released.buttons = 0;
        latest.publish(released);
        status.changed().await.unwrap();
        assert!(
            !record
                .lock()
                .unwrap()
                .calls
                .contains(&Call::Submit(XboxReport {
                    buttons: 0x1000,
                    ..XboxReport::NEUTRAL
                }))
        );
        tokio::time::advance(INPUT_TIMEOUT).await;
        tokio::task::yield_now().await;
        assert!(status.borrow().timed_out);
        drop(requests);
        task.await.unwrap().unwrap();
        assert_eq!(
            record
                .lock()
                .unwrap()
                .calls
                .iter()
                .filter(|call| **call == Call::Detach)
                .count(),
            1
        );
    }

    #[test]
    fn update_failure_is_faulted_and_cannot_be_hidden_by_new_session() {
        let record = Arc::new(Mutex::new(RecordingBackend::default()));
        record
            .lock()
            .unwrap()
            .failures
            .push(Call::Submit(XboxReport {
                buttons: 0x1000,
                ..XboxReport::NEUTRAL
            }));
        let mut owner = OutputOwner::new(record.clone());
        let now = Instant::now();
        for session in [1, 2] {
            owner.step(
                Some(SessionRequest {
                    session: id(session),
                    expires_at: now + INPUT_TIMEOUT,
                    armed: true,
                }),
                Some(snapshot(session, 1, now)),
                now,
            );
            assert!(owner.health.fault);
            assert!(!owner.health.ready);
        }
        assert_eq!(
            record
                .lock()
                .unwrap()
                .calls
                .iter()
                .filter(|call| **call == Call::Attach)
                .count(),
            1
        );
    }

    #[test]
    fn configured_timeouts_and_counters_obey_exact_boundaries() {
        for ms in [100, 250, 1000] {
            let record = Arc::new(Mutex::new(RecordingBackend::default()));
            let timeout = crate::config::InputTimeout::new(ms).unwrap();
            let mut owner = OutputOwner::with_timeout(record.clone(), timeout);
            let now = Instant::now();
            let request = Some(SessionRequest {
                session: id(1),
                armed: true,
                expires_at: now + timeout.duration(),
            });
            let state = Some(snapshot(1, 1, now));
            owner.counters.updates = u64::MAX;
            owner.counters.timeouts = u64::MAX;
            owner.step(request, state, now);
            owner.step(
                request,
                state,
                now + timeout.duration() - Duration::from_nanos(1),
            );
            assert_eq!(owner.phase, InputPhase::Active);
            owner.step(request, state, now + timeout.duration());
            assert_eq!(owner.phase, InputPhase::TimedOut);
            assert_eq!(owner.counters.updates, u64::MAX);
            assert_eq!(owner.counters.timeouts, u64::MAX);
            let count = record.lock().unwrap().calls.len();
            owner.step(request, state, now + Duration::from_secs(10));
            assert_eq!(record.lock().unwrap().calls.len(), count + 2);
            assert!(!owner.health.attached);
            owner.shutdown().unwrap();
            assert_eq!(owner.phase, InputPhase::ShuttingDown);
        }
    }

    #[tokio::test(start_paused = true)]
    async fn thirty_minutes_of_simulated_latest_state_has_no_replay_or_timeout() {
        let record = Arc::new(Mutex::new(RecordingBackend::default()));
        let mut owner = OutputOwner::new(record.clone());
        let (latest, states) = LatestState::channel();
        let start = Instant::now();
        let request = Some(SessionRequest {
            session: id(1),
            armed: true,
            expires_at: start + INPUT_TIMEOUT,
        });
        for tick in 0..108_000u32 {
            let now = Instant::now();
            for offset in 0..4 {
                let mut state = snapshot(1, tick * 4 + offset, now).state;
                state.buttons = u32::from(offset != 3);
                latest.publish(state);
            }
            owner.step(request, *states.borrow(), now);
            assert_eq!(
                record.lock().unwrap().calls.last(),
                Some(&Call::Submit(XboxReport::NEUTRAL))
            );
            // The recording fixture itself is not production storage. Keep it bounded too.
            record.lock().unwrap().calls.clear();
            tokio::time::advance(Duration::from_nanos(if tick % 3 == 0 {
                16_666_666
            } else {
                16_666_667
            }))
            .await;
        }
        assert_eq!(start.elapsed(), Duration::from_secs(1800));
        assert_eq!(owner.counters.updates, 108_000);
        assert_eq!(owner.counters.timeouts, 0);
        owner.shutdown().unwrap();
        assert_eq!(
            record.lock().unwrap().calls,
            vec![Call::Neutral, Call::Detach]
        );
    }
}
