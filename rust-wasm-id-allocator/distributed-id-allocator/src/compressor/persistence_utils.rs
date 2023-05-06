pub struct Deserializer<'a> {
    bytes: &'a [u8],
}

impl<'a> Deserializer<'a> {
    pub fn new(bytes: &'a [u8]) -> Self {
        Self { bytes }
    }

    pub fn take_u32(&mut self) -> u32 {
        self.take_one(u32::from_le_bytes)
    }

    pub fn take_u64(&mut self) -> u64 {
        self.take_one(u64::from_le_bytes)
    }

    pub fn take_u128(&mut self) -> u128 {
        self.take_one(u128::from_le_bytes)
    }

    #[inline]
    fn take_one<FBuild, T, const SIZE: usize>(&mut self, builder: FBuild) -> T
    where
        FBuild: Fn([u8; SIZE]) -> T,
    {
        let val = builder(self.bytes[..SIZE].try_into().unwrap());
        self.bytes = &self.bytes[SIZE..];
        val
    }
}

fn write_to_vec<FToBytes, T, const SIZE: usize>(bytes: &mut Vec<u8>, val: T, builder: FToBytes)
where
    FToBytes: Fn(T) -> [u8; SIZE],
{
    let val_arr = builder(val);
    bytes.extend_from_slice(&val_arr);
}

#[inline]
pub fn write_u32_to_vec(buffer: &mut Vec<u8>, num: u32) {
    write_to_vec(buffer, num, |val: u32| val.to_le_bytes());
}

#[inline]
pub fn write_u64_to_vec(buffer: &mut Vec<u8>, num: u64) {
    write_to_vec(buffer, num, |val: u64| val.to_le_bytes());
}

#[inline]
pub fn write_u128_to_vec(buffer: &mut Vec<u8>, num: u128) {
    write_to_vec(buffer, num, |val: u128| val.to_le_bytes());
}

#[cfg(test)]
mod tests {
    use id_types::{SessionId, StableId};

    use super::*;

    #[test]
    fn test_serde_utils() {
        let mut bytes: Vec<u8> = Vec::new();

        vec![1, 2, 3]
            .iter()
            .for_each(|val| write_u64_to_vec(&mut bytes, *val));

        let u128s_original = vec![
            u128::from(StableId::from(SessionId::new())),
            u128::from(StableId::from(SessionId::new())),
            u128::from(StableId::from(SessionId::new())),
        ];
        u128s_original
            .iter()
            .for_each(|val| write_u128_to_vec(&mut bytes, *val));

        let mut deser = Deserializer::new(&bytes);

        let mut u64s = vec![];
        for _ in 0..3 {
            u64s.push(deser.take_u64())
        }

        let mut u128s = vec![];
        for _ in 0..3 {
            u128s.push(deser.take_u128())
        }

        assert_eq!(u64s, vec![1, 2, 3]);
        assert_eq!(u128s, u128s_original);
    }

    #[test]
    #[should_panic]
    fn test_malformed_input() {
        let mut bytes: Vec<u8> = Vec::new();
        write_u64_to_vec(&mut bytes, 42);
        let mut deser = Deserializer::new(&bytes);
        _ = deser.take_u128();
    }
}
