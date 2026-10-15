use std::{net::SocketAddr, num::NonZeroU64, sync::mpsc, time::Duration};
use tokio::sync::watch;
use virtual_gamepad_agent::{
    config::InputTimeout,
    input::{OutputHealth, SessionRequest},
    paired_session::{PairedSession, PairingCommand},
    pairing_store::{PairingRecord, StoreRequest},
    protocol::*,
    session::{AdmissionError, Handler},
};

struct Harness {
    handler: PairedSession,
    commands: mpsc::SyncSender<PairingCommand>,
    codes: mpsc::Receiver<(u32, tokio::time::Instant)>,
    store: mpsc::Receiver<StoreRequest>,
    requests: watch::Receiver<Option<SessionRequest>>,
    health: watch::Sender<OutputHealth>,
}
fn id(value: u64) -> InstanceId {
    InstanceId(NonZeroU64::new(value).unwrap())
}
fn hello(client: u64, nonce: u64) -> Hello {
    Hello {
        client_nonce: Nonce(NonZeroU64::new(nonce).unwrap()),
        client_instance: id(client),
        state_rate: 60,
        capabilities: 3,
        app_build: 1,
        api_level: 34,
    }
}
fn source(port: u16) -> SocketAddr {
    ([127, 0, 0, 1], port).into()
}
fn state(session: SessionId, sequence: u32) -> ControllerState {
    ControllerState {
        session,
        sequence,
        client_ms: 0,
        buttons: 1,
        left_x: 100,
        left_y: 0,
        right_x: 0,
        right_y: 0,
        left_trigger: 0,
        right_trigger: 0,
    }
}
impl Harness {
    fn new(client: Option<InstanceId>) -> Self {
        let (commands, command_rx) = mpsc::sync_channel(8);
        let (code_tx, codes) = mpsc::sync_channel(8);
        let (store_tx, store) = mpsc::sync_channel(1);
        let (request_tx, requests) = watch::channel(None);
        let (health, health_rx) = watch::channel(OutputHealth::default());
        let handler = PairedSession::new(
            PairingRecord {
                server: id(99),
                client,
            },
            store_tx,
            command_rx,
            request_tx,
            health_rx,
            InputTimeout::default(),
            code_tx,
        );
        Self {
            handler,
            commands,
            codes,
            store,
            requests,
            health,
        }
    }
    fn request(&mut self) -> PairRequest {
        self.commands.send(PairingCommand::Open).unwrap();
        self.handler.transition_status();
        let code = self.codes.try_recv().unwrap().0;
        let Message::HelloAck(ack) = self
            .handler
            .control(Message::Hello(hello(1, 10)), source(1))
            .unwrap()
        else {
            panic!()
        };
        assert!(ack.session.is_none());
        assert_eq!(ack.flags, 1);
        assert!(self.requests.borrow().is_none());
        PairRequest {
            client_nonce: ack.client_nonce,
            server_nonce: ack.server_nonce,
            client_instance: id(1),
            code,
            slot: 255,
        }
    }
    fn ready(&mut self) -> SessionId {
        self.handler.transition_status();
        let session = self.requests.borrow().unwrap().session;
        self.health.send_replace(OutputHealth {
            session: Some(session),
            ready: true,
            attached: true,
            ..Default::default()
        });
        self.handler.transition_status();
        session
    }
    fn pair(&mut self) -> (PairRequest, PairAccept) {
        let request = self.request();
        assert!(
            self.handler
                .control(Message::PairRequest(request), source(1))
                .is_none()
        );
        let session = self.ready();
        assert_eq!(
            self.handler.admit(&state(session, 1), source(1)),
            Err(AdmissionError::Refused)
        );
        let save = self.store.try_recv().unwrap();
        assert_eq!(save.record.client, Some(id(1)));
        save.result.send(Ok(())).unwrap();
        let (Message::PairAccept(accept), destination) = self.handler.transition_status().unwrap()
        else {
            panic!()
        };
        assert_eq!(destination, source(1));
        assert_eq!(accept.server_instance, id(99));
        (request, accept)
    }
}
#[tokio::test(start_paused = true)]
async fn production_pair_is_durable_before_input_and_duplicate_reply_does_not_rotate() {
    let mut h = Harness::new(None);
    let (request, accept) = h.pair();
    h.handler
        .admit(&state(accept.session, 1), source(1))
        .unwrap();
    assert_eq!(
        h.handler.control(Message::PairRequest(request), source(1)),
        Some(Message::PairAccept(accept))
    );
    assert_eq!(h.requests.borrow().unwrap().session, accept.session);
    assert_eq!(
        h.handler.admit(&state(accept.session, 1), source(1)),
        Err(AdmissionError::Stale)
    );
    assert!(
        h.handler
            .admit(&state(accept.session, 2), source(2))
            .is_err()
    );
    let Message::Status(status) = h.handler.status().unwrap().0 else {
        panic!()
    };
    assert_eq!(status.flags & 16, 16);
    let Message::HelloAck(other) = h
        .handler
        .control(Message::Hello(hello(2, 1)), source(2))
        .unwrap()
    else {
        panic!()
    };
    assert!(other.session.is_none());
    assert_eq!(h.requests.borrow().unwrap().session, accept.session);
}
#[tokio::test(start_paused = true)]
async fn every_connection_requires_code_but_brief_gap_keeps_session() {
    let mut h = Harness::new(Some(id(1)));
    let Message::HelloAck(ack) = h
        .handler
        .control(Message::Hello(hello(1, 1)), source(1))
        .unwrap()
    else {
        panic!()
    };
    assert_eq!(ack.flags, 1);
    assert!(ack.session.is_none());
    assert!(h.requests.borrow().is_none());
    let (_, first) = h.pair();
    h.handler
        .admit(&state(first.session, 1), source(1))
        .unwrap();
    tokio::time::advance(Duration::from_millis(300)).await;
    h.handler
        .admit(&state(first.session, 2), source(1))
        .unwrap();
    tokio::time::advance(Duration::from_secs(3)).await;
    assert!(
        h.handler
            .admit(&state(first.session, 3), source(1))
            .is_err()
    );
    let Message::HelloAck(ack) = h
        .handler
        .control(Message::Hello(hello(1, 2)), source(3))
        .unwrap()
    else {
        panic!()
    };
    assert!(ack.session.is_none());
    let (_, second) = h.pair();
    assert_ne!(first.session, second.session);
    assert!(
        h.handler
            .admit(&state(first.session, 4), source(1))
            .is_err()
    );
    h.handler.control(
        Message::Disconnect(Disconnect {
            session: second.session,
            reason: DisconnectReason::UserLeft,
        }),
        source(1),
    );
    assert!(h.requests.borrow().is_none());
    let Message::HelloAck(ack) = h
        .handler
        .control(Message::Hello(hello(1, 3)), source(1))
        .unwrap()
    else {
        panic!()
    };
    assert!(ack.session.is_none());
    assert_eq!(ack.flags, 1);
}

#[tokio::test(start_paused = true)]
async fn pair_command_revokes_current_session_before_issuing_another_code() {
    let mut h = Harness::new(None);
    let (_, first) = h.pair();
    h.handler
        .admit(&state(first.session, 1), source(1))
        .unwrap();
    h.commands.send(PairingCommand::Open).unwrap();
    h.handler.transition_status();
    assert!(h.requests.borrow().is_none());
    assert!(h.handler.take_reset());
    assert!(h.codes.try_recv().is_ok());
    assert!(
        h.handler
            .admit(&state(first.session, 2), source(1))
            .is_err()
    );
}
#[tokio::test(start_paused = true)]
async fn storage_failure_never_admits_and_forget_revokes_before_disk_completion() {
    let mut h = Harness::new(None);
    let request = h.request();
    h.handler.control(Message::PairRequest(request), source(1));
    let session = h.ready();
    h.store
        .try_recv()
        .unwrap()
        .result
        .send(Err(std::io::Error::other("disk full")))
        .unwrap();
    assert!(matches!(
        h.handler.transition_status(),
        Some((Message::PairReject(_), _))
    ));
    assert!(h.handler.admit(&state(session, 1), source(1)).is_err());
    assert!(h.requests.borrow().is_none());
    let mut h = Harness::new(None);
    let (_, accept) = h.pair();
    h.handler
        .admit(&state(accept.session, 1), source(1))
        .unwrap();
    h.commands.send(PairingCommand::Forget).unwrap();
    h.handler.transition_status();
    assert!(h.requests.borrow().is_none());
    assert!(h.handler.take_reset());
    assert!(
        h.handler
            .admit(&state(accept.session, 2), source(1))
            .is_err()
    );
    let save = h.store.try_recv().unwrap();
    assert_eq!(save.record.client, None);
    save.result.send(Ok(())).unwrap();
    h.handler.transition_status();
    let Message::HelloAck(ack) = h
        .handler
        .control(Message::Hello(hello(1, 11)), source(1))
        .unwrap()
    else {
        panic!()
    };
    assert_eq!(ack.flags, 1);
    assert!(ack.session.is_none());
}

#[tokio::test(start_paused = true)]
async fn late_storage_success_keeps_pairing_but_never_revives_expired_session() {
    let mut h = Harness::new(None);
    let request = h.request();
    h.handler.control(Message::PairRequest(request), source(1));
    let session = h.ready();
    let save = h.store.try_recv().unwrap();
    tokio::time::advance(Duration::from_secs(3)).await;
    save.result.send(Ok(())).unwrap();
    assert!(matches!(
        h.handler.transition_status(),
        Some((Message::PairReject(_), _))
    ));
    assert!(h.requests.borrow().is_none());
    assert!(h.handler.admit(&state(session, 1), source(1)).is_err());
    let Message::HelloAck(ack) = h
        .handler
        .control(Message::Hello(hello(1, 20)), source(2))
        .unwrap()
    else {
        panic!()
    };
    assert_eq!(ack.flags & 1, 1); // Saved metadata cannot authorize another session.
    assert!(h.requests.borrow().is_none());
}

#[tokio::test(start_paused = true)]
async fn cached_accept_never_outlives_pairing_deadline_or_backend_health() {
    for fault in [false, true] {
        let mut h = Harness::new(None);
        let (request, accept) = h.pair();
        if fault {
            h.health.send_replace(OutputHealth {
                fault: true,
                ..Default::default()
            });
        } else {
            tokio::time::advance(Duration::from_secs(3)).await;
        }
        assert!(!matches!(
            h.handler.control(Message::PairRequest(request), source(1)),
            Some(Message::PairAccept(_))
        ));
        assert!(
            h.handler
                .admit(&state(accept.session, 1), source(1))
                .is_err()
        );
    }
}

#[tokio::test(start_paused = true)]
async fn replacement_waits_for_durable_removal_and_second_phone_cannot_request_it() {
    let mut h = Harness::new(None);
    let (request, accept) = h.pair();
    assert!(matches!(
        h.handler.control(
            Message::PairRequest(PairRequest {
                client_instance: id(2),
                ..request
            }),
            source(2)
        ),
        Some(Message::PairReject(_))
    ));
    assert_eq!(h.requests.borrow().unwrap().session, accept.session);
    h.commands.send(PairingCommand::Replace).unwrap();
    h.handler.transition_status();
    assert!(h.requests.borrow().is_none());
    assert!(h.codes.try_recv().is_err());
    h.store.try_recv().unwrap().result.send(Ok(())).unwrap();
    h.handler.transition_status();
    assert!(h.codes.try_recv().is_ok());
    assert!(
        h.handler
            .admit(&state(accept.session, 2), source(1))
            .is_err()
    );
}
