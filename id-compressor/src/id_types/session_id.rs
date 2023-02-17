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
    // Note: leading character should be 3 to mask at 0011
    // Verified: The more-significant half of the N nibble is used to denote the variant (10xx)
    const LOWER_MASK: u128 = 0x3FFFFFFFFFFFFFFF;

    pub(crate) fn new() -> SessionId {
        // todo doc restriction on upper bits and debug assert
        SessionId::from_uuid(Uuid::new_v4())
    }

    pub(super) fn from_uuid(uuid: uuid::Uuid) -> SessionId {
        let as_u128 = uuid.as_u128();
        SessionId::from_uuid_u128(as_u128)
    }

    pub(crate) fn from_uuid_u128(as_u128: u128) -> SessionId {
        let upper_masked = as_u128 & SessionId::UPPER_MASK;
        let middie_bitties_masked = as_u128 & SessionId::MIDDIE_BITTIES_MASK;
        let lower_masked = as_u128 & SessionId::LOWER_MASK;

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
