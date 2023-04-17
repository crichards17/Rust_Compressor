use crate::LocalId;
use uuid::Uuid;

#[derive(Clone, Copy, PartialEq, Hash, Eq, PartialOrd, Ord, Debug)]
/// A compressed version 4, variant 1 uuid (https://datatracker.ietf.org/doc/html/rfc4122).
/// Can be converted to a UUID, u128, or String as needed.
pub struct StableId {
    id: u128,
}

impl StableId {
    /// Returns the StableId representation of the nil UUID.
    pub fn null() -> StableId {
        StableId { id: 0 }
    }
}

// xxxxxxxx-xxxx-Mxxx-Nxxx-xxxxxxxxxxxx
const VERSION_MASK: u128 = 0x4 << (19 * 4); // Version 4
const VARIANT_MASK: u128 = 0x8 << (15 * 4); // Variant RFC4122 (1 0 x x)
const UPPER_MASK: u128 = 0xFFFFFFFFFFFF << (20 * 4);
// Upper mask when version/variant bits are removed
const STRIPPED_UPPER_MASK: u128 = UPPER_MASK >> 6;
const MIDDIE_BITTIES_MASK: u128 = 0xFFF << (16 * 4);
// Middie mask when version/variant bits are removed
const STRIPPED_MIDDIE_BITTIES_MASK: u128 = MIDDIE_BITTIES_MASK >> 2;
// Note: leading character should be 3 to mask at 0011
// The more-significant half of the N nibble is used to denote the variant (10xx)
const LOWER_MASK: u128 = 0x3FFFFFFFFFFFFFFF;

impl From<StableId> for [u8; 36] {
    fn from(id: StableId) -> Self {
        let mut uuid_arr: [u8; 36] = ['0' as u8; 36];
        _ = Uuid::from(id).as_hyphenated().encode_lower(&mut uuid_arr);
        uuid_arr
    }
}

impl From<u128> for StableId {
    fn from(uuid_u128: u128) -> Self {
        let upper_masked = uuid_u128 & UPPER_MASK;
        let middie_bitties_masked = uuid_u128 & MIDDIE_BITTIES_MASK;
        let lower_masked = uuid_u128 & LOWER_MASK;

        let upper_masked = upper_masked >> 6;
        let middie_bitties_masked = middie_bitties_masked >> 2;

        let id = upper_masked | middie_bitties_masked | lower_masked;

        StableId { id }
    }
}

impl From<StableId> for u128 {
    fn from(value: StableId) -> Self {
        // bitwise reverse transform
        let upper_masked = (value.id & STRIPPED_UPPER_MASK) << 6;
        let middie_bitties_masked = (value.id & STRIPPED_MIDDIE_BITTIES_MASK) << 2;
        let lower_masked = value.id & LOWER_MASK;
        let transformed_id =
            upper_masked | VERSION_MASK | middie_bitties_masked | VARIANT_MASK | lower_masked;
        transformed_id
    }
}

impl From<Uuid> for StableId {
    fn from(value: Uuid) -> Self {
        value.as_u128().into()
    }
}

impl From<StableId> for Uuid {
    fn from(value: StableId) -> Self {
        uuid::Builder::from_u128(value.into()).into_uuid()
    }
}

impl From<StableId> for String {
    fn from(value: StableId) -> Self {
        Uuid::from(value).to_string()
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
        assert_eq!(u128::from(stable_id), 0xe507602db1504fccBfffffffffffffff);
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
