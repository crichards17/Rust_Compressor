use super::{persistence_utils::Deserializer, IdCompressor};
use id_types::{errors::ErrorString, SessionId};

pub(super) const DEFAULT_CLUSTER_CAPACITY: u64 = 512;

pub fn deserialize<FMakeSession>(
    bytes: &[u8],
    make_session_id: FMakeSession,
) -> Result<IdCompressor, DeserializationError>
where
    FMakeSession: FnOnce() -> SessionId,
{
    let mut deserializer = Deserializer::new(bytes);
    let version = deserializer.take_u32();
    match version {
        1 => v1::deserialize(&mut deserializer, make_session_id),
        _ => Err(DeserializationError::UnknownVersion),
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum DeserializationError {
    InvalidResumedSession,
    UnknownVersion,
    MalformedInput,
}

impl ErrorString for DeserializationError {
    /// Returns the string representation for the error variant.
    fn to_error_string(&self) -> &str {
        match self {
            DeserializationError::InvalidResumedSession => "Cannot resume existing session.",
            DeserializationError::MalformedInput => "Malformed serialized input.",
            DeserializationError::UnknownVersion => "Unknown deserialization error.",
        }
    }
}

pub mod v1 {
    use super::DeserializationError;
    use crate::{
        compressor::IdCompressor,
        compressor::{
            persistence_utils::{
                write_u128_to_vec, write_u32_to_vec, write_u64_to_vec, Deserializer,
            },
            tables::{
                session_space::{ClusterRef, IdCluster},
                session_space_normalizer::persistence::v1::{
                    deserialize_normalizer, serialize_normalizer,
                },
            },
        },
    };
    use id_types::{
        final_id::{final_id_from_id, get_id_from_final_id},
        session_id::{session_id_from_id_u128, session_id_from_uuid_u128},
        LocalId, SessionId, StableId,
    };

    // Layout
    // version: u32
    // has_local_state: bool as u32
    // clusters_are_32_bit: bool as u32
    // if has_local_state
    //      session_uuid_u128: u128,
    //      generated_id_count: u64,
    //      next_range_base_generation_count: u64,
    //      persistent_normalizer: PersistenceNormalizer,
    // cluster_capacity: u64,
    // session_uuid_u128s: Vec<u128>,
    // cluster_data: Vec<(session_index: u64, capacity: u64, count: u64)>,

    pub fn serialize(compressor: &IdCompressor) -> Vec<u8> {
        let mut bytes: Vec<u8> = Vec::new();
        let is_32_bit = serialize_header(compressor, false, &mut bytes);
        serialize_finalized(compressor, is_32_bit, &mut bytes);
        bytes
    }

    pub fn serialize_with_local(compressor: &IdCompressor) -> Vec<u8> {
        let mut bytes: Vec<u8> = Vec::new();
        let is_32_bit = serialize_header(compressor, true, &mut bytes);
        write_u128_to_vec(&mut bytes, StableId::from(compressor.session_id).into());
        write_u64_to_vec(&mut bytes, compressor.generated_id_count);
        write_u64_to_vec(&mut bytes, compressor.next_range_base_generation_count);
        serialize_normalizer(&compressor.session_space_normalizer, &mut bytes);
        serialize_finalized(compressor, is_32_bit, &mut bytes);
        bytes
    }

    fn serialize_header(compressor: &IdCompressor, is_local: bool, bytes: &mut Vec<u8>) -> bool {
        // Version
        write_u32_to_vec(bytes, 1);
        write_u32_to_vec(bytes, is_local as u32);
        let is_32_bit = match compressor
            .final_space
            .get_tail_cluster(&compressor.sessions)
        {
            Some(cluster) => get_id_from_final_id(cluster.max_allocated_final()),
            None => 0,
        } < u32::MAX as u64;
        write_u32_to_vec(bytes, is_32_bit as u32);
        is_32_bit
    }

    fn serialize_finalized(compressor: &IdCompressor, is_32_bit: bool, bytes: &mut Vec<u8>) {
        let write_cluster: fn(cluster: &IdCluster, cluster_ref: ClusterRef, bytes: &mut Vec<u8>) =
            if is_32_bit {
                |cluster, cluster_ref, bytes| {
                    write_u32_to_vec(
                        bytes,
                        cluster_ref.get_session_space_ref().get_index() as u32,
                    );
                    write_u32_to_vec(bytes, cluster.capacity as u32);
                    write_u32_to_vec(bytes, cluster.count as u32);
                }
            } else {
                |cluster, cluster_ref, bytes| {
                    write_u64_to_vec(
                        bytes,
                        cluster_ref.get_session_space_ref().get_index() as u64,
                    );
                    write_u64_to_vec(bytes, cluster.capacity);
                    write_u64_to_vec(bytes, cluster.count);
                }
            };

        write_u64_to_vec(bytes, compressor.cluster_capacity);
        write_u64_to_vec(bytes, compressor.sessions.get_session_count() as u64);

        bytes.extend_from_slice(compressor.sessions.get_session_id_slice());

        write_u64_to_vec(bytes, compressor.final_space.get_cluster_count() as u64);
        compressor
            .final_space
            .get_clusters(&compressor.sessions)
            .for_each(|(id_cluster, cluster_ref)| {
                write_cluster(id_cluster, cluster_ref, bytes);
            });
    }

    pub(super) fn deserialize<FMakeSession>(
        deserializer: &mut Deserializer,
        make_session_id: FMakeSession,
    ) -> Result<IdCompressor, DeserializationError>
    where
        FMakeSession: FnOnce() -> SessionId,
    {
        let with_local_state = deserializer.take_u32() != 0;
        let is_32_bit = deserializer.take_u32() != 0;
        let mut compressor = match with_local_state {
            false => IdCompressor::new_with_session_id(make_session_id()),
            true => {
                let session_uuid_u128 = deserializer.take_u128();
                let mut compressor =
                    IdCompressor::new_with_session_id(session_id_from_uuid_u128(session_uuid_u128));
                compressor.generated_id_count = deserializer.take_u64();
                compressor.next_range_base_generation_count = deserializer.take_u64();
                compressor.session_space_normalizer = deserialize_normalizer(deserializer);
                compressor
            }
        };

        compressor.cluster_capacity = deserializer.take_u64();
        let session_count = deserializer.take_u64();
        let mut session_ref_remap = Vec::new();
        for _ in 0..session_count {
            let session_uuid_u128 = deserializer.take_u128();
            let session_id = session_id_from_id_u128(session_uuid_u128);
            if !with_local_state && session_id == compressor.session_id {
                return Err(DeserializationError::InvalidResumedSession);
            }
            session_ref_remap.push(compressor.sessions.get_or_create(session_id));
        }

        let read_cluster: fn(deserializer: &mut Deserializer) -> (u64, u64, u64) = if is_32_bit {
            |deser| {
                (
                    deser.take_u32() as u64,
                    deser.take_u32() as u64,
                    deser.take_u32() as u64,
                )
            }
        } else {
            |deser| (deser.take_u64(), deser.take_u64(), deser.take_u64())
        };

        let cluster_count = deserializer.take_u64();
        for _ in 0..cluster_count {
            let (session_index, capacity, count) = read_cluster(deserializer);

            let base_final_id = match compressor
                .final_space
                .get_tail_cluster(&compressor.sessions)
            {
                Some(cluster) => cluster.base_final_id + cluster.capacity,
                None => final_id_from_id(0),
            };
            let session_space_ref = session_ref_remap[session_index as usize];
            let session_space = compressor.sessions.deref_session_space(session_space_ref);
            let base_local_id = match session_space.get_tail_cluster() {
                Some(cluster) => cluster.base_local_id - cluster.capacity,
                None => LocalId::from_id(-1),
            };
            let new_cluster = IdCluster {
                base_final_id,
                base_local_id,
                capacity,
                count,
            };
            let new_cluster_ref = compressor
                .sessions
                .deref_session_space_mut(session_space_ref)
                .add_cluster(session_space_ref, new_cluster);
            compressor
                .final_space
                .add_cluster(new_cluster_ref, &compressor.sessions);
        }
        compressor.final_id_limit = match compressor
            .final_space
            .get_tail_cluster(&compressor.sessions)
        {
            Some(cluster) => cluster.base_final_id + cluster.count,
            None => final_id_from_id(0),
        };
        Ok(compressor)
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn assert_local_id_alignment() {
            assert_eq!(LocalId::from_id(-1).to_generation_count(), 1);
        }
    }
}
