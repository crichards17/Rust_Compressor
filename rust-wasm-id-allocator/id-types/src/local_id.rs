use serde::{Deserialize, Serialize};
use std::ops::Sub;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Serialize, Deserialize)]
pub struct LocalId {
    id: i64,
}

impl LocalId {
    pub fn from_id(id: i64) -> LocalId {
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
        LocalId::from_id(-(generation_count as i64))
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
        LocalId::from_id(self.id - rhs as i64)
    }
}
