//! Production admission policy, sharing the existing output/freshness engine.
use crate::{
    config::InputTimeout,
    input::{OutputHealth, SessionRequest},
    pairing::{PairingPolicy, random_code},
    pairing_store::{PairingRecord, StoreRequest},
    protocol::*,
    session::{AdmissionError, Handler, SessionAdmission},
};
use std::{
    net::SocketAddr,
    sync::mpsc::{Receiver, SyncSender},
    time::Duration,
};
use tokio::{
    sync::{oneshot, watch},
    time::Instant,
};

pub enum PairingCommand {
    Open,
    Close,
    /// UI dismissal is scoped to its window, including when commands queue up.
    CloseIf(Instant),
    Forget,
    Replace,
}
/// Local presentation only. Never format with Debug, trace or persist the code.
#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub struct PairingView {
    pub window: Option<(u32, Instant)>,
    pub last_phone: Option<u32>,
    pub pending: bool,
    pub storage_failed: bool,
}
struct Pending {
    hello: Hello,
    source: SocketAddr,
    request: PairRequest,
    started: Instant,
    saved: Option<oneshot::Receiver<std::io::Result<()>>>,
    session: Option<SessionId>,
}
struct Cached {
    request: PairRequest,
    source: SocketAddr,
    reply: PairAccept,
    expires: Instant,
}
pub struct PairedSession {
    engine: SessionAdmission,
    policy: PairingPolicy,
    record: PairingRecord,
    store: SyncSender<StoreRequest>,
    commands: Receiver<PairingCommand>,
    timeout: InputTimeout,
    pending: Option<Pending>,
    cache: Option<Cached>,
    revoking: Option<(oneshot::Receiver<std::io::Result<()>>, bool)>,
    storage_failed: bool,
    codes: SyncSender<(u32, Instant)>,
    view: Option<watch::Sender<PairingView>>,
}
impl PairedSession {
    pub fn new(
        record: PairingRecord,
        store: SyncSender<StoreRequest>,
        commands: Receiver<PairingCommand>,
        requests: watch::Sender<Option<SessionRequest>>,
        health: watch::Receiver<OutputHealth>,
        timeout: InputTimeout,
        codes: SyncSender<(u32, Instant)>,
    ) -> Self {
        Self {
            engine: SessionAdmission::with_output(true, requests, health).with_timeout(timeout),
            // Stored metadata never authorizes a new connection.
            policy: PairingPolicy::new(None),
            record,
            store,
            commands,
            timeout,
            pending: None,
            cache: None,
            revoking: None,
            storage_failed: false,
            codes,
            view: None,
        }
    }
    pub fn with_view(mut self, view: watch::Sender<PairingView>) -> Self {
        self.view = Some(view);
        self.publish_view();
        self
    }
    fn publish_view(&self) {
        if let Some(view) = &self.view {
            let next = PairingView {
                window: self.policy.window(Instant::now()),
                last_phone: self.record.client.map(|id| (id.0.get() & 0xffffff) as u32),
                pending: self.pending.is_some() || self.revoking.is_some(),
                storage_failed: self.storage_failed,
            };
            view.send_if_modified(|old| {
                if *old == next {
                    false
                } else {
                    *old = next;
                    true
                }
            });
        }
    }
    fn save(&self, client: Option<InstanceId>) -> Option<oneshot::Receiver<std::io::Result<()>>> {
        let (tx, rx) = oneshot::channel();
        self.store
            .try_send(StoreRequest {
                record: PairingRecord {
                    client,
                    ..self.record
                },
                result: tx,
            })
            .ok()?;
        Some(rx)
    }
    fn open(&mut self) {
        if self.pending.is_some() || self.revoking.is_some() || self.storage_failed {
            tracing::warn!(
                "Pairing unavailable while storage is pending or failed; use forget to retry"
            );
            return;
        }
        self.engine.revoke();
        self.policy.revoke();
        self.cache = None;
        match random_code() {
            Ok(code) if self.policy.open(code, Instant::now()) => {
                // A bounded terminal worker prints the code; never block the UDP owner.
                if self
                    .codes
                    .try_send((code, Instant::now() + crate::pairing::WINDOW))
                    .is_err()
                {
                    self.policy.revoke();
                    tracing::warn!("Terminal unavailable; pairing window closed");
                }
            }
            Ok(_) => tracing::warn!("One phone is paired; use replace to revoke it explicitly"),
            Err(_) => tracing::error!("OS randomness unavailable; pairing stays closed"),
        }
    }
    fn commands(&mut self) {
        // Bounded work per tick, even if terminal input is flooded.
        for _ in 0..4 {
            let Ok(command) = self.commands.try_recv() else {
                break;
            };
            match command {
                PairingCommand::Open => self.open(),
                PairingCommand::Close => self.policy.close_window(),
                PairingCommand::CloseIf(deadline) => {
                    if self
                        .policy
                        .window(Instant::now())
                        .is_some_and(|(_, current)| current == deadline)
                    {
                        self.policy.close_window();
                    }
                }
                PairingCommand::Forget | PairingCommand::Replace => {
                    self.engine.revoke();
                    self.policy.revoke();
                    self.pending = None;
                    self.cache = None;
                    if self.revoking.is_some() {
                        continue;
                    }
                    let replace = matches!(command, PairingCommand::Replace);
                    if let Some(rx) = self.save(None) {
                        self.revoking = Some((rx, replace));
                    } else {
                        self.storage_failed = true;
                        tracing::error!(
                            "Could not queue pairing removal; input remains disabled, retry forget"
                        );
                    }
                }
            }
        }
        if let Some((rx, replace)) = &mut self.revoking {
            match rx.try_recv() {
                Ok(Ok(())) => {
                    let replace = *replace;
                    self.revoking = None;
                    self.record.client = None;
                    self.storage_failed = false;
                    tracing::info!("Pairing forgotten; controller session revoked");
                    if replace {
                        self.open();
                    }
                }
                Err(oneshot::error::TryRecvError::Empty) => {}
                _ => {
                    self.revoking = None;
                    self.storage_failed = true;
                    tracing::error!(
                        "Pairing removal failed; input disabled, retry forget before restart"
                    );
                }
            }
        }
    }
    fn tick(&mut self) -> Option<(Message, SocketAddr)> {
        self.commands();
        let mut p = self.pending.take()?;
        let reject = |reason| {
            (
                Message::PairReject(PairReject {
                    client_nonce: p.hello.client_nonce.0.get(),
                    reason,
                    retry_after: 0,
                }),
                p.source,
            )
        };
        if p.started.elapsed() >= Duration::from_secs(5) && p.saved.is_none() {
            self.engine.revoke();
            return Some(reject(RejectReason::BackendNotReady));
        }
        if p.saved.is_none() {
            if self.engine.output_health().fault {
                self.engine.revoke();
                return Some(reject(RejectReason::BackendNotReady));
            }
            if let Some(Message::HelloAck(ack)) =
                self.engine.control(Message::Hello(p.hello), p.source)
                && let Some(session) = ack.session
            {
                p.session = Some(session);
                p.saved = self.save(Some(p.hello.client_instance));
                if p.saved.is_none() {
                    self.engine.revoke();
                    return Some(reject(RejectReason::Unspecified));
                }
            }
        }
        if let Some(rx) = &mut p.saved {
            match rx.try_recv() {
                Ok(Ok(())) => {
                    self.record.client = Some(p.hello.client_instance);
                    self.policy.commit(p.hello.client_instance);
                    let session = p.session?;
                    // A late disk result retains durable pairing, but never revives expired input.
                    if p.started.elapsed() >= Duration::from_secs(3)
                        || !self.engine.output_health().ready
                    {
                        self.engine.revoke();
                        return Some(reject(RejectReason::BackendNotReady));
                    }
                    let reply = PairAccept {
                        session,
                        client_nonce: p.hello.client_nonce,
                        server_instance: self.record.server,
                        state_rate: 60,
                        timeout_ms: self.timeout.millis(),
                        slot: 0,
                        capabilities: (p.hello.capabilities as u8) & 3,
                    };
                    self.cache = Some(Cached {
                        request: p.request,
                        source: p.source,
                        reply,
                        expires: p.started + Duration::from_secs(3),
                    });
                    return Some((Message::PairAccept(reply), p.source));
                }
                Err(oneshot::error::TryRecvError::Empty) => {}
                _ => {
                    self.engine.revoke();
                    self.storage_failed = true;
                    tracing::error!(
                        "Pairing could not be saved; input remains disabled, use forget to retry"
                    );
                    return Some(reject(RejectReason::Unspecified));
                }
            }
        }
        self.pending = Some(p);
        None
    }
    fn paired_status(
        &self,
        status: Option<(Message, SocketAddr)>,
    ) -> Option<(Message, SocketAddr)> {
        status.map(|(message, source)| {
            (
                match message {
                    Message::Status(mut status) => {
                        if self.policy.paired().is_some() {
                            status.flags |= 16;
                        }
                        Message::Status(status)
                    }
                    other => other,
                },
                source,
            )
        })
    }
}
impl Handler for PairedSession {
    fn transition_status(&mut self) -> Option<(Message, SocketAddr)> {
        let reply = self.tick();
        self.publish_view();
        if let Some(reply) = reply {
            return Some(reply);
        }
        if self.pending.is_some() {
            return None;
        }
        let status = self.engine.transition_status();
        self.paired_status(status)
    }
    fn status(&mut self) -> Option<(Message, SocketAddr)> {
        if self.pending.is_some() {
            return None;
        }
        let status = self.engine.status();
        self.paired_status(status)
    }
    fn control(&mut self, message: Message, source: SocketAddr) -> Option<Message> {
        if !source.is_ipv4() {
            return None;
        }
        match message {
            Message::Hello(hello) => {
                let nonce = Nonce(crate::session::random_id()?);
                let nonce = self
                    .policy
                    .challenge(hello, source, nonce, Instant::now())
                    .unwrap_or(nonce);
                Some(Message::HelloAck(HelloAck {
                    client_nonce: hello.client_nonce,
                    server_nonce: nonce,
                    session: None,
                    state_rate: 60,
                    timeout_ms: self.timeout.millis(),
                    flags: 1,
                    capacity: 1,
                    slot: 255,
                }))
            }
            Message::PairRequest(request) => {
                if let Some(cache) = &self.cache
                    && cache.request == request
                    && cache.source == source
                    && Instant::now() < cache.expires
                    && self.engine.output_health().ready
                    && !self.engine.output_health().fault
                    && self.engine.output_health().session == Some(cache.reply.session)
                {
                    return Some(Message::PairAccept(cache.reply));
                }
                if let Some(p) = &self.pending
                    && p.request == request
                    && p.source == source
                {
                    return None;
                }
                let error = if self.storage_failed || self.revoking.is_some() {
                    Some((RejectReason::Unspecified, 0))
                } else if self.pending.is_some() {
                    Some((RejectReason::NoSlot, 0))
                } else if self.engine.output_health().fault {
                    Some((RejectReason::BackendNotReady, 0))
                } else {
                    match self.policy.reserve(request, source, Instant::now()) {
                        Ok(hello) => {
                            self.pending = Some(Pending {
                                hello,
                                source,
                                request,
                                started: Instant::now(),
                                saved: None,
                                session: None,
                            });
                            None
                        }
                        Err(error) => Some(error),
                    }
                };
                error.map(|(reason, retry_after)| {
                    Message::PairReject(PairReject {
                        client_nonce: request.client_nonce.0.get(),
                        reason,
                        retry_after,
                    })
                })
            }
            Message::Disconnect(_) => {
                self.cache = None;
                self.engine.control(message, source)
            }
            _ => None,
        }
    }
    fn admit(&mut self, state: &ControllerState, source: SocketAddr) -> Result<(), AdmissionError> {
        if self.policy.paired().is_none()
            || self.pending.is_some()
            || self.revoking.is_some()
            || self.storage_failed
        {
            return Err(AdmissionError::Refused);
        }
        self.engine.admit(state, source)
    }
    fn take_reset(&mut self) -> bool {
        self.engine.take_reset()
    }
    fn invalid(&mut self, source: SocketAddr) {
        self.engine.invalid(source);
    }
}

impl Drop for PairedSession {
    fn drop(&mut self) {
        if let Some(view) = &self.view {
            view.send_replace(PairingView::default());
        }
    }
}
