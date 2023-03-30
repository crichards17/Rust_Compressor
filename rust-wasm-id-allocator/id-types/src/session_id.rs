use crate::LocalId;

use super::StableId;

#[derive(Eq, PartialEq, Hash, Copy, Clone, Debug)]
pub struct SessionId {
    id: StableId,
}

impl From<StableId> for SessionId {
    fn from(value: StableId) -> Self {
        SessionId { id: value }
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
