mod compressor_test_utils;

use compressor_test_utils::serialize_roundtrip;
use distributed_id_allocator::compressor::*;
use id_types::*;
use std::collections::{BTreeSet, HashSet};

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

#[test]
fn test_detects_invalid_cluster_capacities() {
    let mut compressor_1 = IdCompressor::new();
    assert!(matches!(
        compressor_1.set_cluster_capacity(0).unwrap_err(),
        ClusterCapacityError::InvalidClusterCapacity
    ));
    assert!(compressor_1.set_cluster_capacity(1).is_ok());
    assert!(compressor_1.set_cluster_capacity(u64::MAX).is_ok());
}

#[test]
fn test_new_with_session_id() {
    let session_id = SessionId::new();
    let compressor = IdCompressor::new_with_session_id(session_id);
    assert_eq!(session_id, compressor.get_local_session_id());
}

#[test]
fn test_manual_id_creation() {
    let mut compressor = IdCompressor::new();
    let id = compressor.generate_next_id();
    assert!(compressor.decompress(id).is_ok());
    let uuid = compressor.decompress(id).unwrap();
    assert!(compressor.recompress(uuid).is_ok());
    assert_eq!(id, compressor.recompress(uuid).unwrap());
}

#[test]
fn test_eager_final_allocation() {
    let mut compressor = IdCompressor::new();
    _ = compressor.set_cluster_capacity(3);
    assert!(compressor.generate_next_id().is_local());
    assert!(compressor.generate_next_id().is_local());
    let range_1 = compressor.take_next_range();
    _ = compressor.finalize_range(&range_1);
    assert!(compressor.generate_next_id().is_final());
    assert!(compressor.generate_next_id().is_final());
    assert!(compressor.generate_next_id().is_final());
    assert!(compressor.generate_next_id().is_local());
}

#[test]
fn test_eager_final_normalization() {
    let mut compressor = IdCompressor::new();
    _ = compressor.set_cluster_capacity(3);
    _ = compressor.generate_next_id();
    let range_1 = compressor.take_next_range();
    _ = compressor.finalize_range(&range_1);
    let final_2 = compressor.generate_next_id();

    assert!(compressor.normalize_to_op_space(final_2).is_ok());
    let op_space_2 = compressor.normalize_to_op_space(final_2).unwrap();
    assert!(op_space_2.is_final());
    assert!(compressor
        .normalize_to_session_space(op_space_2, compressor.get_local_session_id())
        .is_ok());
    let session_space_2 = compressor
        .normalize_to_session_space(op_space_2, compressor.get_local_session_id())
        .unwrap();
    assert!(session_space_2.is_final());
    assert_eq!(final_2, session_space_2);
}

#[test]
fn test_eager_finals_with_outstanding_locals() {
    let mut compressor = IdCompressor::new();
    _ = compressor.set_cluster_capacity(2);

    // Before cluster expansion
    assert!(compressor.generate_next_id().is_local());
    let range_a = compressor.take_next_range();
    _ = compressor.finalize_range(&range_a);
    assert!(compressor.generate_next_id().is_final());
    assert!(compressor.generate_next_id().is_final());

    //After cluster expansion
    assert!(compressor.generate_next_id().is_local());
    let range_b = compressor.take_next_range();
    let local_id = compressor.generate_next_id();
    assert!(local_id.is_local());

    // Finalizing range B should associate this range with finals
    let range_c = compressor.take_next_range();

    _ = compressor.finalize_range(&range_b);
    let eager_id = compressor.generate_next_id();
    assert!(eager_id.is_final());

    assert_eq!(
        local_id,
        compressor
            .recompress(compressor.decompress(local_id).unwrap())
            .unwrap()
    );
    assert_eq!(
        eager_id,
        compressor
            .recompress(compressor.decompress(eager_id).unwrap())
            .unwrap()
    );

    _ = compressor.finalize_range(&range_c);
    assert_eq!(
        local_id,
        compressor
            .recompress(compressor.decompress(local_id).unwrap())
            .unwrap()
    );
    assert_eq!(
        eager_id,
        compressor
            .recompress(compressor.decompress(eager_id).unwrap())
            .unwrap()
    );
}

#[test]
fn test_unique_eager_finals_with_multiple_outstanding_ranges() {
    let mut compressor = IdCompressor::new();
    _ = compressor.set_cluster_capacity(10);
    let mut generated_ids: Vec<SessionSpaceId> = Vec::new();

    // Make a first outstanding range
    let id_a_1 = compressor.generate_next_id();
    let id_a_2 = compressor.generate_next_id();
    assert!(id_a_1.is_local());
    assert!(id_a_2.is_local());
    generated_ids.push(id_a_1);
    generated_ids.push(id_a_2);
    let range_a = compressor.take_next_range();

    // Make a second outstanding range
    let id_b_1 = compressor.generate_next_id();
    let id_b_2 = compressor.generate_next_id();
    assert!(id_b_1.is_local());
    assert!(id_b_2.is_local());
    generated_ids.push(id_b_1);
    generated_ids.push(id_b_2);
    let range_b = compressor.take_next_range();

    // Finalize the first range, which should create finals that align with both outstanding ranges
    _ = compressor.finalize_range(&range_a);

    // Make a third range. This one should be composed of eager finals that align after the two ranges above.
    let id_c_1 = compressor.generate_next_id();
    let id_c_2 = compressor.generate_next_id();
    assert!(id_c_1.is_final());
    assert!(id_c_2.is_final());
    generated_ids.push(id_c_1);
    generated_ids.push(id_c_2);
    let range_c = compressor.take_next_range();

    // Finalize both outstanding ranges.
    _ = compressor.finalize_range(&range_b);
    _ = compressor.finalize_range(&range_c);

    // Make some more eager finals that should be aligned correctly.
    let id_d_1 = compressor.generate_next_id();
    let id_d_2 = compressor.generate_next_id();
    assert!(id_d_1.is_final());
    assert!(id_d_2.is_final());
    generated_ids.push(id_d_1);
    generated_ids.push(id_d_2);

    // Assert everything is unique and consistent.
    let mut session_space_ids: BTreeSet<SessionSpaceId> = BTreeSet::new();
    let mut stable_ids: BTreeSet<StableId> = BTreeSet::new();
    for id in generated_ids {
        session_space_ids.insert(id);
        stable_ids.insert(compressor.decompress(id).unwrap());
    }
    assert_eq!(session_space_ids.len(), 8);
    assert_eq!(stable_ids.len(), 8);
}

#[test]
fn test_unique_eager_finals_with_outstanding_locals_after_expansion() {
    let mut compressor = IdCompressor::new();
    _ = compressor.set_cluster_capacity(2);

    // Make locals to fill half of the future cluster
    let id_a_1 = compressor.generate_next_id();
    let id_a_2 = compressor.generate_next_id();
    assert!(id_a_1.is_local());
    assert!(id_a_2.is_local());
    let range_a = compressor.take_next_range();

    // Make locals to overflow the future cluster
    let id_b_1 = compressor.generate_next_id();
    let id_b_2 = compressor.generate_next_id();
    let id_b_3 = compressor.generate_next_id();
    assert!(id_b_1.is_local());
    assert!(id_b_2.is_local());
    assert!(id_b_3.is_local());
    let range_b = compressor.take_next_range();

    // Finalize the first range. This should align the first four locals (i.e. all of range1, and 2/3 of range2)
    _ = compressor.finalize_range(&range_a);
    assert!(compressor.normalize_to_op_space(id_b_2).unwrap().is_final());
    assert!(compressor.normalize_to_op_space(id_b_3).unwrap().is_local());

    // Make a single range that should still be overflowing the initial cluster (i.e. be local)
    let id_c_1 = compressor.generate_next_id();
    assert!(id_c_1.is_local());
    let range_c = compressor.take_next_range();

    // Second finalize should expand the cluster and align all outstanding ranges.
    _ = compressor.finalize_range(&range_b);

    // All generated IDs should have aligned finals (even though range_c has not been finalized)
    let mut generated_ids = vec![id_a_1, id_a_2, id_b_1, id_b_2, id_b_3, id_c_1];
    for id in &generated_ids {
        assert!(compressor.normalize_to_op_space(*id).unwrap().is_final());
    }

    _ = compressor.finalize_range(&range_c);

    // Make one eager final
    let id_d_1 = compressor.generate_next_id();
    assert!(id_d_1.is_final());
    generated_ids.push(id_d_1);

    // Assert everything is unique and consistent.
    let mut session_space_ids: BTreeSet<SessionSpaceId> = BTreeSet::new();
    let mut stable_ids: BTreeSet<StableId> = BTreeSet::new();
    for id in generated_ids {
        session_space_ids.insert(id);
        stable_ids.insert(compressor.decompress(id).unwrap());
    }
    assert_eq!(session_space_ids.len(), 7);
    assert_eq!(stable_ids.len(), 7);
}

#[test]
fn test_id_range_creation() {
    const CLUSTER_CAPACITY: u64 = 5;
    let mut compressor = IdCompressor::new();
    _ = compressor.set_cluster_capacity(CLUSTER_CAPACITY);

    let range_counts = [0, 1, CLUSTER_CAPACITY * 2];
    for count in range_counts {
        generate_n_ids(&mut compressor, count as i32);
        let id_range = compressor.take_next_range();
        match id_range.range {
            Some(range) => {
                assert_eq!(count, range.1);
            }
            None => {
                assert_eq!(count, 0);
            }
        }
    }
}

#[test]
fn test_prevent_finalizing_ranges_twice() {
    let mut compressor = IdCompressor::new();
    generate_n_ids(&mut compressor, 5);
    let range_a = compressor.take_next_range();
    _ = compressor.finalize_range(&range_a);
    assert!(matches!(
        compressor.finalize_range(&range_a).unwrap_err(),
        FinalizationError::RangeFinalizedOutOfOrder
    ));
}

#[test]
fn test_prevent_finalizing_ranges_out_of_order() {
    let mut compressor = IdCompressor::new();
    generate_n_ids(&mut compressor, 5);
    let _range_a = compressor.take_next_range();
    generate_n_ids(&mut compressor, 5);
    let range_b = compressor.take_next_range();
    assert!(matches!(
        compressor.finalize_range(&range_b).unwrap_err(),
        FinalizationError::RangeFinalizedOutOfOrder
    ));
}

#[test]
fn test_finalize_to_clusters_of_varying_size() {
    for i in 1..5 {
        for j in 0..=i {
            let mut compressor = IdCompressor::new();
            _ = compressor.set_cluster_capacity(i);
            let mut session_space_ids: BTreeSet<SessionSpaceId> = BTreeSet::new();
            for _ in 0..=j {
                session_space_ids.insert(compressor.generate_next_id());
            }
            let range = compressor.take_next_range();
            _ = compressor.finalize_range(&range);
            let mut op_space_ids: BTreeSet<OpSpaceId> = BTreeSet::new();
            for session_space_id in &session_space_ids {
                op_space_ids.insert(compressor.normalize_to_op_space(*session_space_id).unwrap());
            }
            assert_eq!(session_space_ids.len(), op_space_ids.len());
            for op_space_id in op_space_ids {
                assert!(op_space_id.is_final());
            }
        }
    }
}

#[test]
fn test_recompress_own_stable_id() {
    let mut compressor = IdCompressor::new();
    let session_space_id = compressor.generate_next_id();
    let stable_id = compressor.decompress(session_space_id).unwrap();
    assert_eq!(compressor.recompress(stable_id).unwrap(), session_space_id);
    finalize_next_range(&mut compressor);
    assert_eq!(compressor.recompress(stable_id).unwrap(), session_space_id);
}

#[test]
fn test_recompress_foreign_stable_id() {
    let mut compressor_1 = IdCompressor::new();
    let id_1 = compressor_1.generate_next_id();
    let stable_id_1 = compressor_1.decompress(id_1).unwrap();

    let mut compressor_2 = IdCompressor::new();
    let range_1 = compressor_1.take_next_range();
    _ = compressor_2.finalize_range(&range_1);
    assert!(compressor_2.recompress(stable_id_1).is_ok());
    assert!(compressor_2.recompress(stable_id_1).unwrap().is_final());
}

#[test]
fn test_prevents_recompressing_unknown_stable_ids() {
    let compressor =
        IdCompressor::new_with_session_id(SessionId::from_uuid_string(_STABLE_IDS[0]).unwrap());
    assert!(matches!(
        compressor
            .recompress(StableId::from(
                Uuid::try_parse("5fff846a-efd4-42fb-8b78-b32ce2672f99").unwrap()
            ))
            .unwrap_err(),
        RecompressionError::UnallocatedStableId
    ));
}

#[test]
fn test_recompress_unfinalized_eager_final() {
    let mut compressor = IdCompressor::new();
    _ = compressor.set_cluster_capacity(5);
    _ = compressor.generate_next_id();
    finalize_next_range(&mut compressor);
    let id_2 = compressor.generate_next_id();
    assert!(id_2.is_final());
    let stable_id = StableId::from(compressor.get_local_session_id()) + 1;
    assert!(compressor.recompress(stable_id).is_ok());
    assert_eq!(compressor.recompress(stable_id).unwrap(), id_2);
}

#[test]
fn test_decompress_unknown_id() {
    let compressor = IdCompressor::new();
    assert!(matches!(
        compressor
            .decompress(SessionSpaceId::from_id(-2))
            .unwrap_err(),
        DecompressionError::UnobtainableId
    ));
    assert!(matches!(
        compressor
            .decompress(SessionSpaceId::from_id(0))
            .unwrap_err(),
        DecompressionError::UnallocatedFinalId
    ));
}

#[test]
fn test_decompress_local_before_and_after_finalizing() {
    let mut compressor = IdCompressor::new();
    let session_space_id = compressor.generate_next_id();
    assert!(compressor.decompress(session_space_id).is_ok());
    let stable_id = compressor.decompress(session_space_id).unwrap();
    finalize_next_range(&mut compressor);
    assert!(compressor.decompress(session_space_id).is_ok());
    assert_eq!(compressor.decompress(session_space_id).unwrap(), stable_id);
}

#[test]
fn test_decompress_final_id() {
    let mut compressor = IdCompressor::new();
    let session_space_id = compressor.generate_next_id();
    finalize_next_range(&mut compressor);
    let op_space_id = compressor.normalize_to_op_space(session_space_id).unwrap();
    assert!(op_space_id.is_final());
    assert!(compressor
        .decompress(
            compressor
                .normalize_to_session_space(op_space_id, compressor.get_local_session_id())
                .unwrap()
        )
        .is_ok());
}

#[test]
fn test_decompress_unfinalized_eager_final() {
    let mut compressor = IdCompressor::new();
    _ = compressor.set_cluster_capacity(5);
    generate_n_ids(&mut compressor, 1);
    finalize_next_range(&mut compressor);
    let eager_final = compressor.generate_next_id();
    assert!(eager_final.is_final());
    let stable_equivalent = StableId::from(compressor.get_local_session_id()) + 1;
    assert!(compressor.decompress(eager_final).is_ok());
    assert_eq!(
        compressor.decompress(eager_final).unwrap(),
        stable_equivalent
    );
}

#[test]
fn test_normalize_unfinalized_local_to_op_space() {
    let mut compressor = IdCompressor::new();
    let session_space_local = compressor.generate_next_id();
    assert!(compressor
        .normalize_to_op_space(session_space_local)
        .is_ok());
    let normalized_id = compressor
        .normalize_to_op_space(session_space_local)
        .unwrap();
    assert!(normalized_id.is_local());
    assert_eq!(normalized_id.id(), session_space_local.id());
}

#[test]
fn test_normalize_finalized_local_to_op_space() {
    let mut compressor = IdCompressor::new();
    let session_space_local = compressor.generate_next_id();
    finalize_next_range(&mut compressor);
    assert!(compressor
        .normalize_to_op_space(session_space_local)
        .is_ok());
    let normalized_id = compressor
        .normalize_to_op_space(session_space_local)
        .unwrap();
    assert!(normalized_id.is_final());
    assert_ne!(normalized_id.id(), session_space_local.id());
}

#[test]
fn test_normalize_eager_final_to_op_space() {
    let mut compressor = IdCompressor::new();
    _ = compressor.set_cluster_capacity(5);
    generate_n_ids(&mut compressor, 1);
    finalize_next_range(&mut compressor);
    let eager_final = compressor.generate_next_id();
    assert!(compressor.normalize_to_op_space(eager_final).is_ok());
    let normalized_id = compressor.normalize_to_op_space(eager_final).unwrap();
    assert!(normalized_id.is_final());
    assert_eq!(normalized_id.id(), eager_final.id());
    assert!(compressor
        .normalize_to_session_space(normalized_id, compressor.get_local_session_id())
        .is_ok());
    assert_eq!(
        compressor
            .normalize_to_session_space(normalized_id, compressor.get_local_session_id())
            .unwrap(),
        eager_final
    );
}

#[test]
fn test_prevents_normalizing_unfinalized_foreign_id_to_session_space() {
    let mut compressor_a = IdCompressor::new();
    let mut compressor_b = IdCompressor::new();

    // Attempt to finalize a foreign local for an unknown session (no ranges finalized)
    let session_space_id_a_1 = compressor_a.generate_next_id();
    let op_space_id_a_1 = compressor_a
        .normalize_to_op_space(session_space_id_a_1)
        .unwrap();
    assert!(op_space_id_a_1.is_local());
    assert!(matches!(
        compressor_b
            .normalize_to_session_space(op_space_id_a_1, compressor_a.get_local_session_id())
            .unwrap_err(),
        NormalizationError::UnknownSessionId
    ));

    // Attempt to finalize an unfinalized foreign local for a known session
    let range_a_1 = compressor_a.take_next_range();
    assert!(compressor_b.finalize_range(&range_a_1).is_ok());
    let session_space_id_a_2 = compressor_a.generate_next_id();
    let op_space_id_a_2 = compressor_a
        .normalize_to_op_space(session_space_id_a_2)
        .unwrap();
    assert!(op_space_id_a_2.is_local());
    assert!(matches!(
        compressor_b
            .normalize_to_session_space(op_space_id_a_2, compressor_a.get_local_session_id())
            .unwrap_err(),
        NormalizationError::UnfinalizedForeignLocal
    ));

    // Attempt to finalize an unfinalized foreign final for a known session
    _ = compressor_a.finalize_range(&range_a_1);
    let session_space_id_a_3 = compressor_a.generate_next_id();
    assert!(session_space_id_a_3.is_final());
    let op_space_id_a_3 = compressor_a
        .normalize_to_op_space(session_space_id_a_3)
        .unwrap();
    assert!(matches!(
        compressor_b
            .normalize_to_session_space(op_space_id_a_3, compressor_a.get_local_session_id())
            .unwrap_err(),
        NormalizationError::UnFinalizedForeignFinal
    ));
}

#[test]
fn test_normalize_foreign_ids_to_session_space() {
    let mut compressor_a = IdCompressor::new();
    let mut compressor_b = IdCompressor::new();

    let session_space_id = compressor_a.generate_next_id();
    let op_space_local = compressor_a
        .normalize_to_op_space(session_space_id)
        .unwrap();
    assert!(op_space_local.is_local());
    let range_a = compressor_a.take_next_range();
    _ = compressor_a.finalize_range(&range_a);
    let op_space_final = compressor_a
        .normalize_to_op_space(session_space_id)
        .unwrap();
    assert!(op_space_final.is_final());
    _ = compressor_b.finalize_range(&range_a);

    assert!(compressor_b
        .normalize_to_session_space(op_space_local, compressor_a.get_local_session_id())
        .is_ok());
    assert_eq!(
        compressor_b
            .normalize_to_session_space(op_space_local, compressor_a.get_local_session_id())
            .unwrap()
            .id(),
        op_space_final.id()
    );
    assert!(compressor_b
        .normalize_to_session_space(op_space_final, compressor_a.get_local_session_id())
        .is_ok());
    assert_eq!(
        compressor_b
            .normalize_to_session_space(op_space_final, compressor_a.get_local_session_id())
            .unwrap()
            .id(),
        op_space_final.id()
    );
}

#[test]
fn test_normalize_own_final_from_foreign_session() {
    // Regression test for the situation in which a client creates a final ID and another client references
    // that final ID in a message back to the creating client. The creating client will normalize it and
    // pass the session ID of the remote (non-creating) client. This should be handled correctly.
    let mut compressor_a = IdCompressor::new();
    let mut compressor_b = IdCompressor::new();
    _ = compressor_a.set_cluster_capacity(5);
    _ = compressor_b.set_cluster_capacity(5);

    let id_a = compressor_a.generate_next_id();
    let range_a = compressor_a.take_next_range();
    _ = compressor_a.finalize_range(&range_a);
    _ = compressor_b.finalize_range(&range_a);

    let foreign_id_to_op_space = compressor_b
        .normalize_to_op_space(
            compressor_b
                .normalize_to_session_space(
                    compressor_a.normalize_to_op_space(id_a).unwrap(),
                    compressor_a.get_local_session_id(),
                )
                .unwrap(),
        )
        .unwrap();

    let normalized_to_originating_session = compressor_a
        .normalize_to_session_space(foreign_id_to_op_space, compressor_b.get_local_session_id())
        .unwrap();

    assert_eq!(normalized_to_originating_session, id_a);
}

#[test]
fn test_serialize_empty_compressor() {
    let compressor = IdCompressor::new();
    _ = serialize_roundtrip(&compressor);
}

#[test]
fn test_serialize_and_resume() {
    let mut compressor_a = IdCompressor::new();
    let mut compressor_b = IdCompressor::new();
    compressor_a.generate_next_id();
    let range_1 = compressor_a.take_next_range();
    _ = compressor_a.finalize_range(&range_1);
    _ = compressor_b.finalize_range(&range_1);

    let compressor_test_utils::RoundtrippedCompressors {
        with_local: mut resumed_with_local,
        without_local: _,
    } = serialize_roundtrip(&compressor_a);
    _ = resumed_with_local.generate_next_id();
    let range_2 = resumed_with_local.take_next_range();
    _ = resumed_with_local.finalize_range(&range_2);
    assert!(compressor_b.finalize_range(&range_2).is_ok());

    let compressor_test_utils::RoundtrippedCompressors {
        with_local: _,
        without_local: roundtripped_without_local_a,
    } = serialize_roundtrip(&resumed_with_local);
    let compressor_test_utils::RoundtrippedCompressors {
        with_local: _,
        without_local: roundtripped_without_local_b,
    } = serialize_roundtrip(&compressor_b);

    assert!(roundtripped_without_local_a.equals_test_only(&roundtripped_without_local_b, false));
}

#[test]
fn test_cluster_collision_detection() {
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