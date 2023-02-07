use super::*;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct SessionSpaceId {
    id: i64,
}

impl SessionSpaceId {
    pub(super) fn id(&self) -> i64 {
        self.id
    }

    pub(crate) fn to_space(&self) -> CompressedId {
        if self.is_local() {
            return CompressedId::Local(LocalId::new(self.id));
        } else {
            CompressedId::Final(FinalId::new(self.id as u64))
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
    fn from(local_id: LocalId) -> Self {
        SessionSpaceId { id: local_id.id() }
    }
}

impl From<FinalId> for SessionSpaceId {
    fn from(final_id: FinalId) -> Self {
        SessionSpaceId {
            id: final_id.id() as i64,
        }
    }
}
