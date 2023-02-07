use super::{LocalId, StableId};
use uuid::Uuid;

#[derive(Eq, PartialEq, Hash, Copy, Clone)]
pub struct SessionId {
    id: u128,
}

// todo: UUID math
impl std::ops::Add<LocalId> for SessionId {
    type Output = StableId;
    fn add(self, rhs: LocalId) -> Self::Output {
        let abs_local = (rhs.to_generation_count() - 1) as u128;
        let new_id = self.id + abs_local;
        StableId::new(new_id)
    }
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
}
