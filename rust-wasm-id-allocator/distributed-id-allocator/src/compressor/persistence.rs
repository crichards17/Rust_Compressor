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
    let deserializer = Deserializer::new(&bytes);
    let (version, deserializer) = deserializer.take_one(u64::from_le_bytes);
    match version {
        1 => v1::deserialize(deserializer, make_session_id),
        _ => Err(DeserializationError::UnknownVersion),
    }
}

#[derive(Debug)]
pub enum DeserializationError {
    InvalidResumedSession,
    UnknownVersion,
}

impl ErrorString for DeserializationError {
    /// Returns the string representation for the error variant.
    fn to_error_string(&self) -> &str {
        match self {
            DeserializationError::InvalidResumedSession => "Cannot resume existing session.",
            DeserializationError::UnknownVersion => "Unknown deserialization error.",
        }
    }
}

pub mod v1 {
    use super::DeserializationError;
    use crate::{
        compressor::IdCompressor,
        compressor::{
            persistence_utils::{write_u128_to_vec, write_u64_to_vec, Deserializer},
            tables::{
                session_space::IdCluster,
                session_space_normalizer::persistence::v1::{
                    deserialize_normalizer, serialize_normalizer,
                },
            },
        },
    };
    use id_types::{FinalId, LocalId, SessionId, StableId};

    // Layout
    // has_local_state: bool as u64
    //      session_uuid_u128: u128,
    //      generated_id_count: u64,
    //      next_range_base_generation_count: u64,
    //      persistent_normalizer: PersistenceNormalizer,
    // cluster_capacity: u64,
    // session_uuid_u128s: Vec<u128>,
    // cluster_data: Vec<(session_index: u64, capacity: u64, count: u64)>,

    pub fn serialize(compressor: &IdCompressor) -> Vec<u8> {
        let mut bytes: Vec<u8> = Vec::new();
        write_u64_to_vec(&mut bytes, false as u64);
        serialize_finalized(compressor, &mut bytes);
        bytes
    }

    pub fn serialize_with_local(compressor: &IdCompressor) -> Vec<u8> {
        let mut bytes: Vec<u8> = Vec::new();
        write_u64_to_vec(&mut bytes, true as u64);
        write_u128_to_vec(&mut bytes, StableId::from(compressor.session_id).into());
        write_u64_to_vec(&mut bytes, compressor.generated_id_count);
        write_u64_to_vec(&mut bytes, compressor.next_range_base_generation_count);
        serialize_normalizer(&compressor.session_space_normalizer, &mut bytes);
        serialize_finalized(compressor, &mut bytes);
        bytes
    }

    fn serialize_finalized(compressor: &IdCompressor, bytes: &mut Vec<u8>) {
        write_u64_to_vec(bytes, compressor.cluster_capacity);

        compressor
            .sessions
            .get_session_spaces()
            .map(|session_space| StableId::from(session_space.session_id()).into())
            .for_each(|session_u128| write_u128_to_vec(bytes, session_u128));

        compressor
            .final_space
            .get_clusters(&compressor.sessions)
            .for_each(|id_cluster| {
                write_u64_to_vec(bytes, id_cluster.session_creator.get_index() as u64);
                write_u64_to_vec(bytes, id_cluster.capacity);
                write_u64_to_vec(bytes, id_cluster.count);
            });
    }

    pub(super) fn deserialize<FMakeSession>(
        deserializer: Deserializer,
        make_session_id: FMakeSession,
    ) -> Result<IdCompressor, DeserializationError>
    where
        FMakeSession: FnOnce() -> SessionId,
    {
        let with_local_state: bool;
        let mut compressor = match persistent_compressor.local_state {
            None => {
                with_local_state = false;
                IdCompressor::new_with_session_id(make_session_id())
            }
            Some(local_state) => {
                let mut compressor = IdCompressor::new_with_session_id(SessionId::from_uuid_u128(
                    local_state.session_uuid_u128,
                ));
                compressor.generated_id_count = local_state.generated_id_count;
                compressor.next_range_base_generation_count =
                    local_state.next_range_base_generation_count;
                compressor.session_space_normalizer =
                    get_normalizer_from_persistent(local_state.persistent_normalizer);
                with_local_state = true;
                compressor
            }
        };
        compressor.cluster_capacity = persistent_compressor.cluster_capacity;
        for session_uuid_u128 in &persistent_compressor.session_uuid_u128s {
            let session_id = SessionId::from_uuid_u128(*session_uuid_u128);
            if !with_local_state && session_id == compressor.session_id {
                return Err(DeserializationError::InvalidResumedSession);
            }
            compressor.sessions.get_or_create(session_id);
        }

        for cluster_data in persistent_compressor.cluster_data {
            let base_final_id = match compressor
                .final_space
                .get_tail_cluster(&compressor.sessions)
            {
                Some(cluster) => cluster.base_final_id + cluster.capacity,
                None => FinalId::from_id(0),
            };
            let session_space_ref = compressor.sessions.get_or_create(SessionId::from_uuid_u128(
                persistent_compressor.session_uuid_u128s[cluster_data.session_index as usize],
            ));
            let session_space = compressor.sessions.deref_session_space(session_space_ref);
            let base_local_id = match session_space.get_tail_cluster() {
                Some(cluster_ref) => {
                    let cluster = compressor.sessions.deref_cluster(cluster_ref);
                    cluster.base_local_id - cluster.capacity
                }
                None => LocalId::from_id(-1),
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
        }
        compressor.final_id_limit = match compressor
            .final_space
            .get_tail_cluster(&compressor.sessions)
        {
            Some(cluster) => cluster.base_final_id + cluster.count,
            None => FinalId::from_id(0),
        };
        Ok(compressor)
    }

    // TODO perst unit tests

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn assert_local_id_alignment() {
            assert_eq!(LocalId::from_id(-1).to_generation_count(), 1);
        }
    }
}
