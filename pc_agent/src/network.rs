//! UDP transport owns I/O only. A handler owns control replies and state admission.
use crate::{
    input::LatestState,
    protocol::{self, Message, PACKET_LEN},
    session::Handler,
    telemetry::Counters,
};
use std::{future::Future, io, net::SocketAddr, time::Duration};
use tokio::{
    net::UdpSocket,
    sync::watch,
    time::{MissedTickBehavior, interval},
};

pub struct Receiver<H> {
    socket: UdpSocket,
    handler: H,
    latest: LatestState,
    counters: Counters,
    metrics: watch::Sender<Counters>,
    last_accepted: Option<(protocol::SessionId, u32)>,
}
impl<H: Handler> Receiver<H> {
    pub fn new(
        socket: UdpSocket,
        handler: H,
        latest: LatestState,
    ) -> (Self, watch::Receiver<Counters>) {
        let (metrics, rx) = watch::channel(Counters::default());
        (
            Self {
                socket,
                handler,
                latest,
                counters: Counters::default(),
                metrics,
                last_accepted: None,
            },
            rx,
        )
    }
    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        self.socket.local_addr()
    }
    /// The shutdown future is pinned once, so unrelated receives cannot reset it.
    /// No detached tasks or queues are created; returning drops the socket.
    pub async fn run(mut self, shutdown: impl Future<Output = ()>) -> io::Result<Counters> {
        let result = self.receive(shutdown).await;
        self.latest.clear();
        self.metrics.send_replace(self.counters);
        self.counters.log("stopped");
        result.map(|()| self.counters)
    }
    async fn receive(&mut self, shutdown: impl Future<Output = ()>) -> io::Result<()> {
        // Covers even the maximum IPv4 UDP payload, avoiding WSAEMSGSIZE truncation.
        let mut buffer = vec![0u8; 65535];
        let mut report = interval(Duration::from_secs(1));
        report.set_missed_tick_behavior(MissedTickBehavior::Skip);
        let mut status = interval(Duration::from_millis(500));
        status.set_missed_tick_behavior(MissedTickBehavior::Skip);
        let mut transitions = interval(Duration::from_millis(5));
        transitions.set_missed_tick_behavior(MissedTickBehavior::Skip);
        let mut last_report = self.counters;
        let mut previous_poll = tokio::time::Instant::now();
        let mut max_poll_gap = Duration::ZERO;
        let mut previous_state: Option<(
            SocketAddr,
            protocol::SessionId,
            u32,
            u32,
            tokio::time::Instant,
        )> = None;
        let mut next_gap_report = previous_poll;
        tokio::pin!(shutdown);
        loop {
            let polled_at = tokio::time::Instant::now();
            max_poll_gap = max_poll_gap.max(polled_at.duration_since(previous_poll));
            previous_poll = polled_at;
            tokio::select! {
                biased;
                _ = &mut shutdown => return Ok(()),
                _ = transitions.tick() => {
                    if let Some((reply, destination)) = self.handler.transition_status() {
                        self.send_reply(reply, destination).await;
                    }
                }
                _ = status.tick() => {
                    if let Some((reply, destination)) = self.handler.status() {
                        self.send_reply(reply, destination).await;
                        self.metrics.send_replace(self.counters);
                    }
                }
                _ = report.tick() => {
                    if self.counters.accepted != last_report.accepted {
                        self.counters.log("traffic");
                    }
                    last_report = self.counters;
                }
                received = self.socket.recv_from(&mut buffer) => {
                    let (len, source) = match received {
                        Ok(result) => result,
                        Err(e) if e.kind() == io::ErrorKind::ConnectionReset || e.kind() == io::ErrorKind::ConnectionRefused => {
                            // Windows may report an ICMP error from a previous UDP reply.
                            self.counters.io_errors = self.counters.io_errors.saturating_add(1);
                            self.metrics.send_replace(self.counters);
                            continue;
                        }
                        Err(e) => return Err(e),
                    };
                    let message = match protocol::decode(&buffer[..len]) {
                        Ok(m) => m,
                        Err(e) => {
                            self.handler.invalid(source);
                            self.counters.reject(e);
                            self.counters.oversized = self.counters.oversized.saturating_add(u64::from(len > PACKET_LEN));
                            self.counters.last_rejection = Some(e);
                            self.metrics.send_replace(self.counters);
                            continue;
                        }
                    };
                    self.counters.decoded = self.counters.decoded.saturating_add(1);
                    if matches!(message, Message::HelloAck(_) | Message::PairAccept(_) | Message::PairReject(_) | Message::Status(_)) {
                        self.handler.invalid(source);
                        self.counters.refused = self.counters.refused.saturating_add(1);
                        self.metrics.send_replace(self.counters);
                        continue;
                    }
                    if let Message::ControllerState(state) = message {
                        let received_at = tokio::time::Instant::now();
                        max_poll_gap = max_poll_gap.max(received_at.duration_since(previous_poll));
                        if let Some((peer, session, sequence, client_ms, at)) = previous_state
                            && peer == source && session == state.session
                            && received_at.duration_since(at) >= Duration::from_millis(100)
                            && received_at >= next_gap_report {
                            tracing::warn!(receive_gap_ms = received_at.duration_since(at).as_millis(),
                                client_gap_ms = state.client_ms.wrapping_sub(client_ms),
                                sequence_gap = state.sequence.wrapping_sub(sequence),
                                receiver_poll_gap_ms = max_poll_gap.as_millis(),
                                "Controller arrival gap");
                            next_gap_report = received_at + Duration::from_secs(1);
                        }
                        previous_state = Some((source, state.session, state.sequence, state.client_ms, received_at));
                        max_poll_gap = Duration::ZERO;
                        match self.handler.admit(&state, source) {
                            Ok(()) => {
                                if let Some((session, sequence)) = self.last_accepted
                                    && session == state.session {
                                    self.counters.estimated_loss = self.counters.estimated_loss.saturating_add(u64::from(state.sequence.wrapping_sub(sequence).saturating_sub(1)));
                                }
                                self.last_accepted = Some((state.session, state.sequence));
                                self.latest.publish(state);
                                self.counters.accepted = self.counters.accepted.saturating_add(1);
                            }
                            Err(error) => self.counters.refuse(error),
                        }
                    } else {
                        let reply = self.handler.control(message, source);
                        if self.handler.take_reset() { self.latest.clear(); self.last_accepted = None; }
                        if let Some(reply) = reply {
                            self.send_reply(reply, source).await;
                        }
                    }
                    self.metrics.send_replace(self.counters);
                }
            }
            if self.handler.take_reset() {
                self.latest.clear();
                self.last_accepted = None;
            }
        }
    }

    async fn send_reply(&mut self, reply: Message, destination: SocketAddr) {
        let Ok(bytes) = reply.encode() else {
            self.counters.reply_errors = self.counters.reply_errors.saturating_add(1);
            return;
        };
        // No reply can hold up admission/cancellation indefinitely under socket backpressure.
        match tokio::time::timeout(
            Duration::from_millis(2),
            self.socket.send_to(&bytes, destination),
        )
        .await
        {
            Ok(Ok(PACKET_LEN)) => self.counters.replies = self.counters.replies.saturating_add(1),
            _ => self.counters.io_errors = self.counters.io_errors.saturating_add(1),
        }
    }
}
