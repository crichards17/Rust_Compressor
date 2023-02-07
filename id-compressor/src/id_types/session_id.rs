use super::{LocalId, StableId};
use uuid::Uuid;

#[derive(Eq, PartialEq, Hash, Copy, Clone)]
pub struct SessionId {
    id: u128,
}

impl SessionId {
    pub(crate) fn new() -> SessionId {
        SessionId {
            id: Uuid::new_v4().as_u128(),
        }
    }

    pub(super) fn id_raw(&self) -> u128 {
        self.id
    }

    // TODO: UUID math
    pub(crate) fn stable_from_local_offset(&self, offset_local: LocalId) -> StableId {
        let new_id = self.id + (offset_local.to_generation_count() - 1) as u128;
        StableId::new(new_id)
    }
}

// Deprecating
// todo: UUID math
// impl std::ops::Add<LocalId> for SessionId {
//     type Output = StableId;
//     fn add(self, rhs: LocalId) -> Self::Output {
//         let new_id = self.id + (rhs.to_generation_count() - 1) as u128;
//         StableId::new(new_id)
//     }
// }
