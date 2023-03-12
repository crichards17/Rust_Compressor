use super::*;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct SessionSpaceId {
    id: i64,
}

impl SessionSpaceId {
    // TODO: don't export out of crate
    pub fn id(&self) -> i64 {
        self.id
    }

    // TODO: don't export out of crate
    pub fn from_id(id: i64) -> SessionSpaceId {
        Self { id }
    }

    pub(crate) fn to_space(&self) -> CompressedId {
        if self.is_local() {
            return CompressedId::Local(LocalId::from_id(self.id));
        } else {
            CompressedId::Final(FinalId::from_id(self.id as u64))
        }
    }

    pub(crate) fn is_local(&self) -> bool {
        self.id < 0
    }

    #[cfg(test)]
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
