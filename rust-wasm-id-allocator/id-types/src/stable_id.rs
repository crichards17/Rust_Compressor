use crate::LocalId;
use uuid::Uuid;

#[derive(Clone, Copy, PartialEq, Hash, Eq, PartialOrd, Ord, Debug)]
pub struct StableId {
    id: u128,
}

impl StableId {
    // xxxxxxxx-xxxx-Mxxx-Nxxx-xxxxxxxxxxxx
    const VERSION_MASK: u128 = 0x4 << (19 * 4); // Version 4
    const VARIANT_MASK: u128 = 0x8 << (15 * 4); // Variant RFC4122 (1 0 x x)
    const UPPER_MASK: u128 = 0xFFFFFFFFFFFF << (20 * 4);
    // Upper mask when version/variant bits are removed
    const STRIPPED_UPPER_MASK: u128 = StableId::UPPER_MASK >> 6;
    const MIDDIE_BITTIES_MASK: u128 = 0xFFF << (16 * 4);
    // Middie mask when version/variant bits are removed
    const STRIPPED_MIDDIE_BITTIES_MASK: u128 = StableId::MIDDIE_BITTIES_MASK >> 2;
    // Note: leading character should be 3 to mask at 0011
    // The more-significant half of the N nibble is used to denote the variant (10xx)
    const LOWER_MASK: u128 = 0x3FFFFFFFFFFFFFFF;

    pub fn null() -> StableId {
        StableId { id: 0 }
    }

    fn from_uuid_u128(as_u128: u128) -> StableId {
        let upper_masked = as_u128 & StableId::UPPER_MASK;
        let middie_bitties_masked = as_u128 & StableId::MIDDIE_BITTIES_MASK;
        let lower_masked = as_u128 & StableId::LOWER_MASK;

        let upper_masked = upper_masked >> 6;
        let middie_bitties_masked = middie_bitties_masked >> 2;

        let id = upper_masked | middie_bitties_masked | lower_masked;

        StableId { id }
    }

    fn to_uuid_u128(&self) -> u128 {
        // bitwise reverse transform
        let upper_masked = (self.id & StableId::STRIPPED_UPPER_MASK) << 6;
        let middie_bitties_masked = (self.id & StableId::STRIPPED_MIDDIE_BITTIES_MASK) << 2;
        let lower_masked = self.id & StableId::LOWER_MASK;
        let transformed_id = upper_masked
            | StableId::VERSION_MASK
            | middie_bitties_masked
            | StableId::VARIANT_MASK
            | lower_masked;
        transformed_id
    }
}

impl From<Uuid> for StableId {
    fn from(value: Uuid) -> Self {
        StableId::from_uuid_u128(value.as_u128())
    }
}

impl From<StableId> for Uuid {
    fn from(value: StableId) -> Self {
        uuid::Builder::from_u128(value.to_uuid_u128()).into_uuid()
    }
}

impl std::ops::Add<LocalId> for StableId {
    type Output = Self;
    fn add(self, rhs: LocalId) -> Self::Output {
        StableId {
            id: self.id + (rhs.to_generation_count() - 1) as u128,
        }
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
        let mut stable_id = StableId::from(uuid);
        assert_eq!(stable_id.to_uuid_u128(), 0xe507602db1504fccBfffffffffffffff);
        stable_id = stable_id + 1;
        let uuid = Uuid::from(stable_id);
        assert_eq!(uuid.get_variant(), uuid::Variant::RFC4122);
        assert_eq!(uuid.get_version_num(), 4);
        assert_eq!(
            Uuid::from(stable_id),
            Uuid::from_u128(0xe507602db1504fcd8000000000000000)
        );
    }
}
