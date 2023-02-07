use super::SessionId;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct StableId {
    id: u128,
}

// TODO: UUID math
impl StableId {
    pub(super) fn id_raw(&self) -> u128 {
        self.id
    }

    pub(super) fn new(id: u128) -> Self {
        StableId { id }
    }

    pub(crate) fn null() -> StableId {
        StableId { id: 0 }
    }

    // TODO: UUID math
    pub(crate) fn offset_by(&self, offset: u64) -> StableId {
        StableId {
            id: self.id + offset as u128,
        }
    }

    // TODO: UUID math
    pub(crate) fn sub_unsafe(self, other: Self) -> u128 {
        (self.id - other.id) as u128
    }
}

impl From<SessionId> for StableId {
    fn from(value: SessionId) -> Self {
        StableId { id: value.id_raw() }
    }
}
