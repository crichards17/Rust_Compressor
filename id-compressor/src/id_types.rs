use std::ops::Sub;

use uuid::Uuid;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct SessionSpaceId {
    pub(crate) id: i64,
}

impl SessionSpaceId {
    pub(crate) fn to_space(&self) -> CompressedId {
        if self.is_local() {
            return CompressedId::Local(LocalId { id: self.id });
        } else {
            CompressedId::Final(FinalId { id: self.id as u64 })
        }
    }

    pub(crate) fn is_local(&self) -> bool {
        self.id < 0
    }

    pub(crate) fn is_final(&self) -> bool {
        self.id >= 0
    }
}

impl From<LocalId> for SessionSpaceId {
    fn from(value: LocalId) -> Self {
        SessionSpaceId { id: value.id }
    }
}

impl From<FinalId> for SessionSpaceId {
    fn from(value: FinalId) -> Self {
        SessionSpaceId {
            id: value.id as i64,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct OpSpaceId {
    pub(crate) id: i64,
}

impl OpSpaceId {
    pub(crate) fn to_space(&self) -> CompressedId {
        if self.is_local() {
            return CompressedId::Local(LocalId { id: self.id });
        } else {
            CompressedId::Final(FinalId { id: self.id as u64 })
        }
    }

    pub(crate) fn is_local(&self) -> bool {
        self.id < 0
    }

    pub(crate) fn is_final(&self) -> bool {
        self.id >= 0
    }
}

impl From<FinalId> for OpSpaceId {
    fn from(value: FinalId) -> Self {
        OpSpaceId {
            id: value.id as i64,
        }
    }
}

impl From<LocalId> for OpSpaceId {
    fn from(value: LocalId) -> Self {
        OpSpaceId { id: value.id() }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct LocalId {
    id: i64,
}

impl LocalId {
    pub fn new(id: i64) -> LocalId {
        debug_assert!(
            id < 0,
            "Local ID must be negative. Passed value was {}.",
            id,
        );
        LocalId { id }
    }

    pub fn id(&self) -> i64 {
        self.id
    }

    pub fn to_generation_count(&self) -> u64 {
        (-self.id) as u64
    }

    pub fn from_generation_count(generation_count: u64) -> Self {
        LocalId::new(-(generation_count as i64))
    }
}

impl PartialEq<i64> for LocalId {
    fn eq(&self, other: &i64) -> bool {
        self.id == *other
    }
}

impl PartialOrd<i64> for LocalId {
    fn ge(&self, other: &i64) -> bool {
        self.id >= *other
    }

    fn gt(&self, other: &i64) -> bool {
        self.id > *other
    }

    fn le(&self, other: &i64) -> bool {
        self.id <= *other
    }

    fn lt(&self, other: &i64) -> bool {
        self.id < *other
    }

    fn partial_cmp(&self, other: &i64) -> Option<std::cmp::Ordering> {
        if self.le(other) {
            return Some(std::cmp::Ordering::Less);
        } else if self.gt(other) {
            return Some(std::cmp::Ordering::Greater);
        } else {
            Some(std::cmp::Ordering::Equal)
        }
    }
}

impl Sub<u64> for LocalId {
    type Output = LocalId;
    fn sub(self, rhs: u64) -> Self::Output {
        LocalId::new(self.id - rhs as i64)
    }
}

#[derive(Clone, Copy, PartialEq, PartialOrd, Eq, Ord, Debug)]
pub struct FinalId {
    pub(crate) id: u64,
}

impl std::ops::Add<u64> for FinalId {
    type Output = FinalId;
    fn add(self, rhs: u64) -> Self::Output {
        FinalId { id: self.id + rhs }
    }
}

impl std::ops::AddAssign<u64> for FinalId {
    fn add_assign(&mut self, rhs: u64) {
        self.id += rhs;
    }
}

impl Sub<FinalId> for FinalId {
    type Output = i64;
    fn sub(self, rhs: FinalId) -> Self::Output {
        debug_assert!(self.id >= rhs.id, "Final ID subtraction overflow");
        self.id as i64 - rhs.id as i64
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct StableId {
    pub(crate) id: u128,
}

impl StableId {
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
        StableId { id: value.id }
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

#[derive(Eq, PartialEq, Hash, Copy, Clone)]
pub struct SessionId {
    id: u128,
}

// todo: UUID math
impl std::ops::Add<LocalId> for SessionId {
    type Output = StableId;
    fn add(self, rhs: LocalId) -> Self::Output {
        let abs_local = (rhs.to_generation_count() - 1) as u128;
        let new_id = self.id + abs_local;
        StableId { id: new_id }
    }
}

impl SessionId {
    pub(crate) fn new() -> SessionId {
        SessionId {
            id: Uuid::new_v4().as_u128(),
        }
    }

    pub(crate) fn id(&self) -> u128 {
        self.id
    }
}

pub enum CompressedId {
    Local(LocalId),
    Final(FinalId),
}
