#[derive(Clone, Copy, PartialEq, PartialOrd, Eq, Ord, Debug)]
/// A compressed ID that is stable and unique within the scope of network of compressors (i.e. a document).
/// It can only be used/decompressed in the context of the originating document.
pub struct FinalId {
    pub(crate) id: u64,
}

/// Creates a final ID from a u64. Intended for internal use only.
pub const fn final_id_from_id(id: u64) -> FinalId {
    FinalId { id }
}

/// Gets a u64 from a final ID. Intended for internal use only.
pub const fn get_id_from_final_id(final_id: FinalId) -> u64 {
    final_id.id
}

impl std::ops::Add<u64> for FinalId {
    type Output = FinalId;
    fn add(self, rhs: u64) -> Self::Output {
        FinalId { id: self.id + rhs }
    }
}

impl std::ops::Sub<FinalId> for FinalId {
    type Output = i64;
    fn sub(self, rhs: FinalId) -> Self::Output {
        debug_assert!(self.id >= rhs.id, "Final ID subtraction overflow");
        self.id as i64 - rhs.id as i64
    }
}
