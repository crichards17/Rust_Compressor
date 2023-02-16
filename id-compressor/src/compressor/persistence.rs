pub(super) const DEFAULT_CLUSTER_CAPACITY: u64 = 512;

enum Version {
    V1 = 1,
}

pub(crate) mod v1 {

    use crate::{
        compressor::tables::session_space_normalizer::SessionSpaceNormalizer,
        compressor::tables::{session_space, session_space_normalizer},
        id_types::{SessionId, StableId},
    };

    enum Variant {
        Finalized = 1,
        WithLocal = 2,
    }

    pub(crate) struct ClusterData {
        pub session_index: u64,
        pub capacity: u64,
        pub count: u64,
    }

    fn write_u32_to_vec(buffer: &mut Vec<u8>, num: u32) {
        let bytes = num.to_be_bytes();
        for byte in bytes {
            buffer.push(byte);
        }
    }

    fn write_u64_to_vec(buffer: &mut Vec<u8>, num: u64) {
        let bytes = num.to_be_bytes();
        for byte in bytes {
            buffer.push(byte);
        }
    }

    fn write_u128_to_vec(buffer: &mut Vec<u8>, num: u128) {
        let bytes = num.to_be_bytes();
        for byte in bytes {
            buffer.push(byte);
        }
    }

    fn serialize(
        sessions: impl Iterator<Item = SessionId>,
        sessions_count: usize,
        clusters: impl Iterator<Item = ClusterData>,
        clusters_count: usize,
        variant: Variant,
    ) -> Vec<u8> {
        let mut buffer: Vec<u8> = Vec::new();
        // 4 bytes: version number
        write_u32_to_vec(&mut buffer, super::Version::V1 as u32);
        // 1 byte: Variant (finalized, with_local)
        buffer.push(variant as u8);
        // 8 bytes: Sessions table count (usize)
        write_u64_to_vec(&mut buffer, sessions_count as u64);
        // n x 16 bytes: Sessions table
        let mut session_count_actual = 0;
        for session_id in sessions {
            write_u128_to_vec(&mut buffer, StableId::from(session_id).to_uuid_u128());
            session_count_actual += 1;
        }
        debug_assert!(session_count_actual == sessions_count, "Bad sessions count");
        // 8 bytes: Final space table count (usize)
        write_u64_to_vec(&mut buffer, clusters_count as u64);
        // n x 32 bytes: Final Space table as ID Clusters:
        /* {
           8 bytes: session_creator,
           8 bytes: capacity,
           8 bytes: count
        } */
        let mut clusters_count_actual = 0;
        for cluster_data in clusters {
            write_u64_to_vec(&mut buffer, cluster_data.session_index);
            write_u64_to_vec(&mut buffer, cluster_data.capacity);
            write_u64_to_vec(&mut buffer, cluster_data.count);

            clusters_count_actual += 1;
        }
        debug_assert!(
            clusters_count_actual == clusters_count,
            "Bad clusters count"
        );
        buffer
    }

    pub(crate) fn serialize_finalized(
        sessions: impl Iterator<Item = SessionId>,
        sessions_count: usize,
        clusters: impl Iterator<Item = ClusterData>,
        clusters_count: usize,
    ) -> Vec<u8> {
        serialize(
            sessions,
            sessions_count,
            clusters,
            clusters_count,
            Variant::Finalized,
        )
    }

    pub(crate) fn serialize_with_local(
        sessions: impl Iterator<Item = SessionId>,
        sessions_count: usize,
        clusters: impl Iterator<Item = ClusterData>,
        clusters_count: usize,
        session_space_normalizer: &SessionSpaceNormalizer,
    ) -> Vec<u8> {
        let mut out_bytes = serialize(
            sessions,
            sessions_count,
            clusters,
            clusters_count,
            Variant::WithLocal,
        );
        let mut normalizer_serialized =
            session_space_normalizer::persistence::v1::serialize(session_space_normalizer);
        // Prepends session_space_normalizer serialized length in byte count
        write_u64_to_vec(&mut out_bytes, normalizer_serialized.len() as u64);
        out_bytes.append(&mut normalizer_serialized);
        out_bytes
    }
}
