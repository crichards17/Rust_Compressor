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
    let version = deserializer.take_u64();
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
    use std::{collections::BTreeMap, mem::size_of};

    use super::DeserializationError;
    use crate::{
        compressor::IdCompressor,
        compressor::{
            persistence_utils::{write_u64_to_vec, Deserializer},
            tables::{
                final_space::FinalSpace,
                session_space::{IdCluster, SessionSpace, SessionSpaceRef, Sessions},
                session_space_normalizer::{
                    persistence::v1::{deserialize_normalizer, serialize_normalizer},
                    SessionSpaceNormalizer,
                },
            },
            TelemetryStats,
        },
    };
    use id_types::{FinalId, LocalId, SessionId};

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
        serialize_header(false, &mut bytes);
        serialize_finalized(compressor, &mut bytes);
        bytes
    }

    pub fn serialize_with_local(compressor: &IdCompressor) -> Vec<u8> {
        let mut bytes: Vec<u8> = Vec::new();
        serialize_header(true, &mut bytes);
        write_u64_to_vec(&mut bytes, compressor.local_session_ref.index as u64);
        write_u64_to_vec(&mut bytes, compressor.generated_id_count);
        write_u64_to_vec(&mut bytes, compressor.next_range_base_generation_count);
        serialize_normalizer(&compressor.session_space_normalizer, &mut bytes);
        serialize_finalized(compressor, &mut bytes);
        bytes
    }

    fn serialize_header(is_local: bool, bytes: &mut Vec<u8>) {
        // Version
        write_u64_to_vec(bytes, 1);
        write_u64_to_vec(bytes, is_local as u64);
    }

    fn serialize_finalized(compressor: &IdCompressor, bytes: &mut Vec<u8>) {
        write_u64_to_vec(bytes, compressor.cluster_capacity);
        write_u64_to_vec(bytes, compressor.sessions.get_session_count() as u64);

        bytes.extend_from_slice(compressor.sessions.get_session_id_slice());

        write_u64_to_vec(bytes, compressor.final_space.get_cluster_count() as u64);
        compressor
            .final_space
            .get_clusters(&compressor.sessions)
            .for_each(|(id_cluster, cluster_ref)| {
                write_u64_to_vec(
                    bytes,
                    cluster_ref.get_session_space_ref().get_index() as u64,
                );
                write_u64_to_vec(bytes, id_cluster.capacity);
                write_u64_to_vec(bytes, id_cluster.count);
            });
    }

    pub(super) fn deserialize<FMakeSession>(
        deserializer: &mut Deserializer,
        make_session_id: FMakeSession,
    ) -> Result<IdCompressor, DeserializationError>
    where
        FMakeSession: FnOnce() -> SessionId,
    {
        let with_local_state = deserializer.take_u64() != 0;
        let local_state = match with_local_state {
            false => None,
            true => Some((
                deserializer.take_u64(),
                deserializer.take_u64(),
                deserializer.take_u64(),
                deserialize_normalizer(deserializer),
            )),
        };

        let cluster_capacity = deserializer.take_u64();
        let mut session_count = deserializer.take_u64() as usize;
        let session_ids = deserializer.take_slice(session_count * size_of::<u128>());
        let mut sessions = Sessions::new();
        sessions.session_ids.extend_from_slice(session_ids);
        let local_index = if with_local_state {
            local_state.as_ref().unwrap().0 as usize
        } else {
            sessions
                .session_ids
                .extend_from_slice(&<[u8; 16]>::from(make_session_id()));
            session_count += 1;
            session_count - 1
        };
        for _ in 0..session_count {
            sessions.session_list.push(SessionSpace::new());
        }
        sessions.session_map = BTreeMap::from_iter((0..session_count).map(|session_space_index| {
            let space_ref = SessionSpaceRef {
                index: session_space_index,
            };
            (sessions.get_session_id(space_ref), space_ref)
        }));

        let local_ref = SessionSpaceRef { index: local_index };
        let session_id = sessions.get_session_id(local_ref);
        let mut compressor = IdCompressor {
            session_id,
            local_session_ref: local_ref,
            generated_id_count: 0,
            next_range_base_generation_count: LocalId::from_id(-1).to_generation_count(),
            sessions,
            final_space: FinalSpace::new(),
            final_id_limit: FinalId::from_id(0),
            session_space_normalizer: SessionSpaceNormalizer::new(),
            cluster_capacity,
            telemetry_stats: TelemetryStats::EMPTY,
        };

        if let Some((_, generated_id_count, next_range_base_generation_count, normalizer)) =
            local_state
        {
            compressor.generated_id_count = generated_id_count;
            compressor.next_range_base_generation_count = next_range_base_generation_count;
            compressor.session_space_normalizer = normalizer;
        }

        let cluster_count = deserializer.take_u64();
        for _ in 0..cluster_count {
            let session_index = deserializer.take_u64();
            let capacity = deserializer.take_u64();
            let count = deserializer.take_u64();

            let base_final_id = match compressor
                .final_space
                .get_tail_cluster(&compressor.sessions)
            {
                Some(cluster) => cluster.base_final_id + cluster.capacity,
                None => FinalId::from_id(0),
            };
            let session_space_ref = SessionSpaceRef {
                index: session_index as usize,
            };
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
            None => FinalId::from_id(0),
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
