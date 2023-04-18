use distributed_id_allocator::compressor::*;
use id_types::*;
use std::collections::HashSet;

use uuid::Uuid;

const LEADING_EDGE_OF_VERSION_SESSION_ID: &str = "00000000-0000-4fff-bfff-ffffffffffff";
const TRAILING_EDGE_OF_VERSION_SESSION_ID: &str = "00000000-0001-4000-8000-000000000000";

const _STABLE_IDS: &[&str] = &[
    "748540ca-b7c5-4c99-83ff-c1b8e02c09d6",
    "748540ca-b7c5-4c99-83ef-c1b8e02c09d6",
    "748540ca-b7c5-4c99-831f-c1b8e02c09d6",
    "0002c79e-b536-4776-b000-000266c252d5",
    "082533b9-6d05-4068-a008-fe2cc43543f7",
    "2c9fa1f8-48d5-4554-a466-000000000000",
    "2c9fa1f8-48d5-4000-a000-000000000000",
    "10000000-0000-4000-b000-000000000000",
    "10000000-0000-4000-b020-000000000000", // 2^52
    "10000000-0000-4000-b00f-ffffffffffff",
    "10000000-0000-4000-b040-000000000000",
    "f0000000-0000-4000-8000-000000000000",
    "efffffff-ffff-4fff-bfff-ffffffffffff",
    LEADING_EDGE_OF_VERSION_SESSION_ID,
];

trait TestSessionSpaceId {
    fn unwrap_uuid_str(&self, compressor: &IdCompressor) -> String;
}

impl TestSessionSpaceId for SessionSpaceId {
    fn unwrap_uuid_str(&self, compressor: &IdCompressor) -> String {
        compressor.decompress(*self).unwrap().into()
    }
}

fn generate_n_ids(compressor: &mut IdCompressor, num_ids: i32) -> Vec<SessionSpaceId> {
    let mut ids = Vec::new();
    for _ in 0..num_ids {
        ids.push(compressor.generate_next_id())
    }
    ids
}

fn finalize_next_range(compressor: &mut IdCompressor) {
    let range = compressor.take_next_range();
    _ = compressor.finalize_range(&range);
}

#[test]
fn test_cluster_spans_reserved_bits() {
    let mut compressor = IdCompressor::new_with_session_id(
        SessionId::from_uuid_string(LEADING_EDGE_OF_VERSION_SESSION_ID).unwrap(),
    );

    let local_first = compressor.generate_next_id();
    assert_eq!(
        local_first.unwrap_uuid_str(&compressor),
        LEADING_EDGE_OF_VERSION_SESSION_ID
    );
    finalize_next_range(&mut compressor);

    // Some eager finals, some locals
    let ids = generate_n_ids(&mut compressor, 10);
    finalize_next_range(&mut compressor);

    let mut uuid_set = HashSet::new();
    for id in &ids {
        uuid_set.insert(id.unwrap_uuid_str(&compressor));
    }
    assert_eq!(uuid_set.len(), ids.len());
    let trailing_uuid = Uuid::try_parse(TRAILING_EDGE_OF_VERSION_SESSION_ID)
        .unwrap()
        .as_u128();
    for uuid_str in &uuid_set {
        let uuid = Uuid::try_parse(uuid_str).unwrap();
        assert!(uuid.as_u128() >= trailing_uuid);
    }
}

#[test]
fn test_complex() {
    let mut compressor = IdCompressor::new();

    _ = compressor.set_cluster_capacity(3);

    // Before first cluster creation
    let session_space_id_1 = compressor.generate_next_id();
    let session_space_id_2 = compressor.generate_next_id();
    assert!(session_space_id_1.is_local());
    assert!(session_space_id_2.is_local());

    // Take initial range
    let out_range = compressor.take_next_range();

    // Finalize initial range
    assert!(compressor.finalize_range(&out_range).is_ok());

    let session_space_id_3 = compressor.generate_next_id();
    let session_space_id_4 = compressor.generate_next_id();
    let session_space_id_5 = compressor.generate_next_id();
    let session_space_id_6 = compressor.generate_next_id();
    let session_space_id_7 = compressor.generate_next_id();

    // 3-5 are within initial cluster capacity (intialized to 3 + 2 capacity)
    assert!(session_space_id_3.is_final());
    assert!(session_space_id_4.is_final());
    assert!(session_space_id_5.is_final());

    // 6 and 7 are outside of initial cluster capacity
    assert!(session_space_id_6.is_local());
    assert!(session_space_id_7.is_local());

    let mut offset: usize = 0;
    let op_space_ids = [0, 1, 2, 3, 4, -6, -7];
    for id in [
        session_space_id_1,
        session_space_id_2,
        session_space_id_3,
        session_space_id_4,
        session_space_id_5,
        session_space_id_6,
        session_space_id_7,
    ] {
        let stable_id = StableId::from(compressor.get_local_session_id()) + offset as u64;
        assert_eq!(compressor.decompress(id).unwrap(), stable_id,);
        assert_eq!(compressor.recompress(stable_id).unwrap(), id);

        let op_space_id = compressor.normalize_to_op_space(id).unwrap();
        assert_eq!(
            id,
            compressor
                .normalize_to_session_space(op_space_id, compressor.get_local_session_id())
                .unwrap()
        );
        if op_space_ids[offset] < 0 {
            assert_eq!(
                op_space_id,
                OpSpaceId::from(LocalId::from_id(op_space_ids[offset]))
            );
        } else {
            assert_eq!(
                op_space_id,
                OpSpaceId::from(FinalId::from_id(op_space_ids[offset] as u64))
            );
        }
        offset += 1;
    }
    // Serialize Deserialize
    let serialized_local = compressor.serialize(true);
    assert!(
        compressor.equals_test_only(&IdCompressor::deserialize(&serialized_local).unwrap(), true)
    );

    let serialized_no_local = compressor.serialize(false);
    assert!(compressor.equals_test_only(
        &IdCompressor::deserialize(&serialized_no_local).unwrap(),
        false
    ));
}

#[test]
fn test_new_with_session_id() {
    let session_id = SessionId::new();
    let compressor = IdCompressor::new_with_session_id(session_id);
    assert_eq!(session_id, compressor.get_local_session_id());
}

#[test]
fn test_cluster_capacity_validation() {
    let mut compressor = IdCompressor::new();
    assert!(compressor.set_cluster_capacity(0).is_err());
    assert!(compressor.set_cluster_capacity(1).is_ok());
    assert!(compressor.set_cluster_capacity(u64::MAX).is_ok())
}

#[test]
fn test_decompress_recompress() {
    let mut compressor = IdCompressor::new();

    let session_space_id = compressor.generate_next_id();

    let stable_id = StableId::from(compressor.get_local_session_id());
    assert_eq!(compressor.decompress(session_space_id).unwrap(), stable_id,);
    assert_eq!(compressor.recompress(stable_id).unwrap(), session_space_id);
}

#[test]
fn test_recompress_invalid() {
    let compressor = IdCompressor::new();
    let foreign_stable = StableId::from(SessionId::new());
    assert!(compressor.recompress(foreign_stable).is_err());
}

#[test]
fn test_finalize_range_ordering() {
    let mut compressor = IdCompressor::new();
    _ = compressor.set_cluster_capacity(3);

    let _ = compressor.generate_next_id();
    let _ = compressor.generate_next_id();
    let out_range = compressor.take_next_range();

    // Finalize the same range twice
    assert!(compressor.finalize_range(&out_range).is_ok());
    assert!(compressor.finalize_range(&out_range).is_err());

    let mut compressor = IdCompressor::new();
    _ = compressor.set_cluster_capacity(3);

    let _ = compressor.generate_next_id();
    let _ = compressor.generate_next_id();
    let out_range_1 = compressor.take_next_range();
    let _ = compressor.generate_next_id();
    let _ = compressor.generate_next_id();
    let out_range_2 = compressor.take_next_range();

    // Finalize ranges out of order
    assert!(compressor.finalize_range(&out_range_2).is_err());
    assert!(compressor.finalize_range(&out_range_1).is_ok());
}

#[test]
fn test_finalize_range_collision() {
    let mut compressor_1 = IdCompressor::new();
    _ = compressor_1.set_cluster_capacity(10);

    let mut compressor_2 = IdCompressor::new_with_session_id(
        Uuid::from(StableId::from(compressor_1.get_local_session_id()) + 3).into(),
    );
    _ = compressor_2.set_cluster_capacity(10);

    _ = compressor_1.generate_next_id();
    let range_1 = compressor_1.take_next_range();
    _ = compressor_1.finalize_range(&range_1);

    _ = compressor_2.generate_next_id();
    let range_2 = compressor_2.take_next_range();
    _ = compressor_2.finalize_range(&range_2);

    assert!(compressor_1.finalize_range(&range_2).is_err());
    assert!(compressor_2.finalize_range(&range_1).is_err());

    _ = compressor_1.generate_next_id();
    let range_1b = compressor_1.take_next_range();
    assert!(compressor_1.finalize_range(&range_1b).is_ok());

    _ = compressor_2.generate_next_id();
    let range_2b = compressor_2.take_next_range();
    assert!(compressor_2.finalize_range(&range_2b).is_ok());
}

#[test]
fn deserialize_and_resume() {
    let mut compressor_1 = IdCompressor::new();
    let mut compressor_2 = IdCompressor::new();
    _ = compressor_1.generate_next_id();
    let out_range = compressor_1.take_next_range();
    _ = compressor_1.finalize_range(&out_range);
    _ = compressor_2.finalize_range(&out_range);
    let serialized_1 = compressor_1.serialize(true);
    let mut compressor_resumed = IdCompressor::deserialize(&serialized_1).ok().unwrap();
    assert!(compressor_resumed.get_local_session_id() == compressor_1.get_local_session_id());
    _ = compressor_resumed.generate_next_id();
    let out_range_2 = compressor_resumed.take_next_range();
    assert!(compressor_resumed.finalize_range(&out_range_2).is_ok())
}
