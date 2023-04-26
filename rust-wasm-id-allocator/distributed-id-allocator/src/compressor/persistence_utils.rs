pub struct Deserializer<'a> {
    bytes: &'a [u8],
    handle: usize,
}

impl<'a> Deserializer<'a> {
    pub fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, handle: 0 }
    }

    pub fn take_one<FBuild, T, const SIZE: usize>(self, builder: FBuild) -> (T, Deserializer<'a>)
    where
        FBuild: Fn([u8; SIZE]) -> T,
        T: Default,
    {
        let mut out: T = T::default();
        let deser = self.take(1, builder, |val| out = val);
        (out, deser)
    }

    pub fn take<FBuild, FConsume, T, const SIZE: usize>(
        self,
        count: usize,
        builder: FBuild,
        mut consume: FConsume,
    ) -> Deserializer<'a>
    where
        FBuild: Fn([u8; SIZE]) -> T,
        FConsume: FnMut(T),
    {
        let new_handle = self.handle + count * SIZE;
        let slice = &self.bytes[self.handle..new_handle];
        for val in (0..count * SIZE).step_by(SIZE).map(|i| {
            let mut val_arr: [u8; SIZE] = [0; SIZE];
            for offset in 0..SIZE {
                val_arr[offset] = slice[i + offset];
            }
            builder(val_arr)
        }) {
            consume(val)
        }
        Deserializer {
            bytes: self.bytes,
            handle: new_handle,
        }
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
    use super::*;

    #[test]
    fn test_serde_utils() {
        let mut bytes: Vec<u8> = Vec::new();

        vec![1, 2, 3]
            .iter()
            .for_each(|val| write_u32_to_vec(&mut bytes, *val));

        vec![1, 2, 3]
            .iter()
            .for_each(|val| write_u64_to_vec(&mut bytes, *val));

        vec![1, 2, 3]
            .iter()
            .for_each(|val| write_u128_to_vec(&mut bytes, *val));

        let deser = Deserializer::new(&bytes);

        let mut u32s = vec![];
        let mut u64s = vec![];
        let mut u128s = vec![];
        _ = deser
            .take(3, u32::from_le_bytes, |val| u32s.push(val))
            .take(3, u64::from_le_bytes, |val| u64s.push(val))
            .take(3, u128::from_le_bytes, |val| u128s.push(val));
        assert_eq!(u32s, vec![1, 2, 3]);
        assert_eq!(u64s, vec![1, 2, 3]);
        assert_eq!(u128s, vec![1, 2, 3]);
    }
}
