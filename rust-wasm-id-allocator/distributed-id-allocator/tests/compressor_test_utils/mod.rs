use distributed_id_allocator::compressor::IdCompressor;

pub fn serialize_roundtrip(compressor: &IdCompressor) -> RoundtrippedCompressors {
    let mut deserialized_vec: Vec<IdCompressor> = Vec::new();
    for with_local in [true, false] {
        let serialized = compressor.serialize(with_local);
        assert!(IdCompressor::deserialize(&serialized).is_ok());
        let deserialized = IdCompressor::deserialize(&serialized).unwrap();
        assert!(deserialized.equals_test_only(compressor, with_local));
        deserialized_vec.push(deserialized);
    }
    RoundtrippedCompressors {
        with_local: deserialized_vec.remove(0),
        without_local: deserialized_vec.remove(0),
    }
}

pub struct RoundtrippedCompressors {
    pub with_local: IdCompressor,
    pub without_local: IdCompressor,
}
