//! Spec 0006: codec, admission, mapping and cleanup for the supported controls.
#![cfg(debug_assertions)]
use std::{
    net::SocketAddr,
    num::NonZeroU64,
    sync::{Arc, Mutex},
};
use tokio::{sync::watch, time::Instant};
use virtual_gamepad_agent::{
    input::{AcceptedState, INPUT_TIMEOUT, OutputHealth, OutputOwner, SessionRequest, xbox_report},
    protocol::*,
    session::{AdmissionError, DevelopmentTrust, Handler},
    virtual_gamepad::{
        XboxReport,
        fake::{Call, RecordingBackend},
    },
};

fn state(session: SessionId, sequence: u32, x: i16, y: i16, a: bool) -> ControllerState {
    ControllerState {
        session,
        sequence,
        client_ms: sequence,
        buttons: u32::from(a),
        left_x: x,
        left_y: y,
        right_x: 0,
        right_y: 0,
        left_trigger: 0,
        right_trigger: 0,
    }
}

#[test]
fn codec_and_mapper_preserve_cardinals_diagonals_and_a() {
    let session = SessionId(NonZeroU64::new(77).unwrap());
    for (x, y) in [
        (i16::MIN, 0),
        (i16::MAX, 0),
        (0, i16::MIN),
        (0, i16::MAX),
        (23170, 23170),
        (-23170, -23170),
        (-23170, 23170),
        (23170, -23170),
        (0, 0),
    ] {
        for a in [false, true] {
            let original = state(session, 0, x, y, a);
            let bytes = Message::ControllerState(original).encode().unwrap();
            assert_eq!(bytes.len(), 40);
            let Message::ControllerState(decoded) = decode(&bytes).unwrap() else {
                panic!()
            };
            assert_eq!(decoded, original);
            assert_eq!(
                xbox_report(&decoded),
                XboxReport {
                    buttons: if a { 0x1000 } else { 0 },
                    left_x: x,
                    left_y: y,
                    ..XboxReport::NEUTRAL
                }
            );
        }
    }
}

#[tokio::test(start_paused = true)]
async fn stick_a_admission_wrap_rejection_and_timeout_remain_strict() {
    let source: SocketAddr = "127.0.0.1:1000".parse().unwrap();
    let (requests, rx) = watch::channel(None::<SessionRequest>);
    let (health, health_rx) = watch::channel(OutputHealth::default());
    let mut handler = DevelopmentTrust::with_output(true, requests, health_rx);
    let hello = |nonce| {
        Message::Hello(Hello {
            client_nonce: Nonce(NonZeroU64::new(nonce).unwrap()),
            client_instance: InstanceId(NonZeroU64::new(1).unwrap()),
            state_rate: 60,
            capabilities: 1,
            app_build: 1,
            api_level: 34,
        })
    };
    handler.control(hello(1), source);
    let session = rx.borrow().unwrap().session;
    health.send_replace(OutputHealth {
        session: Some(session),
        ready: true,
        attached: true,
        ..Default::default()
    });
    handler.control(hello(2), source);
    let held = state(session, u32::MAX, i16::MIN, i16::MAX, true);
    assert_eq!(
        handler.admit(&held, "127.0.0.1:1001".parse().unwrap()),
        Err(AdmissionError::WrongSource)
    );
    let mut wrong = held;
    wrong.session = SessionId(NonZeroU64::new(if session.0.get() == 1 { 2 } else { 1 }).unwrap());
    assert_eq!(
        handler.admit(&wrong, source),
        Err(AdmissionError::WrongSession)
    );
    assert!(handler.admit(&held, source).is_ok());
    let release = state(session, 0, 0, 0, false);
    assert!(handler.admit(&release, source).is_ok());
    assert_eq!(handler.admit(&held, source), Err(AdmissionError::Stale));
    assert_eq!(handler.admit(&release, source), Err(AdmissionError::Stale));
    tokio::time::advance(
        virtual_gamepad_agent::input::SESSION_IDLE_TIMEOUT - std::time::Duration::from_millis(1),
    )
    .await;
    for invalid_bits in [0x4000, 0x8000, 0x80000000, 0xc00, 0x3000] {
        let mut invalid = state(session, 1, i16::MAX, i16::MIN, true);
        invalid.buttons = invalid_bits;
        assert_eq!(
            handler.admit(&invalid, source),
            Err(AdmissionError::UnsupportedState)
        );
    }
    tokio::time::advance(std::time::Duration::from_millis(1)).await;
    assert!(
        handler
            .admit(&state(session, 1, 0, 1, true), source)
            .is_err()
    );
}

#[test]
fn held_axes_and_a_neutralize_on_timeout_disconnect_fault_and_shutdown() {
    let session = SessionId(NonZeroU64::new(77).unwrap());
    for reason in 0..4 {
        let record = Arc::new(Mutex::new(RecordingBackend::default()));
        let mut owner = OutputOwner::new(record.clone());
        let start = Instant::now();
        let request = Some(SessionRequest {
            session,
            expires_at: start + INPUT_TIMEOUT,
            armed: true,
        });
        let held = AcceptedState {
            state: state(session, 1, i16::MIN, i16::MAX, true),
            received_at: start,
        };
        owner.step(request, Some(held), start);
        assert!(
            record
                .lock()
                .unwrap()
                .calls
                .contains(&Call::Submit(XboxReport {
                    buttons: 0x1000,
                    left_x: i16::MIN,
                    left_y: i16::MAX,
                    ..XboxReport::NEUTRAL
                }))
        );
        match reason {
            0 => {
                owner.step(request, Some(held), start + INPUT_TIMEOUT);
                assert!(owner.health.attached);
                assert_eq!(
                    record.lock().unwrap().calls.last(),
                    Some(&Call::Submit(XboxReport::NEUTRAL))
                );
                owner.step(
                    request,
                    Some(held),
                    start + virtual_gamepad_agent::input::SESSION_IDLE_TIMEOUT,
                );
                let mut fresh = held;
                fresh.state.sequence = 2;
                fresh.received_at = start + INPUT_TIMEOUT;
                owner.step(request, Some(fresh), fresh.received_at);
                assert!(owner.health.timed_out);
            }
            1 => owner.step(None, None, start),
            2 => {
                let newer = AcceptedState {
                    state: state(session, 2, 1, -1, true),
                    received_at: start,
                };
                record
                    .lock()
                    .unwrap()
                    .failures
                    .push(Call::Submit(xbox_report(&newer.state)));
                owner.step(request, Some(newer), start);
                assert!(owner.health.fault);
            }
            _ => owner.shutdown().unwrap(),
        }
        assert!(
            record
                .lock()
                .unwrap()
                .calls
                .ends_with(&[Call::Neutral, Call::Detach])
        );
        let count = record.lock().unwrap().calls.len();
        owner.shutdown().unwrap();
        assert_eq!(record.lock().unwrap().calls.len(), count);
    }
}
