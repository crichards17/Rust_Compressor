use serde::{Deserialize, Serialize};
use std::ops::Sub;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Serialize, Deserialize)]
/// A compressed ID that is local to a session (can only be decompressed when paired with a SessionId).
/// Internally, it should not be persisted outside a scope annotated with the originating SessionId in order to be unambiguous.
/// If external persistence is needed (e.g. by a client), a StableId should be used instead.
pub struct LocalId {
    id: i64,
}

impl LocalId {
    /// Creates a local ID from a i64. Intended for internal use only.
    pub fn from_id(id: i64) -> LocalId {
        debug_assert!(
            id < 0,
            "Local ID must be negative. Passed value was {}.",
            id,
        );
        LocalId { id }
    }

    /// Returns the inner ID as an i64. Intended for internal use only.
    pub fn id(&self) -> i64 {
        self.id
    }

    /// Returns the inner ID as a generation count. Intended for internal use only.
    pub fn to_generation_count(&self) -> u64 {
        (-self.id) as u64
    }

    /// Creates a local ID from a generation count. Intended for internal use only.
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
            Some(std::cmp::Ordering::Less)
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
