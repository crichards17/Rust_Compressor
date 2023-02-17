pub(crate) mod v1 {
    use crate::compressor::persistence_utils::Deserializer;
    use crate::compressor::tables::session_space_normalizer::SessionSpaceNormalizer;
    use crate::id_types::LocalId;

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

    pub(crate) fn deserialize(
        deserializer: &mut Deserializer,
        byte_length: usize,
    ) -> SessionSpaceNormalizer {
        let mut normalizer = SessionSpaceNormalizer::new();

        for _ in 0..byte_length / 16 {
            let local_id = LocalId::from_generation_count(deserializer.consume_u64());
            let count = deserializer.consume_u64();
            normalizer.leading_locals.push((local_id, count));
        }
        normalizer
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compressor::persistence_utils::Deserializer;
    use crate::compressor::tables::session_space_normalizer::SessionSpaceNormalizer;
    use crate::id_types::LocalId;

    #[test]
    fn serialize_deserialize() {
        let mut session_space_normalizer = SessionSpaceNormalizer::new();

        session_space_normalizer.add_local_range(LocalId::new(-1), 3);
        session_space_normalizer.add_local_range(LocalId::new(-6), 1);
        session_space_normalizer.add_local_range(LocalId::new(-8), 2);
        session_space_normalizer.add_local_range(LocalId::new(-12), 5);
        session_space_normalizer.add_local_range(LocalId::new(-20), 3);

        let serialized = v1::serialize(&session_space_normalizer);
        assert_eq!(
            session_space_normalizer.leading_locals,
            v1::deserialize(&mut Deserializer::new(&serialized), serialized.len()).leading_locals
        );
    }
}
