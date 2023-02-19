use super::IdCompressor;
use postcard::from_bytes;
use serde::{Deserialize, Serialize};

pub(super) const DEFAULT_CLUSTER_CAPACITY: u64 = 512;

pub(crate) fn deserialize(bytes: &[u8]) -> Result<IdCompressor, DeserializationError> {
    let versioned_persistent_compressor: VersionedPersistentCompressor = match from_bytes(bytes) {
        Ok(result) => result,
        Err(e) => return Err(DeserializationError::PostcardError(e)),
    };
    match versioned_persistent_compressor {
        VersionedPersistentCompressor::V1(persistent_compressor) => {
            Ok(v1::deserialize(persistent_compressor))
        }
        _ => return Err(DeserializationError::UnknownError),
    }
}

#[derive(Deserialize, Serialize)]
enum VersionedPersistentCompressor {
    V1(v1::PersistentCompressor),
}

#[derive(Debug)]
pub enum DeserializationError {
    PostcardError(postcard::Error),
    UnknownError,
}

pub(crate) mod v1 {

    use crate::{
        compressor::tables::{
            session_space::{IdCluster, SessionSpaceRef},
            session_space_normalizer::persistence::v1::{
                get_normalizer_from_persistent, get_persistent_normalizer, PersistenceNormalizer,
            },
        },
        compressor::IdCompressor,
        id_types::{FinalId, LocalId, SessionId, StableId},
    };
    use postcard::to_allocvec;
    use serde::{Deserialize, Serialize};

    #[derive(Deserialize, Serialize)]
    pub(super) struct PersistentCompressor {
        local_state: Option<LocalState>,
        cluster_capacity: u64,
        session_uuid_u128s: Vec<u128>,
        cluster_data: Vec<ClusterData>,
    }

    #[derive(Deserialize, Serialize)]
    struct LocalState {
        session_uuid_u128: u128,
        generated_id_count: u64,
        next_range_base: LocalId,
        persistent_normalizer: PersistenceNormalizer,
    }

    #[derive(Deserialize, Serialize)]
    struct ClusterData {
        pub session_index: u64,
        pub capacity: u64,
        pub count: u64,
    }

    pub(crate) fn serialize(compressor: &IdCompressor) -> Vec<u8> {
        let versioned_persistent_compressor =
            super::VersionedPersistentCompressor::V1(serialize_finalized(compressor));
        to_allocvec(&versioned_persistent_compressor).unwrap()
    }

    pub(crate) fn serialize_with_local(compressor: &IdCompressor) -> Vec<u8> {
        let local_state = LocalState {
            session_uuid_u128: StableId::from(compressor.session_id).to_uuid_u128(),
            generated_id_count: compressor.generated_id_count,
            next_range_base: compressor.next_range_base,
            persistent_normalizer: get_persistent_normalizer(&compressor.session_space_normalizer),
        };

        let mut persistent_compressor = serialize_finalized(compressor);
        persistent_compressor.local_state = Some(local_state);
        let versioned_persistent_compressor =
            super::VersionedPersistentCompressor::V1(persistent_compressor);
        to_allocvec(&versioned_persistent_compressor).unwrap()
    }

    fn serialize_finalized(compressor: &IdCompressor) -> PersistentCompressor {
        let session_uuid_u128s: Vec<u128> = compressor
            .sessions
            .get_session_spaces()
            .map(|session_space| StableId::from(session_space.session_id()).to_uuid_u128())
            .collect();

        let cluster_data: Vec<ClusterData> = compressor
            .final_space
            .get_clusters(&compressor.sessions)
            .map(|id_cluster| ClusterData {
                session_index: id_cluster.session_creator.get_index() as u64,
                capacity: id_cluster.capacity,
                count: id_cluster.count,
            })
            .collect();

        PersistentCompressor {
            local_state: None,
            session_uuid_u128s,
            cluster_capacity: compressor.cluster_capacity,
            cluster_data,
        }
    }

    pub(super) fn deserialize(persistent_compressor: PersistentCompressor) -> IdCompressor {
        let mut compressor = match persistent_compressor.local_state {
            None => IdCompressor::new(),
            Some(local_state) => {
                let mut compressor = IdCompressor::new_with_session_id(SessionId::from_uuid_u128(
                    local_state.session_uuid_u128,
                ));
                compressor.generated_id_count = local_state.generated_id_count;
                compressor.next_range_base = local_state.next_range_base;
                compressor.session_space_normalizer =
                    get_normalizer_from_persistent(local_state.persistent_normalizer);
                compressor
            }
        };
        compressor.cluster_capacity = persistent_compressor.cluster_capacity;
        for session_uuid_u128 in persistent_compressor.session_uuid_u128s {
            compressor
                .sessions
                .get_or_create(SessionId::from_uuid_u128(session_uuid_u128));
        }

        for cluster_data in persistent_compressor.cluster_data {
            let base_final_id = match compressor
                .final_space
                .get_tail_cluster(&compressor.sessions)
            {
                Some(cluster) => cluster.base_final_id + cluster.capacity,
                None => FinalId::new(0),
            };
            let session_space_ref =
                SessionSpaceRef::create_from_index(cluster_data.session_index as usize);
            let session_space = compressor.sessions.deref_session_space(session_space_ref);
            let base_local_id = match session_space.get_tail_cluster() {
                Some(cluster_ref) => {
                    let cluster = compressor.sessions.deref_cluster(cluster_ref);
                    cluster.base_local_id - cluster.capacity
                }
                None => LocalId::new(-1),
            };
            let new_cluster = IdCluster {
                session_creator: session_space_ref,
                base_final_id,
                base_local_id,
                capacity: cluster_data.capacity,
                count: cluster_data.count,
            };
            let new_cluster_ref = compressor
                .sessions
                .deref_session_space_mut(session_space_ref)
                .add_cluster(new_cluster);
            compressor
                .final_space
                .add_cluster(new_cluster_ref, &compressor.sessions);
            let session_space = compressor.sessions.deref_session_space(session_space_ref);
            compressor.uuid_space.add_cluster(
                session_space.session_id(),
                new_cluster_ref,
                &compressor.sessions,
            );
        }
        compressor
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn assert_local_id_alignment() {
            assert_eq!(LocalId::new(-1).to_generation_count(), 1);
        }
    }
}
