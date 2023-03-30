use uuid::Uuid;

use crate::LocalId;

use super::StableId;

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

    pub fn to_uuid_string(&self) -> String {
        Uuid::from(self.id).to_string()
    }
}

impl From<Uuid> for SessionId {
    fn from(value: Uuid) -> Self {
        SessionId { id: value.into() }
    }
}

impl From<SessionId> for Uuid {
    fn from(value: SessionId) -> Self {
        value.id.into()
    }
}

impl std::ops::Add<LocalId> for SessionId {
    type Output = StableId;
    fn add(self, rhs: LocalId) -> Self::Output {
        self.id + rhs
    }
}

#[derive(Debug)]
pub enum UuidGenerationError {
    InvalidUuidString,
    InvalidVersionOrVariant,
}
