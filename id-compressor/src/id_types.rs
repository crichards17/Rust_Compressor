use std::ops::Sub;

use uuid::Uuid;

#[derive(Clone, Copy)]
pub struct SessionSpaceId {
    pub(crate) id: i64,
}

#[derive(Clone, Copy)]
pub struct OpSpaceId {
    pub(crate) id: i64,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
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
        LocalId {
            id: self.id - rhs as i64,
        }
    }
}

#[derive(Clone, Copy, PartialEq, PartialOrd)]
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

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct StableId {
    pub(crate) id: u128,
}

#[derive(Eq, PartialEq, Hash, Copy, Clone)]
pub struct SessionId {
    id: u128,
}

impl std::ops::Add<LocalId> for SessionId {
    type Output = StableId;
    fn add(self, rhs: LocalId) -> Self::Output {
        let abs_local = (-rhs.id - 1) as u128;
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
