#[derive(Clone, Copy, PartialEq, PartialOrd, Eq, Ord, Debug)]
/// A compressed ID that is stable and unique within the scope of network of compressors (i.e. a document).
/// It can only be used/decompressed in the context of the originating document.
pub struct FinalId {
    id: u64,
}

impl FinalId {
    pub(super) fn id(&self) -> u64 {
        self.id
    }

    /// Creates a final ID from a u64. Intended for internal use only.
    pub fn from_id(id: u64) -> Self {
        FinalId { id }
    }
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

impl std::ops::Sub<FinalId> for FinalId {
    type Output = i64;
    fn sub(self, rhs: FinalId) -> Self::Output {
        debug_assert!(self.id >= rhs.id, "Final ID subtraction overflow");
        self.id as i64 - rhs.id as i64
    }
}
