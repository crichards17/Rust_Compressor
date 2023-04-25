use uuid::Uuid;

use crate::{AllocatorError, LocalId, StableId};

#[derive(Eq, PartialEq, PartialOrd, Ord, Hash, Copy, Clone, Debug)]
/// A StableId which is suitable for use as a session identifier.
/// Uniquely identifies a compressor within a network.
pub struct SessionId {
    id: StableId,
}

impl SessionId {
    /// Returns the SessionId representation of the nil UUID.
    pub fn nil() -> SessionId {
        SessionId {
            id: StableId::nil(),
        }
    }

    #[cfg(feature = "uuid-generation")]
    /// Generates a new SessionId from a random UUID.
    pub fn new() -> SessionId {
        SessionId {
            id: StableId::from(Uuid::new_v4()),
        }
    }

    /// Creates a new SessionId from the supplied UUID. Intended for internal use only.
    pub fn from_uuid_string(uuid_string: &str) -> Result<SessionId, AllocatorError> {
        let bytes = uuid_string.as_bytes();
        if bytes.len() != 36
            || bytes[8] != b'-'
            || bytes[13] != b'-'
            || bytes[18] != b'-'
            || bytes[23] != b'-'
        {
            return Err(AllocatorError::InvalidUuidString);
        }
        let mut buff = [b'0'; 16];
        fill_bytes(&bytes[0..8], &mut buff[0..4])?;
        fill_bytes(&bytes[9..13], &mut buff[4..6])?;
        fill_bytes(&bytes[14..18], &mut buff[6..8])?;
        fill_bytes(&bytes[19..23], &mut buff[8..10])?;
        fill_bytes(&bytes[24..36], &mut buff[10..16])?;

        let uuid = Uuid::from_bytes(buff);
        if uuid.get_variant() != uuid::Variant::RFC4122 || uuid.get_version_num() != 4 {
            Err(AllocatorError::InvalidVersionOrVariant)
        } else {
            Ok(SessionId {
                id: StableId::from(uuid),
            })
        }
    }

    /// Creates a new SessionId from the supplied UUID in bit form. Intended for internal use only.
    pub fn from_uuid_u128(uuid_u128: u128) -> SessionId {
        uuid::Builder::from_u128(uuid_u128).into_uuid().into()
    }
}

fn fill_bytes(chars: &[u8], buff: &mut [u8]) -> Result<(), AllocatorError> {
    for i in 0..buff.len() {
        let l = to_byte(chars[i * 2]);
        let h = to_byte(chars[i * 2 + 1]);
        if h == 0xFF || l == 0xFF {
            return Err(AllocatorError::InvalidUuidString);
        }
        buff[i] = l << 4 | h;
    }
    Ok(())
}

fn to_byte(byte: u8) -> u8 {
    match byte {
        b'0' => 0b0000_u8,
        b'1' => 0b0001_u8,
        b'2' => 0b0010_u8,
        b'3' => 0b0011_u8,
        b'4' => 0b0100_u8,
        b'5' => 0b0101_u8,
        b'6' => 0b0110_u8,
        b'7' => 0b0111_u8,
        b'8' => 0b1000_u8,
        b'9' => 0b1001_u8,
        b'A' => 0b1010_u8,
        b'a' => 0b1010_u8,
        b'B' => 0b1011_u8,
        b'b' => 0b1011_u8,
        b'C' => 0b1100_u8,
        b'c' => 0b1100_u8,
        b'D' => 0b1101_u8,
        b'd' => 0b1101_u8,
        b'E' => 0b1110_u8,
        b'e' => 0b1110_u8,
        b'F' => 0b1111_u8,
        b'f' => 0b1111_u8,
        _ => 0xFF_u8,
    }
}

impl From<Uuid> for SessionId {
    fn from(value: Uuid) -> Self {
        SessionId { id: value.into() }
    }
}

impl From<SessionId> for String {
    fn from(value: SessionId) -> Self {
        value.id.into()
    }
}

impl From<SessionId> for Uuid {
    fn from(value: SessionId) -> Self {
        value.id.into()
    }
}

impl From<SessionId> for StableId {
    fn from(value: SessionId) -> Self {
        value.id
    }
}

/// Internal type conversion
pub fn from_stable_id(stable_id: StableId) -> SessionId {
    SessionId { id: stable_id }
}

impl std::ops::Add<LocalId> for SessionId {
    type Output = StableId;
    fn add(self, rhs: LocalId) -> Self::Output {
        self.id + rhs
    }
}
