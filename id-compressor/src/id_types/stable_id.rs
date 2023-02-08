use super::SessionId;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct StableId {
    id: u128,
}

// TODO: to_string uuid math
// TODO: reimplement arithmetic (internal IDs are safe)
impl StableId {
    pub(super) fn id(&self) -> u128 {
        self.id
    }

    pub(super) fn new(id: u128) -> Self {
        StableId { id }
    }

    pub(crate) fn null() -> StableId {
        StableId { id: 0 }
    }

    pub(crate) fn offset_by(&self, offset: i64) -> StableId {
        StableId {
            id: self.id + offset as u128,
        }
    }

    pub(crate) fn sub(self, other: Self) -> u128 {
        (self.id - other.id) as u128
    }

    // TODO: to_uuid_string() to reverse transform
}

impl From<SessionId> for StableId {
    fn from(value: SessionId) -> Self {
        StableId { id: value.id() }
    }
}
