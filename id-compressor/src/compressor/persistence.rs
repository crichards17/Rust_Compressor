pub(super) const DEFAULT_CLUSTER_CAPACITY: u64 = 512;

enum Version {
    V1 = 1,
}

pub(crate) mod v1 {

    use crate::id_types::{SessionId, StableId};

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

    pub(crate) fn serialize(
        sessions: impl Iterator<Item = SessionId>,
        sessions_count: usize,
        clusters: impl Iterator<Item = ClusterData>,
        clusters_count: usize,
    ) -> Vec<u8> {
        let mut buffer: Vec<u8> = Vec::new();
        // 4 bytes: version number
        write_u32_to_vec(&mut buffer, super::Version::V1 as u32);
        // 1 byte: Variant (finalized, with_local)
        buffer.push(Variant::Finalized as u8);
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
}
