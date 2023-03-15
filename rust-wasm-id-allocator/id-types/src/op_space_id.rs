use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OpSpaceId {
    id: i64,
}

impl OpSpaceId {
    pub fn id(&self) -> i64 {
        self.id
    }

    // TODO: don't export out of crate
    pub fn from_id(id: i64) -> OpSpaceId {
        Self { id }
    }

    pub fn to_space(&self) -> CompressedId {
        if self.is_local() {
            return CompressedId::Local(LocalId::from_id(self.id));
        } else {
            CompressedId::Final(FinalId::from_id(self.id as u64))
        }
    }

    pub fn is_local(&self) -> bool {
        self.id < 0
    }
}

impl From<FinalId> for OpSpaceId {
    fn from(final_id: FinalId) -> Self {
        OpSpaceId {
            id: final_id.id() as i64,
        }
    }
}

impl From<LocalId> for OpSpaceId {
    fn from(local_id: LocalId) -> Self {
        OpSpaceId { id: local_id.id() }
    }
}
