use std::ops::Sub;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
/// A compressed ID that is local to a session (can only be decompressed when paired with a SessionId).
/// Internally, it should not be persisted outside a scope annotated with the originating SessionId in order to be unambiguous.
/// If external persistence is needed (e.g. by a client), a StableId should be used instead.
pub struct LocalId {
    id: i64,
}

impl LocalId {
    /// Returns the inner ID as a generation count. Intended for internal use only.
    pub fn to_generation_count(&self) -> u64 {
        (-self.id) as u64
    }

    /// Creates a local ID from a generation count. Intended for internal use only.
    pub fn from_generation_count(generation_count: u64) -> Self {
        local_id_from_id(-(generation_count as i64))
    }
}

/// Creates a local ID from an i64. Intended for internal use only.
pub fn local_id_from_id(id: i64) -> LocalId {
    debug_assert!(
        id < 0,
        "Local ID must be negative. Passed value was {}.",
        id,
    );
    LocalId { id }
}

/// Gets an i64 from a local ID. Intended for internal use only.
pub const fn get_id_from_local_id(local_id: LocalId) -> i64 {
    local_id.id
}

impl PartialEq<i64> for LocalId {
    fn eq(&self, other: &i64) -> bool {
        self.id == *other
    }
}

impl Sub<u64> for LocalId {
    type Output = LocalId;
    fn sub(self, rhs: u64) -> Self::Output {
        local_id_from_id(self.id - rhs as i64)
    }
}
