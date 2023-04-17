use thiserror::Error;
use uuid::Uuid;

use super::StableId;
use crate::LocalId;

#[derive(Eq, PartialEq, PartialOrd, Ord, Hash, Copy, Clone, Debug)]
/// A StableId which is suitable for use as a session identifier.
/// Uniquely identifies a compressor within a network.
pub struct SessionId {
    id: StableId,
}

impl SessionId {
    #[cfg(feature = "uuid-generation")]
    /// Generates a new SessionId from a random UUID.
    pub fn new() -> SessionId {
        SessionId {
            id: StableId::from(Uuid::new_v4()),
        }
    }

    /// Creates a new SessionId from the supplied UUID. Intended for internal use only.
    pub fn from_uuid_string(uuid_string: &str) -> Result<SessionId, UuidGenerationError> {
        match Uuid::try_parse(uuid_string) {
            Err(_) => Err(UuidGenerationError::InvalidUuidString),
            Ok(uuid) => {
                if uuid.get_variant() != uuid::Variant::RFC4122 || uuid.get_version_num() != 4 {
                    Err(UuidGenerationError::InvalidVersionOrVariant)
                } else {
                    Ok(SessionId {
                        id: StableId::from(uuid),
                    })
                }
            }
        }
    }

    /// Creates a new SessionId from the supplied UUID in bit form. Intended for internal use only.
    pub fn from_uuid_u128(uuid_u128: u128) -> SessionId {
        uuid::Builder::from_u128(uuid_u128).into_uuid().into()
    }

    /// Returns the SessionId as a hyphenated UUID string.
    pub fn to_uuid_string(&self) -> String {
        Uuid::from(self.id).to_string()
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

impl std::ops::Add<LocalId> for SessionId {
    type Output = StableId;
    fn add(self, rhs: LocalId) -> Self::Output {
        self.id + rhs
    }
}

#[derive(Error, Debug)]
/// Error enum for generating SessionId_s.
pub enum UuidGenerationError {
    #[error("Invalid Uuid String")]
    /// Invalid Uuid String
    InvalidUuidString,
    #[error("Invalid Version or Variant")]
    /// Invalid Version or Variant
    InvalidVersionOrVariant,
}
