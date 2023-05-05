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
        match Uuid::try_parse(uuid_string) {
            Err(_) => Err(AllocatorError::InvalidUuidString),
            Ok(uuid) => {
                if uuid.get_variant() != uuid::Variant::RFC4122 || uuid.get_version_num() != 4 {
                    Err(AllocatorError::InvalidVersionOrVariant)
                } else {
                    Ok(SessionId {
                        id: StableId::from(uuid),
                    })
                }
            }
        }
    }
}

/// Creates a new SessionId from the supplied UUID in number form.
/// Free function to keep it internal.
pub fn session_id_from_uuid_u128(uuid_u128: u128) -> SessionId {
    uuid::Builder::from_u128(uuid_u128).into_uuid().into()
}

/// Creates a new SessionId from the supplied u128 in internal ID form.
/// Free function to keep it internal.
pub fn session_id_from_id_u128(id_u128: u128) -> SessionId {
    SessionId {
        id: StableId { id: id_u128 },
    }
}

/// Internal type conversion
pub fn session_id_from_stable_id(stable_id: StableId) -> SessionId {
    SessionId { id: stable_id }
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

impl From<SessionId> for [u8; 16] {
    fn from(value: SessionId) -> Self {
        value.id.id.to_le_bytes()
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

impl std::ops::Add<LocalId> for SessionId {
    type Output = StableId;
    fn add(self, rhs: LocalId) -> Self::Output {
        self.id + rhs
    }
}
