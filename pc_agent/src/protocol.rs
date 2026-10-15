//! Pure, strict codecs for the normative 40 byte v1 wire format.
use std::{fmt, num::NonZeroU64};

pub const PACKET_LEN: usize = 40;
pub const MAGIC: [u8; 2] = *b"VG";
pub const VERSION: u8 = 1;
pub const BUTTON_MASK: u32 = 0x3fff;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecodeError {
    Length,
    Magic,
    Version,
    MessageType,
    Crc,
    Reserved,
    Range,
    DpadConflict,
}
impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for DecodeError {}

// Offsets are internal constants. Slice length is checked before any field access.
trait Wire: Sized {
    fn read(bytes: &[u8], offset: usize) -> Result<Self, DecodeError>;
    fn write(&self, bytes: &mut [u8], offset: usize);
}
macro_rules! integer {
    ($($t:ty),*) => {$(impl Wire for $t {
        fn read(b: &[u8], o: usize) -> Result<Self, DecodeError> {
            let mut a = [0; size_of::<Self>()];
            a.copy_from_slice(&b[o..o + size_of::<Self>()]);
            Ok(Self::from_le_bytes(a))
        }
        fn write(&self, b: &mut [u8], o: usize) {
            b[o..o + size_of::<Self>()].copy_from_slice(&self.to_le_bytes());
        }
    })*};
}
integer!(u8, u16, u32, u64, i16);

macro_rules! identifier {
    ($($name:ident),*) => {$(
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub struct $name(pub NonZeroU64);
        impl Wire for $name {
            fn read(b: &[u8], o: usize) -> Result<Self, DecodeError> {
                NonZeroU64::new(u64::read(b, o)?).map(Self).ok_or(DecodeError::Range)
            }
            fn write(&self, b: &mut [u8], o: usize) { self.0.get().write(b, o); }
        }
    )*};
}
identifier!(Nonce, InstanceId, SessionId);
impl Wire for Option<SessionId> {
    fn read(b: &[u8], o: usize) -> Result<Self, DecodeError> {
        Ok(NonZeroU64::new(u64::read(b, o)?).map(SessionId))
    }
    fn write(&self, b: &mut [u8], o: usize) {
        self.map_or(0, |id| id.0.get()).write(b, o);
    }
}

macro_rules! registry {
    ($name:ident { $($variant:ident = $value:literal),* $(,)? }) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        #[repr(u8)]
        pub enum $name { $($variant = $value),* }
        impl Wire for $name {
            fn read(b: &[u8], o: usize) -> Result<Self, DecodeError> {
                match b[o] { $($value => Ok(Self::$variant),)* _ => Err(DecodeError::Range) }
            }
            fn write(&self, b: &mut [u8], o: usize) { b[o] = *self as u8; }
        }
    };
}
registry!(RejectReason { Unspecified = 0, WindowClosed = 1, IncorrectCode = 2,
    NoSlot = 3, Revoked = 4, RateLimited = 5, BackendNotReady = 6 });
registry!(DisconnectReason { UserLeft = 0, Lifecycle = 1, NetworkLost = 2,
    LocalError = 3, Shutdown = 4 });

// One visible field/offset list drives both directions. Reserved storage is never exposed.
macro_rules! messages {
    ($($name:ident = $tag:literal { $($field:ident: $ty:ty = $offset:literal),* $(,)? }),* $(,)?) => {
        $(#[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub struct $name { $(pub $field: $ty),* })*
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub enum Message { $($name($name)),* }
        impl Message {
            pub fn message_type(&self) -> u8 { match self { $(Self::$name(_) => $tag),* } }
            fn raw_encode(&self) -> [u8; PACKET_LEN] {
                let mut b = [0; PACKET_LEN];
                b[..2].copy_from_slice(&MAGIC); b[2] = VERSION; b[3] = self.message_type();
                match self { $(Self::$name(p) => { $(p.$field.write(&mut b, $offset);)* }),* }
                crc32fast::hash(&b[..36]).write(&mut b, 36);
                b
            }
            /// Invalid caller constructed payloads are refused, never truncated or masked.
            pub fn encode(&self) -> Result<[u8; PACKET_LEN], DecodeError> {
                self.validate()?;
                Ok(self.raw_encode())
            }
        }
        pub fn decode(b: &[u8]) -> Result<Message, DecodeError> {
            if b.len() != PACKET_LEN { return Err(DecodeError::Length); }
            if b[..2] != MAGIC { return Err(DecodeError::Magic); }
            if b[2] != VERSION { return Err(DecodeError::Version); }
            if !matches!(b[3], $($tag)|*) { return Err(DecodeError::MessageType); }
            if crc32fast::hash(&b[..36]) != u32::read(b, 36)? { return Err(DecodeError::Crc); }
            match b[3] {
                1 => reserved(&b[30..36])?, 3 => reserved(&b[33..36])?,
                4 => reserved(&b[34..36])?, 5 => reserved(&b[14..36])?,
                0x10 => reserved(&b[34..36])?, 0x12 => reserved(&b[13..36])?, _ => {}
            }
            let m = match b[3] {
                $($tag => Message::$name($name { $($field: <$ty>::read(b, $offset)?),* }),)*
                _ => return Err(DecodeError::MessageType),
            };
            m.validate()?;
            Ok(m)
        }
    };
}
messages! {
    Hello = 1 { client_nonce: Nonce = 4, client_instance: InstanceId = 12,
        state_rate: u16 = 20, capabilities: u16 = 22, app_build: u32 = 24, api_level: u16 = 28 },
    HelloAck = 2 { client_nonce: Nonce = 4, server_nonce: Nonce = 12,
        session: Option<SessionId> = 20, state_rate: u16 = 28, timeout_ms: u16 = 30,
        flags: u16 = 32, capacity: u8 = 34, slot: u8 = 35 },
    PairRequest = 3 { client_nonce: Nonce = 4, server_nonce: Nonce = 12,
        client_instance: InstanceId = 20, code: u32 = 28, slot: u8 = 32 },
    PairAccept = 4 { session: SessionId = 4, client_nonce: Nonce = 12,
        server_instance: InstanceId = 20, state_rate: u16 = 28, timeout_ms: u16 = 30,
        slot: u8 = 32, capabilities: u8 = 33 },
    PairReject = 5 { client_nonce: u64 = 4, reason: RejectReason = 12, retry_after: u8 = 13 },
    ControllerState = 0x10 { session: SessionId = 4, sequence: u32 = 12,
        client_ms: u32 = 16, buttons: u32 = 20, left_x: i16 = 24, left_y: i16 = 26,
        right_x: i16 = 28, right_y: i16 = 30, left_trigger: u8 = 32, right_trigger: u8 = 33 },
    Status = 0x11 { session: Option<SessionId> = 4, last_sequence: u32 = 12,
        client_ms: u32 = 16, accepted: u32 = 20, rejected: u32 = 24,
        lost: u16 = 28, observed_rate: u16 = 30, flags: u16 = 32, timeout_ms: u16 = 34 },
    Disconnect = 0x12 { session: SessionId = 4, reason: DisconnectReason = 12 },
}
fn reserved(b: &[u8]) -> Result<(), DecodeError> {
    if b.iter().any(|v| *v != 0) {
        Err(DecodeError::Reserved)
    } else {
        Ok(())
    }
}
fn bits(value: u32, mask: u32) -> Result<(), DecodeError> {
    if value & !mask != 0 {
        Err(DecodeError::Reserved)
    } else {
        Ok(())
    }
}
fn range(ok: bool) -> Result<(), DecodeError> {
    if ok { Ok(()) } else { Err(DecodeError::Range) }
}
impl Message {
    fn validate(&self) -> Result<(), DecodeError> {
        match self {
            Self::Hello(p) => bits(p.capabilities.into(), 7),
            Self::HelloAck(p) => {
                bits(p.flags.into(), 15)?;
                range(
                    (1..=4).contains(&p.capacity)
                        && if p.session.is_some() {
                            p.slot < p.capacity
                        } else {
                            p.slot == 255
                        },
                )?;
                range((100..=1000).contains(&p.timeout_ms))
            }
            Self::PairRequest(p) => range(p.code <= 999999 && p.slot == 255),
            Self::PairAccept(p) => {
                bits(p.capabilities.into(), 7)?;
                range(p.slot == 0 && (100..=1000).contains(&p.timeout_ms))
            }
            Self::ControllerState(p) => {
                bits(p.buttons, BUTTON_MASK)?;
                if p.buttons & 0xc00 == 0xc00 || p.buttons & 0x3000 == 0x3000 {
                    Err(DecodeError::DpadConflict)
                } else {
                    Ok(())
                }
            }
            Self::Status(p) => {
                bits(p.flags.into(), 63)?;
                range((100..=1000).contains(&p.timeout_ms))
            }
            Self::PairReject(_) | Self::Disconnect(_) => Ok(()),
        }
    }
}
