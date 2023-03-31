use thiserror::Error;
use uuid::Uuid;

use super::StableId;
use crate::LocalId;

#[derive(Eq, PartialEq, Hash, Copy, Clone, Debug)]
pub struct SessionId {
    id: StableId,
}

impl SessionId {
    #[cfg(feature = "uuid-generation")]
    pub fn new() -> SessionId {
        SessionId {
            id: StableId::from(Uuid::new_v4()),
        }
    }

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

    pub fn from_uuid_u128(uuid_u128: u128) -> SessionId {
        uuid::Builder::from_u128(uuid_u128).into_uuid().into()
    }

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
pub enum UuidGenerationError {
    #[error("Invalid Uuid String")]
    InvalidUuidString,
    #[error("Invalid Version or Variant")]
    InvalidVersionOrVariant,
}
