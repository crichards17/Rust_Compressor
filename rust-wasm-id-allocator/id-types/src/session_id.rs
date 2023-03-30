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
