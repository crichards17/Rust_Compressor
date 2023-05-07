use crate::{
    final_id::final_id_from_id,
    local_id::{get_id_from_local_id, local_id_from_id},
};

use super::*;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
/// A compressed ID that has been normalized into "session space".
/// Consumer-facing APIs and data structures should use session-space IDs as their lifetime and equality is stable and tied to
/// the scope of the session (i.e. compressor) that produced them.
pub struct SessionSpaceId {
    id: i64,
}

impl SessionSpaceId {
    /// Returns the inner ID as an i64. Intended for internal use only.
    pub fn id(&self) -> i64 {
        self.id
    }

    /// Creates an SessionSpaceId from an i64. Intended for internal use only.
    pub fn from_id(id: i64) -> SessionSpaceId {
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

    /// Returns true iff the session space ID is in local space. Intended for internal use only.
    pub fn is_local(&self) -> bool {
        self.id < 0
    }

    /// Returns true iff the session space ID is in final space. Intended for internal use only.
    pub fn is_final(&self) -> bool {
        self.id >= 0
    }
}

impl From<LocalId> for SessionSpaceId {
    fn from(local_id: LocalId) -> Self {
        SessionSpaceId {
            id: get_id_from_local_id(local_id),
        }
    }
}

impl From<FinalId> for SessionSpaceId {
    fn from(final_id: FinalId) -> Self {
        SessionSpaceId {
            id: final_id.id as i64,
        }
    }
}
