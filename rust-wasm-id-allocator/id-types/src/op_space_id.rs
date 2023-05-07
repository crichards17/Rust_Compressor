use crate::{
    final_id::final_id_from_id,
    local_id::{get_id_from_local_id, local_id_from_id},
};

use super::*;

#[derive(Clone, Copy, Debug, PartialOrd, Ord, PartialEq, Eq)]
/// A compressed ID that has been normalized into "op space".
/// Serialized/persisted structures (e.g. ops) should use op-space IDs as a performance optimization, as they require less work to
/// normalize when received by a remote client.
pub struct OpSpaceId {
    id: i64,
}

impl OpSpaceId {
    /// Returns the inner ID as an i64. Intended for internal use only.
    pub fn id(&self) -> i64 {
        self.id
    }

    /// Creates an OpSpaceId from an i64. Intended for internal use only.
    pub fn from_id(id: i64) -> OpSpaceId {
        Self { id }
    }

    /// Maps the ID to local or final space. Intended for internal use only.
    pub fn to_space(&self) -> CompressedId {
        if self.is_local() {
            CompressedId::Local(local_id_from_id(self.id))
        } else {
            CompressedId::Final(final_id_from_id(self.id as u64))
        }
    }

    /// Returns true iff the op space ID is in local space. Intended for internal use only.
    pub fn is_local(&self) -> bool {
        self.id < 0
    }

    /// Returns true iff the op space ID is in final space. Intended for internal use only.
    pub fn is_final(&self) -> bool {
        self.id >= 0
    }
}

impl From<FinalId> for OpSpaceId {
    fn from(final_id: FinalId) -> Self {
        OpSpaceId {
            id: final_id.id as i64,
        }
    }
}

impl From<LocalId> for OpSpaceId {
    fn from(local_id: LocalId) -> Self {
        OpSpaceId {
            id: get_id_from_local_id(local_id),
        }
    }
}
