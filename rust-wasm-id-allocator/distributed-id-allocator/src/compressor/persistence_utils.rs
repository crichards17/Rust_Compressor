pub struct Deserializer<'a> {
    bytes: &'a [u8],
}

impl<'a> Deserializer<'a> {
    pub fn new(bytes: &'a [u8]) -> Self {
        Self { bytes }
    }

    pub fn take_u64(&mut self) -> u64 {
        self.take_one(&u64::from_le_bytes)
    }

    pub fn take_u128(&mut self) -> u128 {
        self.take_one(&u128::from_le_bytes)
    }

    fn take_one<FBuild, T, const SIZE: usize>(&mut self, builder: &'a FBuild) -> T
    where
        FBuild: Fn([u8; SIZE]) -> T,
    {
        let mut val_arr: [u8; SIZE] = [0; SIZE];
        for offset in 0..SIZE {
            val_arr[offset] = self.bytes[offset];
        }
        self.bytes = &self.bytes[SIZE..];
        builder(val_arr)
    }
}

fn write_to_vec<FToBytes, T, const SIZE: usize>(bytes: &mut Vec<u8>, val: T, builder: FToBytes)
where
    FToBytes: Fn(T) -> [u8; SIZE],
{
    let val_arr = builder(val);
    for byte in val_arr {
        bytes.push(byte);
    }
}

pub fn write_u64_to_vec(buffer: &mut Vec<u8>, num: u64) {
    write_to_vec(buffer, num, |val: u64| val.to_le_bytes());
}

pub fn write_u128_to_vec(buffer: &mut Vec<u8>, num: u128) {
    write_to_vec(buffer, num, |val: u128| val.to_le_bytes());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serde_utils() {
        let mut bytes: Vec<u8> = Vec::new();

        vec![1, 2, 3]
            .iter()
            .for_each(|val| write_u64_to_vec(&mut bytes, *val));

        vec![1, 2, 3]
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
        assert_eq!(u128s, vec![1, 2, 3]);
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
