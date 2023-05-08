pub(crate) mod v1 {
    use crate::compressor::{
        persistence_utils::{write_u64_to_vec, Deserializer},
        tables::session_space_normalizer::SessionSpaceNormalizer,
    };
    use id_types::LocalId;

    pub fn serialize_normalizer(
        session_space_normalizer: &SessionSpaceNormalizer,
        bytes: &mut Vec<u8>,
    ) {
        // Layout:
        // Len (u64)
        // (Local as gen_count, count)[]
        write_u64_to_vec(bytes, session_space_normalizer.leading_locals.len() as u64);
        session_space_normalizer
            .leading_locals
            .iter()
            .for_each(|(local, count)| {
                write_u64_to_vec(bytes, local.to_generation_count());
                write_u64_to_vec(bytes, *count);
            });
    }

    pub fn deserialize_normalizer(deserializer: &mut Deserializer) -> SessionSpaceNormalizer {
        let len = deserializer.take_u64();
        let mut normalizer = SessionSpaceNormalizer::new();
        for _ in 0..len {
            let gen_count = deserializer.take_u64();
            let count = deserializer.take_u64();
            normalizer
                .leading_locals
                .push((LocalId::from_generation_count(gen_count), count));
        }
        normalizer
    }
}

#[cfg(test)]
mod tests {
    use super::v1::{deserialize_normalizer, serialize_normalizer};
    use crate::compressor::{
        persistence_utils::Deserializer, tables::session_space_normalizer::SessionSpaceNormalizer,
    };
    use id_types::local_id::local_id_from_id;

    #[test]
    fn test_serde_normalizer() {
        let mut session_space_normalizer = SessionSpaceNormalizer::new();
        session_space_normalizer.add_local_range(local_id_from_id(-1), 2);
        session_space_normalizer.add_local_range(local_id_from_id(-3), 4);
        session_space_normalizer.add_local_range(local_id_from_id(-15), 1);

        let mut bytes: Vec<u8> = Vec::new();
        serialize_normalizer(&session_space_normalizer, &mut bytes);
        let normalizer = deserialize_normalizer(&mut Deserializer::new(&bytes));
        assert!(normalizer.eq(&session_space_normalizer));
    }
}
