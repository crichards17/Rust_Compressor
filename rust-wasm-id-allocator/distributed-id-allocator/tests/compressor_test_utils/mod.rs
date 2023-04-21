use distributed_id_allocator::compressor::IdCompressor;
use id_types::SessionSpaceId;

pub fn serialize_roundtrip(compressor: &IdCompressor) -> RoundtrippedCompressors {
    let mut deserialized_vec: Vec<IdCompressor> = Vec::new();
    for with_local in [true, false] {
        let serialized = compressor.serialize(with_local);
        let deserialized_result = IdCompressor::deserialize(&serialized);
        assert!(deserialized_result.is_ok());
        let deserialized = deserialized_result.unwrap();
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

pub trait TestSessionSpaceId {
    fn unwrap_uuid_str(&self, compressor: &IdCompressor) -> String;
}

impl TestSessionSpaceId for SessionSpaceId {
    fn unwrap_uuid_str(&self, compressor: &IdCompressor) -> String {
        compressor.decompress(*self).ok().unwrap().into()
    }
}

pub fn generate_n_ids(compressor: &mut IdCompressor, num_ids: i32) -> Vec<SessionSpaceId> {
    let mut ids = Vec::new();
    for _ in 0..num_ids {
        ids.push(compressor.generate_next_id())
    }
    ids
}

pub fn finalize_next_range(compressor: &mut IdCompressor) {
    let range = compressor.take_next_range();
    _ = compressor.finalize_range(&range);
}
