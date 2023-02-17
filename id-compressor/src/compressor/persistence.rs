use super::{persistence_utils, IdCompressor};
use persistence_utils::Deserializer;

pub(super) const DEFAULT_CLUSTER_CAPACITY: u64 = 512;

enum Version {
    V1 = 1,
}

enum CompressorSerializationError {
    UnknownVersionError,
    UnknownVariantError,
}

impl TryFrom<u32> for Version {
    type Error = CompressorSerializationError;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            x if x == Version::V1 as u32 => Ok(Version::V1),
            _ => Err(CompressorSerializationError::UnknownVersionError),
        }
    }
}

pub(crate) fn deserialize(bytes: &[u8]) -> IdCompressor {
    // Check version
    let mut deserializer = Deserializer::new(bytes);
    let version = deserializer.consume_u32();
    match Version::try_from(version) {
        Ok(Version::V1) => v1::deserialize(&mut deserializer),
        Err(_) => panic!("Unknown serialized compressor version found: {}", version),
    }
}

pub(crate) mod v1 {

    use super::persistence_utils::{
        write_u128_to_vec, write_u32_to_vec, write_u64_to_vec, Deserializer,
    };
    use crate::{
        compressor::tables::{
            session_space::{IdCluster, SessionSpaceRef},
            session_space_normalizer,
        },
        compressor::IdCompressor,
        id_types::{FinalId, LocalId, SessionId, StableId},
    };

    enum Variant {
        Finalized = 1,
        WithLocal = 2,
    }

    impl TryFrom<u8> for Variant {
        type Error = super::CompressorSerializationError;

        fn try_from(value: u8) -> Result<Self, Self::Error> {
            match value {
                x if x == Variant::Finalized as u8 => Ok(Variant::Finalized),
                x if x == Variant::WithLocal as u8 => Ok(Variant::WithLocal),
                _ => Err(super::CompressorSerializationError::UnknownVariantError),
            }
        }
    }

    struct ClusterData {
        pub session_index: u64,
        pub capacity: u64,
        pub count: u64,
    }

    pub(crate) fn serialize(compressor: &IdCompressor) -> Vec<u8> {
        let mut bytes: Vec<u8> = Vec::new();
        serialize_header(&mut bytes, Variant::Finalized);
        serialize_finalized(&mut bytes, compressor);
        bytes
    }

    pub(crate) fn serialize_with_local(compressor: &IdCompressor) -> Vec<u8> {
        let mut bytes: Vec<u8> = Vec::new();
        serialize_header(&mut bytes, Variant::WithLocal);
        // 16 bytes: Session Id
        write_u128_to_vec(
            &mut bytes,
            StableId::from(compressor.session_id).to_uuid_u128(),
        );
        // 8 bytes: generated_id_count
        write_u64_to_vec(&mut bytes, compressor.generated_id_count);
        // 8 bytes: next_range_base as to_generation_count
        write_u64_to_vec(&mut bytes, compressor.next_range_base.to_generation_count());
        let mut normalizer_serialized = session_space_normalizer::persistence::v1::serialize(
            &compressor.session_space_normalizer,
        );
        // Prepends session_space_normalizer serialized length in byte count
        write_u64_to_vec(&mut bytes, normalizer_serialized.len() as u64);
        bytes.append(&mut normalizer_serialized);
        serialize_finalized(&mut bytes, compressor);
        bytes
    }

    fn serialize_header(bytes: &mut Vec<u8>, variant: Variant) {
        // 4 bytes: version number
        write_u32_to_vec(bytes, super::Version::V1 as u32);
        // 1 byte: Variant (finalized, with_local)
        bytes.push(variant as u8);
    }

    fn serialize_finalized(bytes: &mut Vec<u8>, compressor: &IdCompressor) {
        let session_space_ids = compressor
            .sessions
            .get_session_spaces()
            .map(|session_space| session_space.session_id());

        let clusters = compressor
            .final_space
            .get_clusters(&compressor.sessions)
            .map(|id_cluster| ClusterData {
                session_index: id_cluster.session_creator.get_index() as u64,
                capacity: id_cluster.capacity,
                count: id_cluster.count,
            });

        // 8 bytes: Cluster capacity (u64)
        write_u64_to_vec(bytes, compressor.cluster_capacity);
        // 8 bytes: Sessions table count (usize)
        write_u64_to_vec(bytes, compressor.sessions.sessions_count() as u64);
        // n x 16 bytes: Sessions table
        let mut session_count_actual = 0;
        for session_id in session_space_ids {
            write_u128_to_vec(bytes, StableId::from(session_id).to_uuid_u128());
            session_count_actual += 1;
        }
        debug_assert!(
            session_count_actual == compressor.sessions.sessions_count(),
            "Bad sessions count"
        );
        // 8 bytes: Final space table count (usize)
        write_u64_to_vec(bytes, compressor.final_space.cluster_count() as u64);
        // n x 24 bytes: Final Space table as ID Clusters:
        /* {
           8 bytes: session_creator,
           8 bytes: capacity,
           8 bytes: count
        } */
        let mut clusters_count_actual = 0;
        for cluster_data in clusters {
            write_u64_to_vec(bytes, cluster_data.capacity);
            write_u64_to_vec(bytes, cluster_data.count);
            write_u64_to_vec(bytes, cluster_data.session_index);

            clusters_count_actual += 1;
        }
        debug_assert!(
            clusters_count_actual == compressor.final_space.cluster_count(),
            "Bad clusters count"
        );
    }

    pub(super) fn deserialize(deserializer: &mut Deserializer) -> IdCompressor {
        let variant = deserializer.consume_u8();
        let mut compressor = match Variant::try_from(variant) {
            Ok(Variant::Finalized) => IdCompressor::new(),
            Ok(Variant::WithLocal) => {
                let mut compressor = IdCompressor::new_with_session_id(SessionId::from_uuid_u128(
                    deserializer.consume_u128(),
                ));
                // generated_id_count
                compressor.generated_id_count = deserializer.consume_u64();
                // next_range_base, serialized as to_generation_count
                compressor.next_range_base =
                    LocalId::from_generation_count(deserializer.consume_u64());
                let normalizer_byte_length = deserializer.consume_u64() as usize;
                compressor.session_space_normalizer =
                    session_space_normalizer::persistence::v1::deserialize(
                        deserializer,
                        normalizer_byte_length,
                    );
                compressor
            }
            Err(_) => panic!("Unknown serialized compressor variant found: {}", variant),
        };
        compressor.cluster_capacity = deserializer.consume_u64();
        let sessions_table_count = deserializer.consume_u64();
        // n x 16 bytes: sessions table
        for _ in 0..sessions_table_count {
            let session_id_u128 = deserializer.consume_u128();
            compressor
                .sessions
                .create(SessionId::from_uuid_u128(session_id_u128));
        }
        let final_space_table_count = deserializer.consume_u64();
        // n x 24 bytes: Final Space table as ID Clusters:
        for _ in 0..final_space_table_count {
            // 8 bytes: session_creator
            let session_creator_index = deserializer.consume_u64() as usize;
            // 8 bytes: capacity
            let capacity = deserializer.consume_u64();
            // 8 bytes: count
            let count = deserializer.consume_u64();
            // Create IdCluster and add to SessionSpace, final_space, uuid_space
            let base_final_id = match compressor
                .final_space
                .get_tail_cluster(&compressor.sessions)
            {
                Some(cluster) => cluster.base_final_id + cluster.capacity,
                None => FinalId::new(0),
            };
            let session_space_ref = SessionSpaceRef::create_from_index(session_creator_index);
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
                capacity,
                count,
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
}
