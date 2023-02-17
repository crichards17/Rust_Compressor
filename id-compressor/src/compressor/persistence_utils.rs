pub(crate) struct Deserializer<'a> {
    bytes: &'a [u8],
    handle: usize,
}

impl<'a> Deserializer<'a> {
    pub fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, handle: 0 }
    }

    pub fn consume_u8(&mut self) -> u8 {
        let out = self.bytes[self.handle];
        self.handle += 1;
        out
    }

    pub fn consume_u32(&mut self) -> u32 {
        let mut slice: [u8; 4] = [0; 4];
        for offset in 0..4 {
            slice[offset] = self.bytes[self.handle + offset];
        }
        self.handle += 4;
        u32::from_be_bytes(slice)
    }

    pub fn consume_u64(&mut self) -> u64 {
        let mut slice: [u8; 8] = [0; 8];
        for offset in 0..8 {
            slice[offset] = self.bytes[self.handle + offset];
        }
        self.handle += 8;
        u64::from_be_bytes(slice)
    }

    pub fn consume_u128(&mut self) -> u128 {
        let mut slice: [u8; 16] = [0; 16];
        for offset in 0..16 {
            slice[offset] = self.bytes[self.handle + offset];
        }
        self.handle += 16;
        u128::from_be_bytes(slice)
    }
}

pub fn write_u32_to_vec(buffer: &mut Vec<u8>, num: u32) {
    let bytes = num.to_be_bytes();
    for byte in bytes {
        buffer.push(byte);
    }
}

pub fn write_u64_to_vec(buffer: &mut Vec<u8>, num: u64) {
    let bytes = num.to_be_bytes();
    for byte in bytes {
        buffer.push(byte);
    }
}

pub fn write_u128_to_vec(buffer: &mut Vec<u8>, num: u128) {
    let bytes = num.to_be_bytes();
    for byte in bytes {
        buffer.push(byte);
    }
}
