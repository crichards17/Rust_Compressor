use uuid::Uuid;

pub struct SessionSpaceId {
    pub(crate) id: i64,
}

pub struct OpSpaceId {
    pub(crate) id: i64,
}

pub struct LocalId {
    id: i64,
}

impl LocalId {
    pub fn new(id: i64) -> LocalId {
        debug_assert!(
            id < 0,
            "Local ID must be negative. Passed value was {}.",
            id,
        );
        LocalId { id }
    }

    pub fn id(&self) -> i64 {
        self.id
    }
}

pub struct FinalId {
    pub(crate) id: u64,
}

pub struct StableId {
    pub(crate) id: u128,
}

#[derive(Copy, Clone)]
pub struct SessionId {
    id: u128,
}

impl SessionId {
    pub(crate) fn new() -> SessionId {
        SessionId {
            id: Uuid::new_v4().as_u128(),
        }
    }

    pub(crate) fn id(&self) -> u128 {
        self.id
    }
}
