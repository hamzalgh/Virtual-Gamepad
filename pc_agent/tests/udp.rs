use std::{net::SocketAddr, time::Duration};
use tokio::{
    net::UdpSocket,
    sync::{oneshot, watch},
    time::timeout,
};
use virtual_gamepad_agent::{
    input::{LatestReceiver, LatestState},
    network::Receiver,
    protocol::*,
    session::{AdmissionError, DenyAll, Handler, is_newer},
    telemetry::Counters,
};

fn fixture(name: &str) -> Vec<u8> {
    std::fs::read(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../protocol/fixtures/v1")
            .join(format!("{name}.bin")),
    )
    .unwrap()
}
fn state(seq: u32) -> ControllerState {
    let Message::ControllerState(mut s) = decode(&fixture("mixed")).unwrap() else {
        panic!()
    };
    s.sequence = seq;
    s
}
// This deliberately lives only in the integration test, with no runtime trust flag.
struct TestHandler {
    source: SocketAddr,
    session: SessionId,
    last: Option<u32>,
}
impl Handler for TestHandler {
    fn control(&mut self, message: Message, _: SocketAddr) -> Option<Message> {
        if let Message::Hello(h) = message {
            let Message::HelloAck(mut ack) = decode(&fixture("hello_ack_pending")).unwrap() else {
                panic!()
            };
            ack.client_nonce = h.client_nonce;
            Some(Message::HelloAck(ack))
        } else {
            None
        }
    }
    fn admit(&mut self, state: &ControllerState, source: SocketAddr) -> Result<(), AdmissionError> {
        if state.session != self.session {
            return Err(AdmissionError::WrongSession);
        }
        if source != self.source {
            return Err(AdmissionError::WrongSource);
        }
        if self
            .last
            .is_some_and(|last| !is_newer(state.sequence, last))
        {
            return Err(AdmissionError::Stale);
        }
        self.last = Some(state.sequence);
        Ok(())
    }
}
async fn socket() -> UdpSocket {
    UdpSocket::bind("127.0.0.1:0").await.unwrap()
}
async fn metrics_until(
    rx: &mut watch::Receiver<Counters>,
    predicate: impl Fn(Counters) -> bool,
) -> Counters {
    timeout(Duration::from_secs(3), async {
        loop {
            let c = *rx.borrow_and_update();
            if predicate(c) {
                return c;
            }
            rx.changed().await.unwrap();
        }
    })
    .await
    .expect("UDP counters did not converge")
}
struct Running {
    addr: SocketAddr,
    latest: LatestReceiver,
    metrics: watch::Receiver<Counters>,
    stop: oneshot::Sender<()>,
    task: tokio::task::JoinHandle<std::io::Result<Counters>>,
}
async fn start<H: Handler + Send + 'static>(handler: H) -> Running {
    let (latest, rx) = LatestState::channel();
    let (server, metrics) = Receiver::new(socket().await, handler, latest);
    let addr = server.local_addr().unwrap();
    let (stop, stopped) = oneshot::channel();
    let task = tokio::spawn(server.run(async {
        let _ = stopped.await;
    }));
    Running {
        addr,
        latest: rx,
        metrics,
        stop,
        task,
    }
}
async fn accepting(client: &UdpSocket) -> Running {
    start(TestHandler {
        source: client.local_addr().unwrap(),
        session: state(0).session,
        last: None,
    })
    .await
}
async fn stop(r: Running) -> Counters {
    r.stop.send(()).unwrap();
    let counts = timeout(Duration::from_secs(1), r.task)
        .await
        .expect("shutdown delayed")
        .unwrap()
        .unwrap();
    assert!(r.latest.borrow().is_none());
    assert!(r.metrics.has_changed().is_err(), "metrics publisher leaked");
    let rebound = UdpSocket::bind(r.addr).await.unwrap();
    assert_eq!(rebound.local_addr().unwrap(), r.addr);
    counts
}
#[tokio::test]
async fn real_loopback_hello_reply_uses_same_socket() {
    let client = socket().await;
    let r = accepting(&client).await;
    client.send_to(&fixture("hello"), r.addr).await.unwrap();
    let mut b = [0; 64];
    let (len, source) = timeout(Duration::from_secs(2), client.recv_from(&mut b))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(source, r.addr);
    assert_eq!(&b[..len], fixture("hello_ack_pending"));
    assert!(r.latest.borrow().is_none());
    let counts = stop(r).await;
    assert_eq!(counts.replies, 1);
    assert_eq!(counts.accepted, 0);
}
#[tokio::test]
async fn burst_overwrites_slot_without_consumer_backlog() {
    let client = socket().await;
    let mut r = accepting(&client).await;
    for sequence in 0..64 {
        client
            .send_to(
                &Message::ControllerState(state(sequence)).encode().unwrap(),
                r.addr,
            )
            .await
            .unwrap();
    }
    metrics_until(&mut r.metrics, |c| c.accepted == 64).await;
    // No state reads occurred while the receiver processed the complete burst.
    r.latest.changed().await.unwrap();
    assert_eq!(r.latest.borrow_and_update().unwrap().state.sequence, 63);
    assert!(
        !r.latest.has_changed().unwrap(),
        "intermediate values must not queue"
    );
    let counts = stop(r).await;
    assert_eq!(counts.accepted, 64);
}
#[tokio::test]
async fn invalid_stale_wrong_source_and_session_preserve_latest_and_timestamp() {
    let client = socket().await;
    let other = socket().await;
    let mut r = accepting(&client).await;
    client
        .send_to(
            &Message::ControllerState(state(u32::MAX)).encode().unwrap(),
            r.addr,
        )
        .await
        .unwrap();
    metrics_until(&mut r.metrics, |c| c.accepted == 1).await;
    client
        .send_to(
            &Message::ControllerState(state(0)).encode().unwrap(),
            r.addr,
        )
        .await
        .unwrap();
    metrics_until(&mut r.metrics, |c| c.accepted == 2).await;
    let original = *r.latest.borrow_and_update();
    for seq in [0, u32::MAX, 1 << 31] {
        client
            .send_to(
                &Message::ControllerState(state(seq)).encode().unwrap(),
                r.addr,
            )
            .await
            .unwrap();
    }
    other
        .send_to(
            &Message::ControllerState(state(100)).encode().unwrap(),
            r.addr,
        )
        .await
        .unwrap();
    let mut wrong = state(100);
    wrong.session = SessionId(1.try_into().unwrap());
    client
        .send_to(&Message::ControllerState(wrong).encode().unwrap(), r.addr)
        .await
        .unwrap();
    client.send_to(&fixture("bad_crc"), r.addr).await.unwrap();
    metrics_until(&mut r.metrics, |c| c.decoded == 7 && c.invalid == 1).await;
    assert_eq!(*r.latest.borrow(), original);
    assert!(!r.latest.has_changed().unwrap());
    // Refused packets must not advance even the test hook's freshness baseline.
    client
        .send_to(
            &Message::ControllerState(state(1)).encode().unwrap(),
            r.addr,
        )
        .await
        .unwrap();
    metrics_until(&mut r.metrics, |c| c.accepted == 3).await;
    let c = stop(r).await;
    assert_eq!(c.stale, 3);
    assert_eq!(c.refused, 2);
    assert_eq!(c.invalid, 1);
}
#[tokio::test]
async fn exact_and_maximum_udp_lengths_are_safe() {
    let client = socket().await;
    let mut r = accepting(&client).await;
    for length in [0, 39, 41, 1024, 65507] {
        let mut b = vec![0; length];
        let count = length.min(40);
        b[..count].copy_from_slice(&fixture("neutral")[..count]);
        client.send_to(&b, r.addr).await.unwrap();
        metrics_until(&mut r.metrics, |c| {
            c.invalid
                >= if length == 0 {
                    1
                } else if length == 39 {
                    2
                } else if length == 41 {
                    3
                } else if length == 1024 {
                    4
                } else {
                    5
                }
        })
        .await;
        assert!(r.latest.borrow().is_none());
    }
    client.send_to(&fixture("neutral"), r.addr).await.unwrap();
    metrics_until(&mut r.metrics, |c| c.accepted == 1).await;
    let c = stop(r).await;
    assert_eq!(c.invalid, 5);
    assert_eq!(c.oversized, 3);
    assert_eq!(c.last_rejection, Some(DecodeError::Length));
}
#[tokio::test]
async fn default_shell_never_admits_or_replies() {
    let client = socket().await;
    let mut r = start(DenyAll).await;
    for name in ["hello", "pair_request", "status", "disconnect_0", "neutral"] {
        client.send_to(&fixture(name), r.addr).await.unwrap();
    }
    metrics_until(&mut r.metrics, |c| c.decoded == 5).await;
    assert!(r.latest.borrow().is_none());
    let c = stop(r).await;
    assert_eq!(c.accepted, 0);
    assert_eq!(c.refused, 2);
    assert_eq!(c.replies, 0);
}
#[tokio::test]
async fn idle_shutdown_and_dropped_cancel_sender_close_promptly() {
    stop(start(DenyAll).await).await;
    let r = start(DenyAll).await;
    drop(r.stop);
    timeout(Duration::from_secs(1), r.task)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert!(r.latest.borrow().is_none());
    UdpSocket::bind(r.addr).await.unwrap();
}
#[tokio::test]
async fn shutdown_remains_responsive_during_malformed_traffic() {
    let client = socket().await;
    let mut r = start(DenyAll).await;
    for _ in 0..64 {
        client.send_to(&[0; 40], r.addr).await.unwrap();
    }
    metrics_until(&mut r.metrics, |c| c.invalid > 0).await;
    stop(r).await;
}

#[test]
fn shared_admission_scenarios_use_explicit_test_policy() {
    let manifest = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../protocol/fixtures/v1/admission.txt"),
    )
    .unwrap();
    let source: SocketAddr = "127.0.0.1:10001".parse().unwrap();
    let other: SocketAddr = "127.0.0.1:10002".parse().unwrap();
    let mut handler = TestHandler {
        source,
        session: state(0).session,
        last: None,
    };
    let mut scenario = "";
    for line in manifest.lines().filter(|l| !l.starts_with('#')) {
        let cols: Vec<_> = line.split('|').collect();
        assert_eq!(cols.len(), 4);
        if cols[0] != scenario {
            handler.last = None;
            scenario = cols[0];
        }
        let Message::ControllerState(s) =
            decode(&fixture(cols[1].strip_suffix(".bin").unwrap())).unwrap()
        else {
            panic!()
        };
        let endpoint = match cols[2] {
            "peer" => source,
            "other" => other,
            _ => panic!(),
        };
        let expected = match cols[3] {
            "Accepted" => Ok(()),
            "Stale" => Err(AdmissionError::Stale),
            "WrongSession" => Err(AdmissionError::WrongSession),
            "WrongSource" => Err(AdmissionError::WrongSource),
            _ => panic!(),
        };
        assert_eq!(handler.admit(&s, endpoint), expected, "{line}");
    }
}

#[tokio::test]
async fn wrong_direction_messages_are_refused_before_control_handler() {
    struct MustNotRun;
    impl Handler for MustNotRun {
        fn control(&mut self, _: Message, _: SocketAddr) -> Option<Message> {
            panic!("wrong direction reached handler");
        }
        fn admit(&mut self, _: &ControllerState, _: SocketAddr) -> Result<(), AdmissionError> {
            panic!("wrong direction reached admission");
        }
    }
    let client = socket().await;
    let mut r = start(MustNotRun).await;
    for name in [
        "hello_ack_pending",
        "pair_accept",
        "pair_reject_0",
        "status",
    ] {
        client.send_to(&fixture(name), r.addr).await.unwrap();
    }
    let c = metrics_until(&mut r.metrics, |c| c.decoded == 4).await;
    assert_eq!(c.refused, 4);
    assert_eq!(c.replies, 0);
    assert!(r.latest.borrow().is_none());
    stop(r).await;
}

#[tokio::test]
async fn invalid_handler_reply_does_not_stop_listener() {
    struct BadFirstReply(bool);
    impl Handler for BadFirstReply {
        fn control(&mut self, _: Message, _: SocketAddr) -> Option<Message> {
            let Message::HelloAck(mut ack) = decode(&fixture("hello_ack_pending")).unwrap() else {
                panic!()
            };
            if !self.0 {
                ack.timeout_ms = 0;
                self.0 = true;
            }
            Some(Message::HelloAck(ack))
        }
        fn admit(&mut self, _: &ControllerState, _: SocketAddr) -> Result<(), AdmissionError> {
            Err(AdmissionError::Refused)
        }
    }
    let client = socket().await;
    let mut r = start(BadFirstReply(false)).await;
    client.send_to(&fixture("hello"), r.addr).await.unwrap();
    metrics_until(&mut r.metrics, |c| c.decoded == 1).await;
    assert!(!r.task.is_finished(), "invalid reply stopped listener");
    client.send_to(&fixture("hello"), r.addr).await.unwrap();
    let mut b = [0; 64];
    let (len, _) = timeout(Duration::from_secs(2), client.recv_from(&mut b))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(&b[..len], fixture("hello_ack_pending"));
    let c = stop(r).await;
    assert_eq!(c.replies, 1);
    assert_eq!(c.invalid, 0);
    assert_eq!(c.reply_errors, 1);
}
