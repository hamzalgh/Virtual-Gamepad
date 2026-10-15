//! Spec 0007 mapping and full report safety, with the unchanged shared wire fixture.
use std::{
    num::NonZeroU64,
    sync::{Arc, Mutex},
};
use tokio::time::Instant;
use virtual_gamepad_agent::{
    input::{
        AcceptedState, INPUT_TIMEOUT, OutputOwner, SessionRequest, supported_controls, xbox_report,
    },
    protocol::{ControllerState, Message, SessionId, decode},
    virtual_gamepad::{
        XboxReport,
        fake::{Call, RecordingBackend},
    },
};

fn neutral() -> ControllerState {
    ControllerState {
        session: SessionId(NonZeroU64::new(77).unwrap()),
        sequence: 1,
        client_ms: 0,
        buttons: 0,
        left_x: 0,
        left_y: 0,
        right_x: 0,
        right_y: 0,
        left_trigger: 0,
        right_trigger: 0,
    }
}

#[test]
fn short_gap_neutralizes_once_recovers_without_detach_then_hard_expiry_closes() {
    let record = Arc::new(Mutex::new(RecordingBackend::default()));
    let mut owner = OutputOwner::new(record.clone());
    let start = Instant::now();
    let state = ControllerState {
        buttons: 1,
        right_x: 12345,
        left_trigger: 128,
        ..neutral()
    };
    let request = Some(SessionRequest {
        session: state.session,
        expires_at: start + INPUT_TIMEOUT,
        armed: true,
    });
    let held = AcceptedState {
        state,
        received_at: start,
    };
    owner.step(request, Some(held), start);
    owner.step(request, Some(held), start + INPUT_TIMEOUT);
    assert!(owner.health.ready && owner.health.attached && owner.health.timed_out);
    assert_eq!(
        record.lock().unwrap().calls.last(),
        Some(&Call::Submit(XboxReport::NEUTRAL))
    );
    let count = record.lock().unwrap().calls.len();
    owner.step(
        request,
        Some(held),
        start + std::time::Duration::from_millis(290),
    );
    assert_eq!(record.lock().unwrap().calls.len(), count);
    let resumed = AcceptedState {
        state: ControllerState {
            sequence: 2,
            ..state
        },
        received_at: start + std::time::Duration::from_millis(300),
    };
    owner.step(request, Some(resumed), resumed.received_at);
    assert!(!owner.health.timed_out);
    assert_eq!(
        record.lock().unwrap().calls.last(),
        Some(&Call::Submit(xbox_report(&state)))
    );
    owner.step(request, Some(resumed), resumed.received_at + INPUT_TIMEOUT);
    owner.step(
        request,
        Some(resumed),
        resumed.received_at + std::time::Duration::from_secs(3),
    );
    assert!(!owner.health.ready && !owner.health.attached);
    let count = record.lock().unwrap().calls.len();
    owner.step(
        request,
        Some(AcceptedState {
            received_at: resumed.received_at + std::time::Duration::from_secs(3),
            ..resumed
        }),
        resumed.received_at + std::time::Duration::from_secs(3),
    );
    assert_eq!(record.lock().unwrap().calls.len(), count);
}

#[test]
fn every_named_button_has_the_xinput_bit_and_no_guide_or_reserved_output() {
    let named = [
        ("A", 0, 0x1000),
        ("B", 1, 0x2000),
        ("X", 2, 0x4000),
        ("Y", 3, 0x8000),
        ("LB", 4, 0x100),
        ("RB", 5, 0x200),
        ("Back", 6, 0x20),
        ("Start", 7, 0x10),
        ("L3", 8, 0x40),
        ("R3", 9, 0x80),
        ("Up", 10, 1),
        ("Down", 11, 2),
        ("Left", 12, 4),
        ("Right", 13, 8),
    ];
    for (name, bit, xbox) in named {
        let state = ControllerState {
            buttons: 1 << bit,
            ..neutral()
        };
        assert!(supported_controls(&state), "{name}");
        let bytes = Message::ControllerState(state).encode().unwrap();
        assert_eq!(decode(&bytes).unwrap(), Message::ControllerState(state));
        assert_eq!(
            xbox_report(&state),
            XboxReport {
                buttons: xbox,
                ..XboxReport::NEUTRAL
            },
            "{name}"
        );
    }
    for bits in 0..=0x3fff {
        let state = ControllerState {
            buttons: bits,
            ..neutral()
        };
        assert_eq!(
            supported_controls(&state),
            bits & 0xc00 != 0xc00 && bits & 0x3000 != 0x3000
        );
        if supported_controls(&state) {
            assert_eq!(xbox_report(&state).buttons & !0xf3ff, 0);
        }
    }
    for bits in [0x4000, 0x8000, 0x80000000, 0xc00, 0x3000] {
        let state = ControllerState {
            buttons: bits,
            ..neutral()
        };
        assert!(!supported_controls(&state));
        assert!(Message::ControllerState(state).encode().is_err());
    }
}

#[test]
fn failed_soft_timeout_neutral_detaches_and_refuses_recovery() {
    let record = Arc::new(Mutex::new(RecordingBackend::default()));
    let mut owner = OutputOwner::new(record.clone());
    let start = Instant::now();
    let state = ControllerState {
        buttons: 1,
        right_trigger: 255,
        ..neutral()
    };
    let request = Some(SessionRequest {
        session: state.session,
        expires_at: start + INPUT_TIMEOUT,
        armed: true,
    });
    let held = AcceptedState {
        state,
        received_at: start,
    };
    owner.step(request, Some(held), start);
    record
        .lock()
        .unwrap()
        .failures
        .push(Call::Submit(XboxReport::NEUTRAL));
    owner.step(request, Some(held), start + INPUT_TIMEOUT);
    assert!(owner.health.fault && !owner.health.ready && !owner.health.attached);
    assert!(
        record
            .lock()
            .unwrap()
            .calls
            .ends_with(&[Call::Neutral, Call::Detach])
    );
    let count = record.lock().unwrap().calls.len();
    owner.step(
        request,
        Some(AcceptedState {
            state: ControllerState {
                sequence: 2,
                ..state
            },
            received_at: start + INPUT_TIMEOUT,
        }),
        start + INPUT_TIMEOUT,
    );
    assert_eq!(record.lock().unwrap().calls.len(), count);
}

#[test]
fn all_axes_and_triggers_copy_independently_without_quantization_or_second_y_flip() {
    for axis in 0..4 {
        for value in [i16::MIN, -23170, 0, 23170, i16::MAX] {
            let mut state = neutral();
            match axis {
                0 => state.left_x = value,
                1 => state.left_y = value,
                2 => state.right_x = value,
                _ => state.right_y = value,
            }
            let mapped = xbox_report(&state);
            let axes = [mapped.left_x, mapped.left_y, mapped.right_x, mapped.right_y];
            for (index, actual) in axes.into_iter().enumerate() {
                assert_eq!(actual, if index == axis { value } else { 0 });
            }
        }
    }
    for left in [0, 128, 255] {
        for right in [0, 128, 255] {
            let state = ControllerState {
                left_trigger: left,
                right_trigger: right,
                ..neutral()
            };
            assert_eq!(
                xbox_report(&state),
                XboxReport {
                    left_trigger: left,
                    right_trigger: right,
                    ..XboxReport::NEUTRAL
                }
            );
        }
    }
    assert_eq!(xbox_report(&neutral()), XboxReport::NEUTRAL);
}

#[test]
fn shared_mixed_fixture_maps_every_field_and_reencodes_identically() {
    let bytes = include_bytes!("../../protocol/fixtures/v1/mixed.bin");
    let Message::ControllerState(state) = decode(bytes).unwrap() else {
        panic!()
    };
    assert_eq!(&Message::ControllerState(state).encode().unwrap(), bytes);
    assert_eq!(
        xbox_report(&state),
        XboxReport {
            buttons: 0x5209,
            left_x: -12345,
            left_y: 23456,
            right_x: 13579,
            right_y: -24680,
            left_trigger: 85,
            right_trigger: 170,
        }
    );
}

#[test]
fn complex_report_neutralizes_on_default_deadline_revocation_fault_and_shutdown() {
    for reason in 0..4 {
        let recording = Arc::new(Mutex::new(RecordingBackend::default()));
        let mut owner = OutputOwner::new(recording.clone());
        let start = Instant::now();
        let state = ControllerState {
            buttons: 0x27ff,
            left_x: i16::MIN,
            left_y: i16::MAX,
            right_x: i16::MAX,
            right_y: i16::MIN,
            left_trigger: 128,
            right_trigger: 255,
            ..neutral()
        };
        let request = Some(SessionRequest {
            session: state.session,
            expires_at: start + INPUT_TIMEOUT,
            armed: true,
        });
        let held = AcceptedState {
            state,
            received_at: start,
        };
        owner.step(request, Some(held), start);
        assert_eq!(
            recording.lock().unwrap().calls.last(),
            Some(&Call::Submit(xbox_report(&state)))
        );
        match reason {
            0 => {
                owner.step(
                    request,
                    Some(held),
                    start + INPUT_TIMEOUT - std::time::Duration::from_millis(1),
                );
                assert!(!owner.health.timed_out);
                owner.step(request, Some(held), start + INPUT_TIMEOUT);
                assert!(owner.health.timed_out);
                assert!(owner.health.attached);
                assert_eq!(
                    recording.lock().unwrap().calls.last(),
                    Some(&Call::Submit(XboxReport::NEUTRAL))
                );
                owner.step(
                    request,
                    Some(held),
                    start + virtual_gamepad_agent::input::SESSION_IDLE_TIMEOUT,
                );
                let later = AcceptedState {
                    state: ControllerState {
                        sequence: 2,
                        ..state
                    },
                    received_at: start + INPUT_TIMEOUT,
                };
                owner.step(request, Some(later), later.received_at);
                assert!(!owner.health.attached);
            }
            1 => owner.step(None, None, start),
            2 => {
                let next = ControllerState {
                    right_trigger: 127,
                    sequence: 2,
                    ..state
                };
                recording
                    .lock()
                    .unwrap()
                    .failures
                    .push(Call::Submit(xbox_report(&next)));
                owner.step(
                    request,
                    Some(AcceptedState {
                        state: next,
                        received_at: start,
                    }),
                    start,
                );
                assert!(owner.health.fault);
            }
            _ => owner.shutdown().unwrap(),
        }
        assert!(
            recording
                .lock()
                .unwrap()
                .calls
                .ends_with(&[Call::Neutral, Call::Detach])
        );
        let count = recording.lock().unwrap().calls.len();
        owner.shutdown().unwrap();
        assert_eq!(recording.lock().unwrap().calls.len(), count);
    }
}
