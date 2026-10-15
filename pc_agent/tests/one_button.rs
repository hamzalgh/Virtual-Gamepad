use std::{
    net::SocketAddr,
    num::NonZeroU64,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::{sync::watch, time::Instant};
use virtual_gamepad_agent::{
    input::{INPUT_TIMEOUT, OutputHealth, OutputOwner, SessionRequest},
    protocol::*,
    session::{AdmissionError, DevelopmentTrust, Handler},
    virtual_gamepad::fake::RecordingBackend,
};

fn source(port: u16) -> SocketAddr {
    ([127, 0, 0, 1], port).into()
}
fn hello(instance: u64, nonce: u64) -> Message {
    Message::Hello(Hello {
        client_nonce: Nonce(NonZeroU64::new(nonce).unwrap()),
        client_instance: InstanceId(NonZeroU64::new(instance).unwrap()),
        state_rate: 60,
        capabilities: 1,
        app_build: 1,
        api_level: 34,
    })
}
fn state(session: SessionId, sequence: u32) -> ControllerState {
    ControllerState {
        session,
        sequence,
        client_ms: sequence,
        buttons: 1,
        left_x: 0,
        left_y: 0,
        right_x: 0,
        right_y: 0,
        left_trigger: 0,
        right_trigger: 0,
    }
}
fn ack(value: Option<Message>) -> HelloAck {
    let Some(Message::HelloAck(ack)) = value else {
        panic!("missing ack")
    };
    ack
}

#[tokio::test(start_paused = true)]
async fn expired_session_cannot_revive_while_output_health_lags() {
    if !cfg!(debug_assertions) {
        return;
    }
    let (requests, rx) = watch::channel(None::<SessionRequest>);
    let (health, health_rx) = watch::channel(OutputHealth::default());
    let mut handler = DevelopmentTrust::with_output(true, requests, health_rx);
    handler.control(hello(1, 1), source(1000));
    let session = rx.borrow().unwrap().session;
    health.send_replace(OutputHealth {
        session: Some(session),
        ready: true,
        attached: true,
        ..Default::default()
    });
    handler.control(hello(1, 2), source(1000));
    assert!(
        handler
            .admit(&state(session, u32::MAX), source(1000))
            .is_ok()
    );
    tokio::time::advance(INPUT_TIMEOUT).await;
    health.send_replace(OutputHealth {
        session: Some(session),
        ready: true,
        attached: true,
        timed_out: true,
        ..Default::default()
    });
    assert_eq!(
        handler.admit(&state(session, u32::MAX), source(1000)),
        Err(AdmissionError::Stale)
    );
    assert_eq!(
        handler.admit(&state(session, 0), source(1001)),
        Err(AdmissionError::WrongSource)
    );
    assert!(handler.admit(&state(session, 0), source(1000)).is_ok());
    tokio::time::advance(virtual_gamepad_agent::input::SESSION_IDLE_TIMEOUT).await;
    // Output has not run and still advertises ready. Admission must enforce expiry itself.
    assert!(handler.admit(&state(session, 2), source(1000)).is_err());
    assert!(
        ack(handler.control(hello(1, 2), source(1000)))
            .session
            .is_none()
    );
}

#[tokio::test(start_paused = true)]
async fn fifty_explicit_reconnects_reject_old_sessions_and_clean_up_once() {
    if !cfg!(debug_assertions) {
        return;
    }
    use virtual_gamepad_agent::{input::AcceptedState, virtual_gamepad::fake::Call};
    let (requests, rx) = watch::channel(None::<SessionRequest>);
    let (health, health_rx) = watch::channel(OutputHealth::default());
    let mut handler = DevelopmentTrust::with_output(true, requests, health_rx);
    let record = Arc::new(Mutex::new(RecordingBackend::default()));
    let mut owner = OutputOwner::new(record.clone());
    let mut previous = None;
    for cycle in 0..50u64 {
        let address = source(1000 + cycle as u16);
        handler.control(hello(1, cycle * 2 + 1), address);
        owner.step(*rx.borrow(), None, Instant::now());
        health.send_replace(owner.health);
        let granted = ack(handler.control(hello(1, cycle * 2 + 2), address));
        assert_eq!(granted.timeout_ms, 250);
        let session = granted.session.unwrap();
        if let Some(old) = previous {
            assert!(handler.admit(&state(old, 2), address).is_err());
        }
        let press = state(session, 1);
        assert!(handler.admit(&press, address).is_ok());
        owner.step(
            *rx.borrow(),
            Some(AcceptedState {
                state: press,
                received_at: Instant::now(),
            }),
            Instant::now(),
        );
        let disconnect = Message::Disconnect(Disconnect {
            session,
            reason: DisconnectReason::Lifecycle,
        });
        handler.control(disconnect, source(9000));
        assert!(rx.borrow().is_some());
        handler.control(disconnect, address);
        handler.control(disconnect, address);
        assert!(rx.borrow().is_none());
        owner.step(None, None, Instant::now());
        health.send_replace(owner.health);
        assert!(
            record
                .lock()
                .unwrap()
                .calls
                .ends_with(&[Call::Neutral, Call::Detach])
        );
        previous = Some(session);
    }
    owner.shutdown().unwrap();
    let calls = &record.lock().unwrap().calls;
    assert_eq!(calls.iter().filter(|c| **c == Call::Attach).count(), 50);
    assert_eq!(calls.iter().filter(|c| **c == Call::Detach).count(), 50);
}

#[test]
fn readiness_source_rotation_sequence_ranges_and_release_boundary() {
    let (requests, rx) = watch::channel(None::<SessionRequest>);
    let (health, health_rx) = watch::channel(OutputHealth::default());
    let mut handler = DevelopmentTrust::with_output(true, requests, health_rx);
    let first = handler.control(hello(1, 1), source(1000));
    if !cfg!(debug_assertions) {
        assert!(first.is_none());
        assert!(rx.borrow().is_none());
        return;
    }
    assert!(ack(first).session.is_none());
    let request = rx.borrow().unwrap();
    assert!(!request.armed);
    assert!(handler.take_reset());
    let mut owner = OutputOwner::new(Arc::new(Mutex::new(RecordingBackend::default())));
    owner.step(Some(request), None, Instant::now());
    health.send_replace(owner.health);
    let session = ack(handler.control(hello(1, 2), source(1000)))
        .session
        .unwrap();
    assert!(rx.borrow().unwrap().armed);
    assert_eq!(
        handler.admit(&state(session, u32::MAX), source(2000)),
        Err(AdmissionError::WrongSource)
    );
    let wrong = SessionId(NonZeroU64::new(session.0.get().wrapping_add(1).max(1)).unwrap());
    assert_eq!(
        handler.admit(&state(wrong, 0), source(1000)),
        Err(AdmissionError::WrongSession)
    );
    assert!(
        handler
            .admit(&state(session, u32::MAX), source(1000))
            .is_ok()
    );
    assert!(handler.admit(&state(session, 0), source(1000)).is_ok());
    assert_eq!(
        handler.admit(&state(session, 0), source(1000)),
        Err(AdmissionError::Stale)
    );
    assert_eq!(
        handler.admit(&state(session, u32::MAX), source(1000)),
        Err(AdmissionError::Stale)
    );
    for invalid_bits in [0x4000, 0x8000, 0x80000000, 0xc00, 0x3000] {
        let mut invalid = state(session, 1);
        invalid.buttons = invalid_bits;
        assert_eq!(
            handler.admit(&invalid, source(1000)),
            Err(AdmissionError::UnsupportedState)
        );
    }
    assert!(handler.admit(&state(session, 1), source(1000)).is_ok());
    let deadline = rx.borrow().unwrap().expires_at;
    assert_eq!(
        ack(handler.control(hello(1, 2), source(1000))).session,
        Some(session)
    );
    assert_eq!(rx.borrow().unwrap().expires_at, deadline);
    assert!(
        ack(handler.control(hello(2, 3), source(2000)))
            .session
            .is_none()
    );
    assert!(
        ack(handler.control(hello(1, 4), source(3000)))
            .session
            .is_none()
    );
    assert!(handler.take_reset());
    assert_ne!(rx.borrow().unwrap().session, session);
    assert!(handler.admit(&state(session, 2), source(1000)).is_err());
    assert!(handler.admit(&state(session, 2), source(3000)).is_err());
}

#[test]
fn timeout_fault_and_disconnect_never_claim_pairing_or_keep_admission() {
    if !cfg!(debug_assertions) {
        return;
    }
    let (requests, rx) = watch::channel(None::<SessionRequest>);
    let (health, health_rx) = watch::channel(OutputHealth::default());
    let mut handler = DevelopmentTrust::with_output(true, requests, health_rx);
    handler.control(hello(1, 1), source(1000));
    let id = rx.borrow().unwrap().session;
    health.send_replace(OutputHealth {
        session: Some(id),
        ready: true,
        attached: true,
        ..Default::default()
    });
    assert!(
        ack(handler.control(hello(1, 2), source(1000)))
            .session
            .is_some()
    );
    let status = handler.status().unwrap().0;
    assert!(matches!(status, Message::Status(Status { flags: 6, .. })));
    let mut value = *health.borrow();
    value.ready = false;
    value.attached = false;
    value.timed_out = true;
    health.send_replace(value);
    assert!(handler.admit(&state(id, 1), source(1000)).is_err());
    assert!(matches!(
        handler.status().unwrap().0,
        Message::Status(Status {
            flags: 8,
            session: Some(_),
            ..
        })
    ));
    value.timed_out = false;
    value.fault = true;
    health.send_replace(value);
    assert!(
        ack(handler.control(hello(1, 3), source(1000)))
            .session
            .is_none()
    );
    handler.control(
        Message::Disconnect(Disconnect {
            session: id,
            reason: DisconnectReason::Lifecycle,
        }),
        source(2000),
    );
    assert!(rx.borrow().is_some());
    handler.control(
        Message::Disconnect(Disconnect {
            session: id,
            reason: DisconnectReason::Lifecycle,
        }),
        source(1000),
    );
    assert!(rx.borrow().is_none());
    assert!(handler.take_reset());
    assert!(handler.admit(&state(id, 2), source(1000)).is_err());
    assert!(INPUT_TIMEOUT < Duration::from_secs(1));
}

#[tokio::test]
async fn real_udp_a_timeout_invalid_storm_reconnect_and_disconnect() {
    if !cfg!(debug_assertions) {
        return;
    }
    use tokio::{
        net::UdpSocket,
        sync::oneshot,
        time::{sleep, timeout},
    };
    use virtual_gamepad_agent::{
        input::{LatestState, run_output},
        network::Receiver,
        virtual_gamepad::{XboxReport, fake::Call},
    };
    async fn receive_ack(phone: &UdpSocket, nonce: u64) -> HelloAck {
        timeout(Duration::from_secs(2), async {
            let mut buffer = [0; 64];
            loop {
                let (len, _) = phone.recv_from(&mut buffer).await.unwrap();
                if let Ok(Message::HelloAck(ack)) = decode(&buffer[..len])
                    && ack.client_nonce.0.get() == nonce
                {
                    return ack;
                }
            }
        })
        .await
        .unwrap()
    }
    async fn ready(health: &mut watch::Receiver<OutputHealth>, old: Option<SessionId>) {
        timeout(Duration::from_secs(2), async {
            loop {
                let value = *health.borrow();
                if value.ready && value.session != old {
                    return;
                }
                health.changed().await.unwrap();
            }
        })
        .await
        .unwrap();
    }
    let server = UdpSocket::bind(source(0)).await.unwrap();
    let address = server.local_addr().unwrap();
    let phone = UdpSocket::bind(source(0)).await.unwrap();
    let stranger = UdpSocket::bind(source(0)).await.unwrap();
    let (requests, request_rx) = watch::channel(None);
    let (health_tx, mut health) = watch::channel(OutputHealth::default());
    let (latest, snapshots) = LatestState::channel();
    let observed = snapshots.clone();
    let record = Arc::new(Mutex::new(RecordingBackend::default()));
    let output = tokio::spawn(run_output(record.clone(), request_rx, snapshots, health_tx));
    let handler = DevelopmentTrust::with_output(true, requests, health.clone());
    let (receiver, metrics) = Receiver::new(server, handler, latest);
    let (stop, stopped) = oneshot::channel();
    let network = tokio::spawn(receiver.run(async {
        let _ = stopped.await;
    }));
    phone
        .send_to(&hello(1, 1).encode().unwrap(), address)
        .await
        .unwrap();
    assert!(receive_ack(&phone, 1).await.session.is_none());
    ready(&mut health, None).await;
    phone
        .send_to(&hello(1, 2).encode().unwrap(), address)
        .await
        .unwrap();
    let session = receive_ack(&phone, 2).await.session.unwrap();
    let press = Message::ControllerState(state(session, 1))
        .encode()
        .unwrap();
    phone.send_to(&press, address).await.unwrap();
    timeout(Duration::from_secs(1), async {
        loop {
            if record
                .lock()
                .unwrap()
                .calls
                .contains(&Call::Submit(XboxReport {
                    buttons: 0x1000,
                    ..XboxReport::NEUTRAL
                }))
            {
                break;
            }
            sleep(Duration::from_millis(1)).await;
        }
    })
    .await
    .unwrap();
    let admitted = *observed.borrow();
    let mut bad_crc = Message::ControllerState(state(session, 2))
        .encode()
        .unwrap();
    bad_crc[36] ^= 1;
    let other_controls = *include_bytes!("../../protocol/fixtures/v1/reserved_buttons.bin");
    let newer = Message::ControllerState(state(session, 2))
        .encode()
        .unwrap();
    // Invalid CRC, forbidden controls, duplicate sequence and wrong source never refresh the slot.
    for _ in 0..30 {
        phone.send_to(&bad_crc, address).await.unwrap();
        phone.send_to(&other_controls, address).await.unwrap();
        phone.send_to(&press, address).await.unwrap();
        stranger.send_to(&newer, address).await.unwrap();
        sleep(Duration::from_millis(10)).await;
    }
    assert_eq!(*observed.borrow(), admitted);
    assert_eq!(metrics.borrow().accepted, 1);
    assert!(health.borrow().timed_out);
    assert!(health.borrow().attached);
    assert_eq!(
        record.lock().unwrap().calls.last(),
        Some(&Call::Submit(XboxReport::NEUTRAL))
    );
    sleep(Duration::from_secs(3)).await;
    assert!(!health.borrow().attached);
    assert!(
        record
            .lock()
            .unwrap()
            .calls
            .ends_with(&[Call::Neutral, Call::Detach])
    );
    phone.send_to(&newer, address).await.unwrap();
    phone
        .send_to(&hello(1, 3).encode().unwrap(), address)
        .await
        .unwrap();
    assert!(receive_ack(&phone, 3).await.session.is_none());
    ready(&mut health, Some(session)).await;
    phone
        .send_to(&hello(1, 4).encode().unwrap(), address)
        .await
        .unwrap();
    let new_session = receive_ack(&phone, 4).await.session.unwrap();
    assert_ne!(new_session, session);
    assert!(observed.borrow().is_none());
    phone
        .send_to(
            &Message::ControllerState(state(new_session, 1))
                .encode()
                .unwrap(),
            address,
        )
        .await
        .unwrap();
    phone
        .send_to(
            &Message::Disconnect(Disconnect {
                session: new_session,
                reason: DisconnectReason::Lifecycle,
            })
            .encode()
            .unwrap(),
            address,
        )
        .await
        .unwrap();
    timeout(Duration::from_secs(1), async {
        loop {
            if health.borrow().session.is_none() {
                break;
            }
            health.changed().await.unwrap();
        }
    })
    .await
    .unwrap();
    assert!(observed.borrow().is_none());
    stop.send(()).unwrap();
    timeout(Duration::from_secs(1), network)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    timeout(Duration::from_secs(1), output)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert!(UdpSocket::bind(address).await.is_ok());
}
