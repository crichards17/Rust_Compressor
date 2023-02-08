use super::{LocalId, StableId};
use uuid::Uuid;

#[derive(Eq, PartialEq, Hash, Copy, Clone)]
pub struct SessionId {
    // doc as not a uuid
    id: u128,
}

impl SessionId {
    // xxxxxxxx-xxxx-Mxxx-Nxxx-xxxxxxxxxxxx
    const UPPER_MASK: u128 = 0xFFFFFFFFFFFF << (20 * 4);
    const MIDDIE_BITTIES_MASK: u128 = 0xFFF << (16 * 4);
    const LOWER_MASK: u128 = 0x2FFFFFFFFFFFFFFF;

    pub(crate) fn new() -> SessionId {
        // todo doc restriction on upper bits and debug assert
        SessionId::from_uuid(Uuid::new_v4().as_u128())
    }

    fn from_uuid(uuid: u128) -> SessionId {
        let upper_masked = uuid & SessionId::UPPER_MASK;
        let middie_bitties_masked = uuid & SessionId::MIDDIE_BITTIES_MASK;
        let lower_masked = uuid & SessionId::LOWER_MASK;

        let upper_masked = upper_masked >> 6;
        let middie_bitties_masked = middie_bitties_masked >> 2;
        let id = upper_masked | middie_bitties_masked | lower_masked;

        SessionId { id }
    }

    pub(super) fn id(&self) -> u128 {
        self.id
    }

    pub(crate) fn stable_from_local_offset(&self, offset_local: LocalId) -> StableId {
        let new_id = self.id + (offset_local.to_generation_count() - 1) as u128;
        StableId::new(new_id)
    }
}
