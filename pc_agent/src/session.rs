//! Shared source/session/sequence safety with an explicit development wrapper.
use crate::protocol::{ControllerState, Message};
use std::net::SocketAddr;

pub fn is_newer(candidate: u32, accepted: u32) -> bool {
    candidate != accepted && candidate.wrapping_sub(accepted) < (1 << 31)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdmissionError {
    Refused,
    WrongSession,
    WrongSource,
    Stale,
    UnsupportedState,
}

/// Hooks must not block. Validate session, source and sequence before accepting.
pub trait Handler {
    fn transition_status(&mut self) -> Option<(Message, SocketAddr)> {
        None
    }
    fn status(&mut self) -> Option<(Message, SocketAddr)> {
        None
    }
    fn control(&mut self, _message: Message, _source: SocketAddr) -> Option<Message> {
        None
    }
    fn admit(&mut self, state: &ControllerState, source: SocketAddr) -> Result<(), AdmissionError>;
    fn take_reset(&mut self) -> bool {
        false
    }
    fn invalid(&mut self, _source: SocketAddr) {}
}

/// Diagnostic constructor never owns output. Output trust remains debug only.
pub struct DevelopmentTrust(SessionAdmission);
impl DevelopmentTrust {
    pub fn new(enabled: bool) -> Self {
        Self(SessionAdmission::new(enabled))
    }
    pub fn with_output(
        enabled: bool,
        requests: tokio::sync::watch::Sender<Option<crate::input::SessionRequest>>,
        health: tokio::sync::watch::Receiver<crate::input::OutputHealth>,
    ) -> Self {
        Self(SessionAdmission::with_output(
            enabled && cfg!(debug_assertions),
            requests,
            health,
        ))
    }
    pub fn with_timeout(self, timeout: crate::config::InputTimeout) -> Self {
        Self(self.0.with_timeout(timeout))
    }
}
impl Handler for DevelopmentTrust {
    fn transition_status(&mut self) -> Option<(Message, SocketAddr)> {
        self.0.transition_status()
    }
    fn status(&mut self) -> Option<(Message, SocketAddr)> {
        self.0.status()
    }
    fn control(&mut self, message: Message, source: SocketAddr) -> Option<Message> {
        self.0.control(message, source)
    }
    fn admit(&mut self, state: &ControllerState, source: SocketAddr) -> Result<(), AdmissionError> {
        self.0.admit(state, source)
    }
    fn take_reset(&mut self) -> bool {
        self.0.take_reset()
    }
    fn invalid(&mut self, source: SocketAddr) {
        self.0.invalid(source);
    }
}

/// The first instance owns admission until process exit. No paired identity is stored.
pub(crate) struct SessionAdmission {
    enabled: bool,
    client: Option<(crate::protocol::InstanceId, SocketAddr)>,
    pending_status: Option<SocketAddr>,
    output: Option<(
        tokio::sync::watch::Sender<Option<crate::input::SessionRequest>>,
        tokio::sync::watch::Receiver<crate::input::OutputHealth>,
    )>,
    provisional: Option<Provisional>,
    reset: bool,
    timeout: crate::config::InputTimeout,
    reported_health: crate::input::OutputHealth,
}

struct Provisional {
    session: crate::protocol::SessionId,
    nonce: crate::protocol::Nonce,
    granted: bool,
    last: Option<ControllerState>,
    accepted: u32,
    rejected: u32,
    lost: u16,
    rate_count: u32,
    rate_since: tokio::time::Instant,
    observed_rate: u16,
    deadline: tokio::time::Instant,
}

pub(crate) fn random_id() -> Option<std::num::NonZeroU64> {
    for _ in 0..8 {
        let mut bytes = [0; 8];
        getrandom::fill(&mut bytes).ok()?;
        if let Some(id) = std::num::NonZeroU64::new(u64::from_ne_bytes(bytes)) {
            return Some(id);
        }
    }
    None
}
impl SessionAdmission {
    pub fn new(enabled: bool) -> Self {
        Self {
            enabled,
            client: None,
            pending_status: None,
            output: None,
            provisional: None,
            reset: false,
            timeout: crate::config::InputTimeout::default(),
            reported_health: Default::default(),
        }
    }

    pub fn with_output(
        enabled: bool,
        requests: tokio::sync::watch::Sender<Option<crate::input::SessionRequest>>,
        health: tokio::sync::watch::Receiver<crate::input::OutputHealth>,
    ) -> Self {
        let mut handler = Self::new(enabled);
        handler.output = Some((requests, health));
        handler
    }

    pub fn with_timeout(mut self, timeout: crate::config::InputTimeout) -> Self {
        self.timeout = timeout;
        self
    }

    pub(crate) fn revoke(&mut self) {
        if let Some((requests, _)) = &self.output {
            requests.send_replace(None);
        }
        self.client = None;
        self.provisional = None;
        self.pending_status = None;
        self.reset = true;
    }

    pub(crate) fn output_health(&self) -> crate::input::OutputHealth {
        self.output
            .as_ref()
            .map_or_else(Default::default, |(_, health)| {
                let mut value = *health.borrow();
                if health.has_changed().is_err() {
                    value.ready = false;
                    value.fault = true;
                }
                value
            })
    }
}
impl Handler for SessionAdmission {
    fn transition_status(&mut self) -> Option<(Message, SocketAddr)> {
        let health = self.output_health();
        if health == self.reported_health {
            return None;
        }
        self.reported_health = health;
        self.status()
    }
    fn control(&mut self, message: Message, source: SocketAddr) -> Option<Message> {
        use crate::protocol::{HelloAck, Nonce};
        if !self.enabled {
            return None;
        }
        if let Message::Disconnect(disconnect) = message {
            if self.client.is_some_and(|(_, bound)| bound == source)
                && self
                    .provisional
                    .as_ref()
                    .is_some_and(|p| p.granted && p.session == disconnect.session)
            {
                if let Some((requests, _)) = &self.output {
                    requests.send_replace(None);
                }
                self.reset = true;
                self.provisional = None;
                self.pending_status = None;
            }
            return None;
        }
        let Message::Hello(hello) = message else {
            return None;
        };
        let server_nonce = Nonce(random_id()?);
        let source_changed = self
            .client
            .is_some_and(|(instance, bound)| instance == hello.client_instance && bound != source);
        match self.client {
            None => {
                self.client = Some((hello.client_instance, source));
                // The outer policy owns identity admission and user-facing trust labels.
            }
            Some((instance, _)) if instance == hello.client_instance => {
                self.client = Some((instance, source));
            }
            _ => {} // Every hello gets an honest ack, but cannot replace the first instance.
        }
        if self.client == Some((hello.client_instance, source)) {
            self.pending_status = Some(source);
            let health = self.output_health();
            if self.output.is_some() && !health.fault {
                let rotate = source_changed
                    || self.provisional.as_ref().is_none_or(|p| {
                        (p.granted && p.nonce != hello.client_nonce)
                            || (health.session == Some(p.session)
                                && health.timed_out
                                && !health.ready)
                            || tokio::time::Instant::now() >= p.deadline
                    });
                if rotate {
                    // Invalidate first, even when random generation subsequently fails.
                    self.reset = true;
                    self.provisional = None;
                    let (requests, _) = self.output.as_ref()?;
                    requests.send_replace(None);
                    let session = crate::protocol::SessionId(random_id()?);
                    let now = tokio::time::Instant::now();
                    self.provisional = Some(Provisional {
                        session,
                        nonce: hello.client_nonce,
                        granted: false,
                        last: None,
                        accepted: 0,
                        rejected: 0,
                        lost: 0,
                        rate_count: 0,
                        rate_since: now,
                        observed_rate: 0,
                        deadline: now + std::time::Duration::from_secs(5),
                    });
                    requests.send_replace(Some(crate::input::SessionRequest {
                        session,
                        expires_at: now + std::time::Duration::from_secs(5),
                        armed: false,
                    }));
                }
                if let Some(p) = &mut self.provisional
                    && health.session == Some(p.session)
                    && health.ready
                    && !p.granted
                {
                    p.granted = true;
                    p.nonce = hello.client_nonce;
                    let now = tokio::time::Instant::now();
                    p.deadline = now + crate::input::SESSION_IDLE_TIMEOUT;
                    p.rate_since = tokio::time::Instant::now();
                    let (requests, _) = self.output.as_ref()?;
                    requests.send_replace(Some(crate::input::SessionRequest {
                        session: p.session,
                        expires_at: now + self.timeout.duration(),
                        armed: true,
                    }));
                }
            }
        }
        let health = self.output_health();
        let session = self
            .provisional
            .as_ref()
            .filter(|p| {
                p.granted
                    && tokio::time::Instant::now() < p.deadline
                    && health.session == Some(p.session)
                    && health.ready
                    && self.client == Some((hello.client_instance, source))
            })
            .map(|p| p.session);
        Some(Message::HelloAck(HelloAck {
            client_nonce: hello.client_nonce,
            server_nonce,
            session,
            state_rate: 60,
            timeout_ms: self.timeout.millis(),
            flags: 4 | if session.is_some() { 2 } else { 0 },
            capacity: 1,
            slot: if session.is_some() { 0 } else { 255 },
        }))
    }
    fn status(&mut self) -> Option<(Message, SocketAddr)> {
        if self.provisional.as_ref().is_some_and(|p| p.granted) {
            let health = self.output_health();
            let p = self.provisional.as_mut()?;
            let (_, source) = self.client?;
            let elapsed = p.rate_since.elapsed().as_secs_f64();
            // Transition reports must not reset the periodic rate window.
            if elapsed >= 0.5 {
                p.observed_rate =
                    (f64::from(p.rate_count) / elapsed).min(f64::from(u16::MAX)) as u16;
                p.rate_count = 0;
                p.rate_since = tokio::time::Instant::now();
            }
            let expired = tokio::time::Instant::now() >= p.deadline;
            let ready = health.session == Some(p.session) && health.ready && !expired;
            return Some((
                Message::Status(crate::protocol::Status {
                    session: Some(p.session),
                    last_sequence: p.last.map_or(0, |s| s.sequence),
                    client_ms: p.last.map_or(0, |s| s.client_ms),
                    accepted: p.accepted,
                    rejected: p.rejected,
                    lost: p.lost,
                    observed_rate: p.observed_rate,
                    timeout_ms: self.timeout.millis(),
                    flags: u16::from(p.last.is_some())
                        | if ready { 2 } else { 0 }
                        | if ready && health.attached { 4 } else { 0 }
                        | if health.timed_out || expired { 8 } else { 0 }
                        | if health.fault { 32 } else { 0 },
                }),
                source,
            ));
        }
        let source = self.pending_status.take()?;
        Some((
            Message::Status(crate::protocol::Status {
                session: None,
                last_sequence: 0,
                client_ms: 0,
                accepted: 0,
                rejected: 0,
                lost: 0,
                observed_rate: 0,
                flags: 0,
                timeout_ms: self.timeout.millis(),
            }),
            source,
        ))
    }
    fn admit(&mut self, state: &ControllerState, source: SocketAddr) -> Result<(), AdmissionError> {
        let health = self.output_health();
        let result = (|| {
            if !self.enabled {
                return Err(AdmissionError::Refused);
            }
            let p = self.provisional.as_mut().ok_or(AdmissionError::Refused)?;
            if !p.granted || !health.ready || health.session != Some(p.session) {
                return Err(AdmissionError::Refused);
            }
            if tokio::time::Instant::now() >= p.deadline {
                return Err(AdmissionError::Refused);
            }
            if state.session != p.session {
                return Err(AdmissionError::WrongSession);
            }
            if !self.client.is_some_and(|(_, bound)| bound == source) {
                return Err(AdmissionError::WrongSource);
            }
            if !crate::input::supported_controls(state) {
                return Err(AdmissionError::UnsupportedState);
            }
            if let Some(last) = p.last {
                if !is_newer(state.sequence, last.sequence) {
                    return Err(AdmissionError::Stale);
                }
                p.lost = p.lost.saturating_add(
                    state
                        .sequence
                        .wrapping_sub(last.sequence)
                        .saturating_sub(1)
                        .min(u32::from(u16::MAX)) as u16,
                );
            }
            p.last = Some(*state);
            p.deadline = tokio::time::Instant::now() + crate::input::SESSION_IDLE_TIMEOUT;
            p.accepted = p.accepted.wrapping_add(1);
            p.rate_count = p.rate_count.saturating_add(1);
            Ok(())
        })();
        if result.is_err() {
            self.invalid(source);
        }
        result
    }
    fn take_reset(&mut self) -> bool {
        std::mem::take(&mut self.reset)
    }
    fn invalid(&mut self, source: SocketAddr) {
        if self.client.is_some_and(|(_, bound)| bound == source)
            && let Some(p) = &mut self.provisional
        {
            p.rejected = p.rejected.wrapping_add(1);
        }
    }
}

pub struct DenyAll;
impl Handler for DenyAll {
    fn admit(&mut self, _: &ControllerState, _: SocketAddr) -> Result<(), AdmissionError> {
        Err(AdmissionError::Refused)
    }
}

#[cfg(all(test, debug_assertions))]
mod diagnostic_tests {
    use super::*;
    use crate::protocol::*;
    use std::num::NonZeroU64;
    use tokio::sync::watch;

    #[tokio::test(start_paused = true)]
    async fn wire_counters_wrap_loss_saturates_and_transition_does_not_reset_rate() {
        let source = "127.0.0.1:1234".parse().unwrap();
        let (requests, rx) = watch::channel(None);
        let (health, health_rx) = watch::channel(crate::input::OutputHealth::default());
        let mut handler = DevelopmentTrust::with_output(true, requests, health_rx);
        let hello = Message::Hello(Hello {
            client_nonce: Nonce(NonZeroU64::new(1).unwrap()),
            client_instance: InstanceId(NonZeroU64::new(1).unwrap()),
            state_rate: 60,
            capabilities: 1,
            app_build: 1,
            api_level: 34,
        });
        handler.control(hello, source);
        let session = rx.borrow().unwrap().session;
        health.send_replace(crate::input::OutputHealth {
            session: Some(session),
            ready: true,
            attached: true,
            ..Default::default()
        });
        handler.control(hello, source);
        let state = ControllerState {
            session,
            sequence: 1,
            client_ms: 0,
            buttons: 0,
            left_x: 0,
            left_y: 0,
            right_x: 0,
            right_y: 0,
            left_trigger: 0,
            right_trigger: 0,
        };
        let p = handler.0.provisional.as_mut().unwrap();
        p.accepted = u32::MAX;
        p.rejected = u32::MAX;
        p.lost = u16::MAX - 1;
        handler.admit(&state, source).unwrap();
        handler.invalid(source);
        assert_eq!(handler.0.provisional.as_ref().unwrap().accepted, 0);
        assert_eq!(handler.0.provisional.as_ref().unwrap().rejected, 0);
        handler
            .admit(
                &ControllerState {
                    sequence: 10,
                    ..state
                },
                source,
            )
            .unwrap();
        assert_eq!(handler.0.provisional.as_ref().unwrap().lost, u16::MAX);
        for sequence in 11..41 {
            tokio::time::advance(std::time::Duration::from_millis(16)).await;
            handler
                .admit(&ControllerState { sequence, ..state }, source)
                .unwrap();
            handler.transition_status();
        }
        tokio::time::advance(std::time::Duration::from_millis(20)).await;
        let Message::Status(status) = handler.status().unwrap().0 else {
            panic!()
        };
        assert_eq!(status.observed_rate, 64);
        assert_eq!(status.lost, u16::MAX);
        assert_eq!(status.flags & 16, 0); // Development trust never claims pairing.
    }
}
