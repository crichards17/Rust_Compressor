use crate::LocalId;
use uuid::Uuid;

#[derive(Clone, Copy, PartialEq, Hash, Eq, PartialOrd, Ord, Debug)]
/// A compressed version 4, variant 1 UUID (https://datatracker.ietf.org/doc/html/rfc4122).
/// Can be converted to a UUID, u128, or String as needed.
pub struct StableId {
    pub(crate) id: u128,
}

impl StableId {
    /// Returns the StableId representation of the nil UUID.
    pub fn nil() -> StableId {
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

impl From<StableId> for Vec<u8> {
    fn from(id: StableId) -> Self {
        let mut uuid_arr: [u8; 36] = [b'0'; 36];
        _ = Uuid::from(id).as_hyphenated().encode_lower(&mut uuid_arr);
        Vec::from(uuid_arr)
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

        upper_masked | VERSION_MASK | middie_bitties_masked | VARIANT_MASK | lower_masked
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
        debug_assert!(self.id >= rhs as u128);
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
    use std::str::FromStr;

    use super::*;

    #[test]
    fn test_addition() {
        let stable_id = StableId::from(0x00000000000040008000000000000000);
        assert_eq!(
            u128::from(stable_id + 5),
            0x00000000000040008000000000000005
        );
    }

    #[test]
    fn test_u64_subtraction() {
        let uuid_1 = StableId::from(0x100000000000400080000000000000F0);
        assert_eq!(
            uuid_1 - 3_u64,
            StableId::from(0x100000000000400080000000000000ED)
        );
    }

    #[test]
    fn test_u128_subtraction() {
        let uuid_1 = StableId::from(0x10000000000040008000000000000005);
        let uuid_2 = StableId::from(0x20000000000040008000000000000009);
        // 0x10000000000040008000000000000009, but remove the four bits for the '4' and the two bits
        // for the variant '8', and slide the '1' to the right those 6 bits.
        assert_eq!(uuid_2 - uuid_1, 0x00400000000000000000000000000004);
    }

    #[test]
    fn test_math_multiple_uuids() {
        let mut uuids = vec![
            Uuid::parse_str("00000000-0000-4000-8000-000000000000").unwrap(),
            Uuid::parse_str("748540ca-b7c5-4c99-83ff-c1b8e02c09d6").unwrap(),
            Uuid::parse_str("748540ca-b7c5-4c99-83ef-c1b8e02c09d6").unwrap(),
            Uuid::parse_str("748540ca-b7c5-4c99-831f-c1b8e02c09d6").unwrap(),
            Uuid::parse_str("0002c79e-b536-4776-b000-000266c252d5").unwrap(),
            Uuid::parse_str("082533b9-6d05-4068-a008-fe2cc43543f7").unwrap(),
            Uuid::parse_str("2c9fa1f8-48d5-4554-a466-000000000000").unwrap(),
            Uuid::parse_str("2c9fa1f8-48d5-4000-a000-000000000000").unwrap(),
            Uuid::parse_str("10000000-0000-4000-b000-000000000000").unwrap(),
            Uuid::parse_str("10000000-0000-4000-b020-000000000000").unwrap(),
            Uuid::parse_str("10000000-0000-4000-b00f-ffffffffffff").unwrap(),
            Uuid::parse_str("10000000-0000-4000-b040-000000000000").unwrap(),
            Uuid::parse_str("f0000000-0000-4000-8000-000000000000").unwrap(),
            Uuid::parse_str("efffffff-ffff-4fff-bfff-ffffffffffff").unwrap(),
        ];
        uuids.sort_by(|uuid_a, uuid_b| StableId::from(*uuid_b).cmp(&StableId::from(*uuid_a)));

        for i in 0..uuids.len() {
            let uuid_a = uuids[i];
            let stable_id_a = StableId::from(uuid_a);
            assert_eq!(stable_id_a - stable_id_a, 0);
            for num in vec![1, 100, 1000000, 1000000000000000000_u64] {
                let uuid = Uuid::from(stable_id_a + num);
                assert_eq!(uuid.get_version_num(), 4);
                assert_eq!(uuid.get_variant(), uuid::Variant::RFC4122);
            }
            for j in i..uuids.len() {
                let uuid_b = uuids[j];
                let stable_id_b = StableId::from(uuid_b);
                let uuid_plus_delta = Uuid::from(StableId {
                    id: stable_id_b.id + (stable_id_a - stable_id_b),
                });
                assert_eq!(uuid_plus_delta, uuid_a);
            }
        }
    }

    #[test]
    fn test_roundtrip() {
        let stable_id = StableId::from(0x00000000000040008000000000000000);
        assert_eq!(
            StableId::from(Uuid::from_str(&String::from(stable_id)).unwrap()),
            stable_id
        );
    }

    #[test]
    fn test_uuid_increment_spillover() {
        let mut stable_id = StableId::from(0xe507602db1504fccBfffffffffffffff);
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
