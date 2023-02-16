pub(crate) mod v1 {

    use crate::compressor::tables::session_space_normalizer::SessionSpaceNormalizer;

    pub fn serialize(session_space_normalizer: &SessionSpaceNormalizer) -> Vec<u8> {
        // Bytewise representation of (LocalId, u64) tuples
        let mut bytes_out: Vec<u8> = Vec::new();
        for (local_id, count) in &session_space_normalizer.leading_locals {
            // ! Serializes as generation count !
            let local_id_bytes = local_id.to_generation_count().to_be_bytes();
            // 8 bytes: LocalId generation count (u64)
            for local_id_byte in local_id_bytes {
                bytes_out.push(local_id_byte);
            }
            let count_bytes = count.to_be_bytes();
            // 8 bytes: count (u64)
            for count_byte in count_bytes {
                bytes_out.push(count_byte);
            }
        }
        bytes_out
    }
}
