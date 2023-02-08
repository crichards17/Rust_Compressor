pub mod compressor;
pub mod id_types;

#[cfg(test)]
mod tests {

    #[test]
    fn it_works() {
        assert_eq!(4, 4);
    }

    // AND: &
    // OR:  |
    // XOR: ^
    // NEG: ~
    // LSHIFT: << X
    // RSHIFT: >> X
    // ALSHIFT: <<< X
    // ARSHIFT: >>> X

    fn twiddling_is_odd(num: u64) -> bool {
        num & 1 == 1
    }

    fn twiddling_middle_bits(num: u64) -> u64 {
        // Given u64, return middle 16 bits as a number
        let mut mask: u64 = (2 as u64).pow(16) - 1;
        mask = mask << 24;
        num & mask
    }

    fn twiddling_mirror(num: u64) -> u64 {
        let mut build_num = 0;
        for i in 0..64 {
            let mut mask = 1 << i;
            mask = num & mask;
            mask = mask >> i;
            mask = mask << 63 - i;
            build_num = build_num | mask;
        }
        build_num
    }

    #[test]
    fn test_mirror() {
        assert_eq!(twiddling_mirror(1), (2 as u64).pow(63));

        assert_eq!(1, twiddling_mirror(twiddling_mirror(1)));

        assert_eq!(134524122134234, twiddling_mirror(twiddling_mirror(134524122134234)));
    }
}
