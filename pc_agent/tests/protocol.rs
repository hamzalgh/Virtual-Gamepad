use std::{collections::BTreeMap, fs, path::PathBuf, str::FromStr};
use virtual_gamepad_agent::{protocol::*, session::is_newer};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../protocol/fixtures/v1")
}
fn fixture(name: &str) -> Vec<u8> {
    fs::read(root().join(format!("{name}.bin"))).unwrap()
}
fn seal(b: &mut [u8]) {
    let crc = crc32fast::hash(&b[..36]);
    b[36..40].copy_from_slice(&crc.to_le_bytes());
}
trait Field: Sized {
    fn field(s: &str) -> Self;
}
macro_rules! numbers { ($($t:ty),*) => {$(impl Field for $t { fn field(s: &str) -> Self { Self::from_str(s).unwrap() } })*}; }
numbers!(u8, u16, u32, u64, i16);
macro_rules! ids { ($($t:ident),*) => {$(impl Field for $t { fn field(s: &str) -> Self { Self(u64::field(s).try_into().unwrap()) } })*}; }
ids!(Nonce, InstanceId, SessionId);
impl Field for Option<SessionId> {
    fn field(s: &str) -> Self {
        let n = u64::field(s);
        std::num::NonZeroU64::new(n).map(SessionId)
    }
}
impl Field for RejectReason {
    fn field(s: &str) -> Self {
        [
            Self::Unspecified,
            Self::WindowClosed,
            Self::IncorrectCode,
            Self::NoSlot,
            Self::Revoked,
            Self::RateLimited,
            Self::BackendNotReady,
        ][usize::from_str(s).unwrap()]
    }
}
impl Field for DisconnectReason {
    fn field(s: &str) -> Self {
        [
            Self::UserLeft,
            Self::Lifecycle,
            Self::NetworkLost,
            Self::LocalError,
            Self::Shutdown,
        ][usize::from_str(s).unwrap()]
    }
}
fn get<T: Field>(fields: &mut BTreeMap<&str, &str>, key: &str) -> T {
    T::field(fields.remove(key).unwrap())
}
// Field names here are independent of the codec offset macro. Every manifest value
// must be consumed, so typos and undocumented additions cannot silently pass.
fn expected(kind: &str, text: &str) -> Message {
    let mut f: BTreeMap<_, _> = text
        .split_whitespace()
        .map(|kv| kv.split_once('=').unwrap())
        .collect();
    macro_rules! payload { ($name:ident, $($field:ident),*) => { Message::$name($name { $($field: get(&mut f, stringify!($field))),* }) }; }
    let message = match kind {
        "Hello" => payload!(
            Hello,
            client_nonce,
            client_instance,
            state_rate,
            capabilities,
            app_build,
            api_level
        ),
        "HelloAck" => payload!(
            HelloAck,
            client_nonce,
            server_nonce,
            session,
            state_rate,
            timeout_ms,
            flags,
            capacity,
            slot
        ),
        "PairRequest" => payload!(
            PairRequest,
            client_nonce,
            server_nonce,
            client_instance,
            code,
            slot
        ),
        "PairAccept" => payload!(
            PairAccept,
            session,
            client_nonce,
            server_instance,
            state_rate,
            timeout_ms,
            slot,
            capabilities
        ),
        "PairReject" => payload!(PairReject, client_nonce, reason, retry_after),
        "ControllerState" => payload!(
            ControllerState,
            session,
            sequence,
            client_ms,
            buttons,
            left_x,
            left_y,
            right_x,
            right_y,
            left_trigger,
            right_trigger
        ),
        "Status" => payload!(
            Status,
            session,
            last_sequence,
            client_ms,
            accepted,
            rejected,
            lost,
            observed_rate,
            flags,
            timeout_ms
        ),
        "Disconnect" => payload!(Disconnect, session, reason),
        _ => panic!("unknown fixture type {kind}"),
    };
    assert!(f.is_empty());
    message
}
fn rejection(s: &str) -> DecodeError {
    match s {
        "Length" => DecodeError::Length,
        "Magic" => DecodeError::Magic,
        "Version" => DecodeError::Version,
        "MessageType" => DecodeError::MessageType,
        "Crc" => DecodeError::Crc,
        "Reserved" => DecodeError::Reserved,
        "Range" => DecodeError::Range,
        "DpadConflict" => DecodeError::DpadConflict,
        _ => panic!("unknown rejection {s}"),
    }
}
#[test]
fn shared_manifest_decodes_and_encodes_byte_exactly() {
    let manifest = fs::read_to_string(root().join("manifest.txt")).unwrap();
    let mut count = 0;
    for line in manifest.lines().filter(|l| !l.starts_with('#')) {
        let columns: Vec<_> = line.split('|').collect();
        assert_eq!(columns.len(), 4);
        let bytes = fs::read(root().join(columns[0])).unwrap();
        if columns[1] == "Valid" {
            let m = expected(columns[2], columns[3]);
            assert_eq!(decode(&bytes), Ok(m), "{}", columns[0]);
            assert_eq!(m.encode().unwrap().as_slice(), bytes, "{}", columns[0]);
        } else {
            assert_eq!(decode(&bytes), Err(rejection(columns[1])), "{}", columns[0]);
        }
        count += 1;
    }
    let binaries = fs::read_dir(root())
        .unwrap()
        .filter(|e| {
            e.as_ref()
                .unwrap()
                .path()
                .extension()
                .is_some_and(|x| x == "bin")
        })
        .count();
    assert_eq!(count, binaries, "every binary must be listed");
    assert!(count >= 50);
}
#[test]
fn crc_canonical_and_every_single_byte_corruption() {
    assert_eq!(crc32fast::hash(b"123456789"), 0xcbf43926);
    let bytes = fixture("mixed");
    for offset in 0..40 {
        let mut bad = bytes.clone();
        bad[offset] ^= 0x80;
        let error = match offset {
            0 | 1 => DecodeError::Magic,
            2 => DecodeError::Version,
            3 => DecodeError::MessageType,
            _ => DecodeError::Crc,
        };
        assert_eq!(decode(&bad), Err(error), "offset {offset}");
    }
}
#[test]
fn every_reserved_byte_and_bit_is_rejected() {
    for (name, offsets) in [
        ("hello", 30..36),
        ("pair_request", 33..36),
        ("pair_accept", 34..36),
        ("pair_reject_0", 14..36),
        ("neutral", 34..36),
        ("disconnect_0", 13..36),
    ] {
        for o in offsets {
            for bit in 0..8 {
                let mut b = fixture(name);
                b[o] = 1 << bit;
                seal(&mut b);
                assert_eq!(decode(&b), Err(DecodeError::Reserved), "{name}:{o}:{bit}");
            }
        }
    }
    for (name, offset, start, end) in [
        ("hello", 22, 3, 16),
        ("hello_ack_pending", 32, 4, 16),
        ("pair_accept", 33, 3, 8),
        ("neutral", 20, 14, 32),
        ("status", 32, 6, 16),
    ] {
        for bit in start..end {
            let mut b = fixture(name);
            b[offset + bit / 8] |= 1 << (bit % 8);
            seal(&mut b);
            assert_eq!(decode(&b), Err(DecodeError::Reserved));
        }
    }
}
#[test]
fn nonzero_identifiers_and_payload_ranges() {
    for (name, offsets) in [
        ("hello", vec![4, 12]),
        ("hello_ack_pending", vec![4, 12]),
        ("pair_request", vec![4, 12, 20]),
        ("pair_accept", vec![4, 12, 20]),
        ("neutral", vec![4]),
        ("disconnect_0", vec![4]),
    ] {
        for o in offsets {
            let mut b = fixture(name);
            b[o..o + 8].fill(0);
            seal(&mut b);
            assert_eq!(decode(&b), Err(DecodeError::Range), "{name}:{o}");
        }
    }
    for (name, offset) in [
        ("hello_ack_pending", 30),
        ("pair_accept", 30),
        ("status", 34),
    ] {
        for value in [0u16, 99, 1001, u16::MAX] {
            let mut b = fixture(name);
            b[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
            seal(&mut b);
            assert_eq!(decode(&b), Err(DecodeError::Range));
        }
    }
    for value in [0u8, 5, 255] {
        let mut b = fixture("hello_ack_pending");
        b[34] = value;
        seal(&mut b);
        assert_eq!(decode(&b), Err(DecodeError::Range));
    }
    for (name, offset, value) in [("hello_ack_session", 35, 4), ("pair_accept", 32, 1)] {
        let mut b = fixture(name);
        b[offset] = value;
        seal(&mut b);
        assert_eq!(decode(&b), Err(DecodeError::Range));
    }
}
#[test]
fn all_button_bits_and_dpad_combinations() {
    let Message::ControllerState(mut state) = decode(&fixture("neutral")).unwrap() else {
        panic!()
    };
    for bit in 0..32 {
        state.buttons = 1 << bit;
        assert_eq!(Message::ControllerState(state).encode().is_ok(), bit < 14);
    }
    for dpad in 0..16 {
        state.buttons = dpad << 10;
        let valid = dpad & 3 != 3 && dpad & 12 != 12;
        assert_eq!(Message::ControllerState(state).encode().is_ok(), valid);
    }
}
#[test]
fn invalid_caller_values_are_not_emitted() {
    let Message::Hello(mut h) = decode(&fixture("hello")).unwrap() else {
        panic!()
    };
    h.capabilities = 8;
    assert_eq!(Message::Hello(h).encode(), Err(DecodeError::Reserved));
    let Message::PairRequest(mut p) = decode(&fixture("pair_request")).unwrap() else {
        panic!()
    };
    p.code = 1000000;
    assert_eq!(Message::PairRequest(p).encode(), Err(DecodeError::Range));
}
#[test]
fn exact_sequence_half_range_rule() {
    for (a, b, yes) in [
        (1, 0, true),
        (0, 0, false),
        (2, 1, true),
        (1, 2, false),
        (0, u32::MAX, true),
        (u32::MAX, 0, false),
        (1, u32::MAX, true),
        (1 << 31, 0, false),
        (0, 1 << 31, false),
        ((1 << 31) - 1, 0, true),
        ((1 << 31) + 1, 0, false),
    ] {
        assert_eq!(is_newer(a, b), yes, "{a} after {b}");
    }
}
#[test]
fn bounded_arbitrary_inputs_never_panic() {
    let mut rng = 0x1234_5678u32;
    for n in 0..20000 {
        let len = if n % 2 == 0 { 40 } else { n % 1025 };
        let mut b = vec![0; len];
        for byte in &mut b {
            rng ^= rng << 13;
            rng ^= rng >> 17;
            rng ^= rng << 5;
            *byte = rng as u8;
        }
        if len == 40 && n % 3 == 0 {
            b[..3].copy_from_slice(&[86, 71, 1]);
            b[3] = [1, 2, 3, 4, 5, 16, 17, 18][n % 8];
            seal(&mut b);
        }
        if let Ok(m) = decode(&b) {
            assert_eq!(m.encode().unwrap().as_slice(), b);
        }
    }
}
