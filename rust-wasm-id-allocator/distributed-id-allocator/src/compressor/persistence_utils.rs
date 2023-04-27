use super::persistence::DeserializationError;

pub struct Deserializer<'a> {
    bytes: &'a [u8],
    pub error: Option<DeserializationError>,
}

impl<'a> Deserializer<'a> {
    pub fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, error: None }
    }

    pub fn take_u64(self) -> (u64, Deserializer<'a>) {
        self.take_one(&u64::from_le_bytes)
    }

    pub fn take_and_write_u64(self, out: &mut u64) -> Deserializer<'a> {
        let (val, deser) = self.take_u64();
        *out = val;
        deser
    }

    pub fn take_u128(self) -> (u128, Deserializer<'a>) {
        self.take_one(&u128::from_le_bytes)
    }

    pub fn take_one<FBuild, T, const SIZE: usize>(
        self,
        builder: &'a FBuild,
    ) -> (T, Deserializer<'a>)
    where
        FBuild: Fn([u8; SIZE]) -> T,
        T: Default,
    {
        let (iter, deser) = self.take(1, builder);
        for val in iter {
            return (val, deser);
        }
        debug_assert!(false, "No value to take.");
        (T::default(), deser)
    }

    pub fn take<FBuild, T, const SIZE: usize>(
        self,
        count: usize,
        builder: &'a FBuild,
    ) -> (impl Iterator<Item = T> + Captures<'a>, Deserializer<'a>)
    where
        FBuild: Fn([u8; SIZE]) -> T,
    {
        let mut deser = Deserializer {
            bytes: self.bytes,
            error: None,
        };
        let read_try = count * SIZE;
        if read_try >= self.bytes.len() {
            debug_assert!(false, "Invalid serialized read.");
            deser.error = Some(DeserializationError::MalformedInput);
        } else {
            deser.bytes = &self.bytes[0..read_try];
        };

        let iter = (0..count * SIZE).step_by(SIZE).map(|i| {
            let mut val_arr: [u8; SIZE] = [0; SIZE];
            for offset in 0..SIZE {
                val_arr[offset] = deser.bytes[i + offset];
            }
            builder(val_arr)
        });
        (iter, deser)
    }
}

pub trait Captures<'a> {}
impl<'a, T: ?Sized> Captures<'a> for T {}

// TODO: make public if pattern is determined to avoid arch-specific things like usize
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

        let deser = Deserializer::new(&bytes);

        let mut u64s = vec![];
        let (iter, deser) = deser.take(3, &u64::from_le_bytes);
        for val in iter {
            u64s.push(val)
        }

        let mut u128s = vec![];
        let (iter, _) = deser.take(3, &u128::from_le_bytes);
        for val in iter {
            u128s.push(val)
        }
        assert_eq!(u64s, vec![1, 2, 3]);
        assert_eq!(u128s, vec![1, 2, 3]);
    }

    #[test]
    fn test_malformed_input() {
        let mut bytes: Vec<u8> = Vec::new();
        write_u64_to_vec(&mut bytes, 42);
        let deser = Deserializer::new(&bytes);
        let (val, deser) = deser.take_u128();
        assert_eq!(val, u128::default());
        assert_eq!(deser.error, Some(DeserializationError::MalformedInput));
    }
}
