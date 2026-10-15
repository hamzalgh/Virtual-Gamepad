//! Development only fault link. All sockets are loopback, queues/windows are bounded,
//! and output is a fake. No helper is reachable from the production executable.
#![cfg(debug_assertions)]
use std::{
    net::SocketAddr,
    num::NonZeroU64,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::{
    net::UdpSocket,
    sync::{oneshot, watch},
    time::{Instant, timeout},
};
use virtual_gamepad_agent::{
    config::InputTimeout,
    input::{LatestReceiver, LatestState, OutputHealth, OutputOwner, SessionRequest},
    network::Receiver,
    protocol::*,
    session::DevelopmentTrust,
    telemetry::Counters,
    virtual_gamepad::{
        XboxReport,
        fake::{Call, RecordingBackend},
    },
};

struct FaultLink {
    address: SocketAddr,
    phone: UdpSocket,
    other: UdpSocket,
    metrics: watch::Receiver<Counters>,
    states: LatestReceiver,
    requests: watch::Receiver<Option<SessionRequest>>,
    health: watch::Sender<OutputHealth>,
    owner: OutputOwner<Arc<Mutex<RecordingBackend>>>,
    record: Arc<Mutex<RecordingBackend>>,
    stop: oneshot::Sender<()>,
    task: tokio::task::JoinHandle<std::io::Result<Counters>>,
    session: SessionId,
    // Fixed capacity reorder/delay window. No heap queue can grow with traffic.
    delayed: [Option<[u8; PACKET_LEN]>; 16],
}
impl FaultLink {
    async fn new() -> Self {
        Self::with_timeout(InputTimeout::new(1000).unwrap()).await
    }
    async fn with_timeout(duration: InputTimeout) -> Self {
        let address: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let server = UdpSocket::bind(address).await.unwrap();
        let address = server.local_addr().unwrap();
        let phone = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let other = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let (request_tx, requests) = watch::channel(None);
        let (health, health_rx) = watch::channel(OutputHealth::default());
        let (latest, states) = LatestState::channel();
        let handler =
            DevelopmentTrust::with_output(true, request_tx, health_rx).with_timeout(duration);
        let (receiver, metrics) = Receiver::new(server, handler, latest);
        let (stop, stopped) = oneshot::channel();
        let task = tokio::spawn(receiver.run(async {
            let _ = stopped.await;
        }));
        let record = Arc::new(Mutex::new(RecordingBackend::default()));
        let mut link = Self {
            address,
            phone,
            other,
            metrics,
            states,
            requests,
            health,
            owner: OutputOwner::with_timeout(record.clone(), duration),
            record,
            stop,
            task,
            session: SessionId(NonZeroU64::new(1).unwrap()),
            delayed: [None; 16],
        };
        link.deliver(link.hello(1), false).await;
        link.pump(Instant::now());
        link.deliver(link.hello(2), false).await;
        let request = link.requests.borrow().unwrap();
        assert!(request.armed);
        link.session = request.session;
        link
    }
    fn hello(&self, nonce: u64) -> [u8; PACKET_LEN] {
        Message::Hello(Hello {
            client_nonce: Nonce(NonZeroU64::new(nonce).unwrap()),
            client_instance: InstanceId(NonZeroU64::new(1).unwrap()),
            state_rate: 60,
            capabilities: 1,
            app_build: 1,
            api_level: 34,
        })
        .encode()
        .unwrap()
    }
    fn state(&self, sequence: u32, pressed: bool) -> [u8; PACKET_LEN] {
        Message::ControllerState(ControllerState {
            session: self.session,
            sequence,
            client_ms: sequence,
            buttons: u32::from(pressed),
            left_x: 0,
            left_y: 0,
            right_x: 0,
            right_y: 0,
            left_trigger: 0,
            right_trigger: 0,
        })
        .encode()
        .unwrap()
    }
    async fn deliver(&mut self, bytes: [u8; PACKET_LEN], other: bool) {
        let before = *self.metrics.borrow_and_update();
        let socket = if other { &self.other } else { &self.phone };
        socket.send_to(&bytes, self.address).await.unwrap();
        timeout(Duration::from_secs(2), async {
            loop {
                let value = *self.metrics.borrow_and_update();
                if value.decoded + value.invalid > before.decoded + before.invalid {
                    break;
                }
                self.metrics.changed().await.unwrap();
            }
        })
        .await
        .unwrap();
    }
    fn pump(&mut self, now: Instant) {
        let request = *self.requests.borrow();
        let latest = *self.states.borrow();
        self.owner.step(request, latest, now);
        self.health.send_replace(self.owner.health);
    }
    async fn finish(mut self) {
        self.stop.send(()).unwrap();
        timeout(Duration::from_secs(2), self.task)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        self.owner.shutdown().unwrap();
        let calls = self.record.lock().unwrap().calls.len();
        self.owner.shutdown().unwrap();
        assert_eq!(self.record.lock().unwrap().calls.len(), calls);
        assert!(UdpSocket::bind(self.address).await.is_ok());
    }
}

#[tokio::test]
async fn repeated_short_udp_gaps_recover_same_session_without_replaying_or_device_churn() {
    let mut link = FaultLink::with_timeout(InputTimeout::default()).await;
    for sequence in 1..=3 {
        link.deliver(link.state(sequence, true), false).await;
        link.pump(Instant::now());
        let last = *link.states.borrow();
        tokio::time::sleep(Duration::from_millis(270)).await;
        link.pump(Instant::now());
        assert!(
            link.owner.health.ready && link.owner.health.attached && link.owner.health.timed_out
        );
        assert_eq!(
            link.record.lock().unwrap().calls.last(),
            Some(&Call::Submit(XboxReport::NEUTRAL))
        );
        let count = link.record.lock().unwrap().calls.len();
        link.deliver(link.state(sequence, true), false).await;
        link.deliver(link.state(sequence + 1, true), true).await;
        let mut corrupt = link.state(sequence + 1, true);
        corrupt[36] ^= 1;
        link.deliver(corrupt, false).await;
        link.pump(Instant::now());
        assert_eq!(*link.states.borrow(), last);
        assert_eq!(link.record.lock().unwrap().calls.len(), count);
    }
    link.deliver(link.state(4, false), false).await;
    link.pump(Instant::now());
    assert!(!link.owner.health.timed_out);
    assert_eq!(link.owner.counters.timeouts, 3);
    let calls = link.record.lock().unwrap().calls.clone();
    assert_eq!(calls.iter().filter(|c| **c == Call::Attach).count(), 1);
    assert!(!calls.contains(&Call::Detach));
    assert_eq!(link.requests.borrow().unwrap().session, link.session);
    link.finish().await;
}

#[tokio::test]
async fn drop_duplicate_reorder_corrupt_source_session_and_silent_loss_campaign() {
    let mut link = FaultLink::new().await;
    link.deliver(link.state(u32::MAX - 2, true), false).await;
    link.pump(Instant::now());
    assert!(
        link.record
            .lock()
            .unwrap()
            .calls
            .contains(&Call::Submit(XboxReport {
                buttons: 0x1000,
                ..XboxReport::NEUTRAL
            }))
    );
    // Drop MAX-1 entirely; delay MAX and release at 0, crossing wrap.
    link.delayed[0] = Some(link.state(u32::MAX, true));
    link.deliver(link.state(0, false), false).await;
    let delayed = link.delayed[0].take().unwrap();
    link.deliver(delayed, false).await;
    link.deliver(link.state(0, false), false).await;
    let mut corrupt = link.state(1, true);
    corrupt[36] ^= 1;
    link.deliver(corrupt, false).await;
    let mut reserved = link.state(1, true);
    reserved[34] = 1;
    let crc = crc32fast::hash(&reserved[..36]).to_le_bytes();
    reserved[36..].copy_from_slice(&crc);
    link.deliver(reserved, false).await;
    link.deliver(link.state(1, true), true).await;
    let Message::ControllerState(mut wrong) = decode(&link.state(1, true)).unwrap() else {
        unreachable!()
    };
    wrong.session = SessionId(NonZeroU64::new(link.session.0.get() ^ 1).unwrap_or(NonZeroU64::MIN));
    if wrong.session == link.session {
        wrong.session = SessionId(NonZeroU64::new(2).unwrap());
    }
    link.deliver(Message::ControllerState(wrong).encode().unwrap(), false)
        .await;
    link.pump(Instant::now());
    assert_eq!(
        link.record.lock().unwrap().calls.last(),
        Some(&Call::Submit(XboxReport::NEUTRAL))
    );
    let counters = *link.metrics.borrow();
    assert_eq!(
        (counters.accepted, counters.stale, counters.estimated_loss),
        (2, 2, 2)
    );
    assert_eq!(
        (
            counters.invalid_crc,
            counters.invalid_reserved,
            counters.wrong_source,
            counters.wrong_session
        ),
        (1, 1, 1, 1)
    );
    let received = link.states.borrow().unwrap().received_at;
    // Stop without disconnect. Fake owner clock proves exact boundary independently of UDP delivery.
    link.pump(received + Duration::from_millis(999));
    assert!(link.owner.health.ready);
    link.pump(received + Duration::from_millis(1000));
    assert!(link.owner.health.timed_out);
    assert!(link.owner.health.attached);
    link.pump(received + Duration::from_secs(3));
    let count = link.record.lock().unwrap().calls.len();
    link.deliver(link.state(2, true), false).await;
    link.pump(received + Duration::from_secs(2));
    assert_eq!(link.record.lock().unwrap().calls.len(), count);
    assert_eq!(link.owner.counters.timeouts, 1);
    // Transition status must arrive well before the next periodic half second tick.
    timeout(Duration::from_millis(150), async {
        let mut buffer = [0; 64];
        loop {
            let (len, _) = link.phone.recv_from(&mut buffer).await.unwrap();
            if let Ok(Message::Status(status)) = decode(&buffer[..len])
                && status.flags & 8 != 0
            {
                assert_eq!(status.timeout_ms, 1000);
                break;
            }
        }
    })
    .await
    .unwrap();
    link.finish().await;
}

#[tokio::test]
async fn burst_with_delayed_consumer_applies_only_final_release() {
    let mut link = FaultLink::new().await;
    // Transport consumes a burst while output work is deliberately withheld.
    for sequence in 0..256 {
        link.deliver(link.state(sequence, sequence != 255), false)
            .await;
    }
    assert_eq!(link.states.borrow().unwrap().state.sequence, 255);
    link.pump(Instant::now());
    let calls = link.record.lock().unwrap().calls.clone();
    assert_eq!(
        calls
            .iter()
            .filter(|call| matches!(call, Call::Submit(_)))
            .count(),
        1
    );
    assert!(!calls.contains(&Call::Submit(XboxReport {
        buttons: 0x1000,
        ..XboxReport::NEUTRAL
    })));
    link.finish().await;
}

#[tokio::test]
async fn backend_fault_transition_refuses_updates_and_cleanup_remains_idempotent() {
    let mut link = FaultLink::new().await;
    link.record
        .lock()
        .unwrap()
        .failures
        .push(Call::Submit(XboxReport {
            buttons: 0x1000,
            ..XboxReport::NEUTRAL
        }));
    link.deliver(link.state(1, true), false).await;
    link.pump(Instant::now());
    assert!(link.owner.health.fault);
    assert_eq!(link.owner.counters.failures, 1);
    assert!(
        link.record
            .lock()
            .unwrap()
            .calls
            .ends_with(&[Call::Neutral, Call::Detach])
    );
    link.deliver(link.state(2, true), false).await;
    assert_eq!(link.metrics.borrow().accepted, 1);
    link.finish().await;
}

#[tokio::test]
async fn left_stick_a_udp_replaces_latest_and_rejects_stale_then_neutralizes() {
    let mut link = FaultLink::new().await;
    let Message::ControllerState(mut held) = decode(&link.state(u32::MAX, true)).unwrap() else {
        panic!()
    };
    held.left_x = i16::MIN;
    held.left_y = i16::MAX;
    let held_bytes = Message::ControllerState(held).encode().unwrap();
    link.deliver(held_bytes, false).await;
    link.pump(Instant::now());
    assert_eq!(
        link.record.lock().unwrap().calls.last(),
        Some(&Call::Submit(XboxReport {
            buttons: 0x1000,
            left_x: i16::MIN,
            left_y: i16::MAX,
            ..XboxReport::NEUTRAL
        }))
    );
    let release = link.state(0, false);
    link.deliver(release, false).await;
    link.deliver(held_bytes, false).await;
    assert_eq!(link.metrics.borrow().accepted, 2);
    assert_eq!(link.metrics.borrow().stale, 1);
    link.pump(Instant::now());
    assert_eq!(
        link.record.lock().unwrap().calls.last(),
        Some(&Call::Submit(XboxReport::NEUTRAL))
    );
    held.sequence = 1;
    link.deliver(Message::ControllerState(held).encode().unwrap(), false)
        .await;
    link.pump(Instant::now());
    let deadline = link.states.borrow().unwrap().received_at + Duration::from_millis(1000);
    link.pump(deadline);
    assert!(link.owner.health.timed_out);
    assert!(link.owner.health.attached);
    assert_eq!(
        link.record.lock().unwrap().calls.last(),
        Some(&Call::Submit(XboxReport::NEUTRAL))
    );
    link.pump(deadline + Duration::from_secs(2));
    assert!(
        link.record
            .lock()
            .unwrap()
            .calls
            .ends_with(&[Call::Neutral, Call::Detach])
    );
    link.finish().await;
}

#[tokio::test]
async fn mixed_burst_loss_reorder_and_wrong_peer_never_replay_after_latest_release() {
    use virtual_gamepad_agent::input::xbox_report;
    let mut link = FaultLink::new().await;
    let Message::ControllerState(mut mixed) =
        decode(include_bytes!("../../protocol/fixtures/v1/mixed.bin")).unwrap()
    else {
        panic!()
    };
    mixed.session = link.session;
    mixed.sequence = u32::MAX;
    let held = Message::ControllerState(mixed).encode().unwrap();
    link.deliver(held, false).await;
    link.pump(Instant::now());
    assert_eq!(
        link.record.lock().unwrap().calls.last(),
        Some(&Call::Submit(xbox_report(&mixed)))
    );
    // The consumer deliberately does not run during the bounded burst. Drop 0 and 4.
    for sequence in [1, 2, 3, 5, 6] {
        mixed.sequence = sequence;
        mixed.right_trigger = sequence as u8 * 30;
        let packet = Message::ControllerState(mixed).encode().unwrap();
        link.delayed[sequence as usize] = Some(packet);
        link.deliver(packet, false).await;
    }
    link.deliver(link.state(7, false), false).await;
    for sequence in [6, 2, 5, 3, 1] {
        link.deliver(link.delayed[sequence].unwrap(), false).await;
    }
    mixed.sequence = 8;
    let newest = Message::ControllerState(mixed).encode().unwrap();
    link.deliver(newest, true).await;
    mixed.session =
        SessionId(NonZeroU64::new(if link.session.0.get() == 1 { 2 } else { 1 }).unwrap());
    link.deliver(Message::ControllerState(mixed).encode().unwrap(), false)
        .await;
    let mut corrupt = newest;
    corrupt[36] ^= 1;
    link.deliver(corrupt, false).await;
    assert_eq!(link.states.borrow().unwrap().state.sequence, 7);
    assert_eq!(link.metrics.borrow().accepted, 7);
    assert_eq!(link.metrics.borrow().stale, 5);
    link.pump(Instant::now());
    assert_eq!(
        link.record.lock().unwrap().calls.last(),
        Some(&Call::Submit(XboxReport::NEUTRAL))
    );
    assert_eq!(link.owner.counters.updates, 2);
    link.finish().await;
}
