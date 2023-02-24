use super::SessionId;
use uuid::Uuid;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct StableId {
    id: u128,
}

impl StableId {
    pub(super) fn new(id: u128) -> Self {
        StableId { id }
    }

    pub(crate) fn null() -> StableId {
        StableId { id: 0 }
    }

    const VERSION_MASK: u128 = 0x4 << (19 * 4); // Version 4
    const VARIANT_MASK: u128 = 0x8 << (15 * 4); // Variant RFC4122 (1 0 x x)
    const UPPER_MASK: u128 = 0xFFFFFFFFFFFF << (74);
    const MIDDIE_BITTIES_MASK: u128 = 0xFFF << (62);
    const LOWER_MASK: u128 = 0x3FFFFFFFFFFFFFFF;

    fn to_uuid(&self) -> Uuid {
        let uuid = uuid::Builder::from_u128(self.to_uuid_u128()).into_uuid();
        return uuid;
    }

    pub fn to_uuid_string(&self) -> String {
        self.to_uuid().to_string()
    }

    pub(crate) fn to_uuid_u128(&self) -> u128 {
        // bitwise reverse transform
        let upper_masked = (self.id & StableId::UPPER_MASK) << 6;
        let middie_bitties_masked = (self.id & StableId::MIDDIE_BITTIES_MASK) << 2;
        let lower_masked = self.id & StableId::LOWER_MASK;
        let transformed_id = upper_masked
            | StableId::VERSION_MASK
            | middie_bitties_masked
            | StableId::VARIANT_MASK
            | lower_masked;
        transformed_id
    }
}

impl From<SessionId> for StableId {
    fn from(value: SessionId) -> Self {
        StableId { id: value.id() }
    }
}

impl std::ops::Add<u64> for StableId {
    type Output = Self;
    fn add(self, rhs: u64) -> Self::Output {
        StableId {
            id: self.id + rhs as u128,
        }
    }
}

impl std::ops::Sub<u64> for StableId {
    type Output = Self;
    fn sub(self, rhs: u64) -> Self::Output {
        StableId {
            id: self.id - rhs as u128,
        }
    }
}

impl std::ops::Sub<StableId> for StableId {
    type Output = u128;
    fn sub(self, rhs: StableId) -> Self::Output {
        debug_assert!(self >= rhs);
        self.id - rhs.id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_uuid_increment_spillover() {
        let uuid = Uuid::from_u128(0xe507602db1504fccBfffffffffffffff);
        let mut stable_id = StableId::from(SessionId::from_uuid(uuid));
        assert_eq!(stable_id.to_uuid_u128(), 0xe507602db1504fccBfffffffffffffff);
        stable_id = stable_id + 1;
        let uuid = stable_id.to_uuid();
        assert_eq!(uuid.get_variant(), uuid::Variant::RFC4122);
        assert_eq!(uuid.get_version_num(), 4);
    }
}
