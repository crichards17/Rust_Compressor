use super::SessionId;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct StableId {
    id: u128,
}

impl StableId {
    pub(super) fn id_raw(&self) -> u128 {
        self.id
    }

    pub(crate) fn new(id: u128) -> Self {
        StableId { id }
    }

    pub(crate) fn null() -> StableId {
        StableId { id: 0 }
    }

    // todo: UUID math
    pub(crate) fn sub_unsafe(self, other: Self) -> u128 {
        (self.id - other.id) as u128
    }
}

impl From<SessionId> for StableId {
    fn from(value: SessionId) -> Self {
        StableId { id: value.id_raw() }
    }
}

// todo: UUID math
impl std::ops::Add<u64> for StableId {
    type Output = StableId;
    fn add(self, rhs: u64) -> Self::Output {
        StableId {
            id: self.id + rhs as u128,
        }
    }
}
