/*
on compressor:

+ generate_next_id

+ take_next_block

finalize_block

setClusterSize // must only be called after sequencing :)

serialize

deserialize

----------------------
on id types:

decompress

recompress

normalize_to_op_space

normalize_to_session_space




STRUCTURE
Proposal: Compressor owns the final_space_table, uuid_space_table, and session_table.

*/
use super::id_types::*;
pub(crate) mod tables;
pub(crate) mod utils;
use self::tables::final_space::FinalSpace;
use self::tables::session_space::Sessions;

const DEFAULT_CLUSTER_CAPACITY: u64 = 512;
pub struct IdCompressor {
    session_id: SessionId,
    local_id_count: u64,
    last_taken_local_id_count: u64,
    sessions: Sessions,
    final_space: FinalSpace,
    cluster_capacity: u64,
    cluster_next_base_final_id: FinalId,
}

impl IdCompressor {
    pub fn new() -> Self {
        IdCompressor {
            session_id: SessionId::new(),
            local_id_count: 0,
            last_taken_local_id_count: 0,
            sessions: Sessions::new(),
            final_space: FinalSpace::new(),
            cluster_capacity: DEFAULT_CLUSTER_CAPACITY,
            // TODO: Confirm 0-based FinalID range:
            cluster_next_base_final_id: FinalId { id: (0) },
        }
    }

    // TODO: Eager finals
    pub fn generate_next_id(&mut self) -> SessionSpaceId {
        self.local_id_count += 1;
        SessionSpaceId {
            id: -(self.local_id_count as i64),
        }
    }

    pub fn take_next_range(&self) -> IdRange {
        IdRange {
            id: self.session_id,
            range: if self.local_id_count == self.last_taken_local_id_count {
                None
            } else {
                let count = self.local_id_count - self.last_taken_local_id_count;
                assert!(
                    count > 0,
                    "Must only allocate a positive number of IDs. Count was {}",
                    count
                );
                Some((
                    LocalId::new(-(self.last_taken_local_id_count as i64)),
                    count,
                ))
            },
        }
    }

    pub fn finalize_range(&mut self, id_range: &IdRange) {
        // Check if the block has IDs
        let (range_base_local, range_len) = match &id_range.range {
            None => return,
            Some(range) => {
                if range.1 == 0 {
                    return;
                }
                range
            }
        };

        // Check for space in this Session's current allocated cluster
        // + Get or create SessionSpace for the passed SessionId:
        let session_space_ref = self.sessions.get_or_create(id_range.id);
        let session_space = self.sessions.deref_session_space(session_space_ref);
        // + Get cluster chain's tail cluster:
        let tail_cluster = match session_space.get_tail_cluster() {
            Some(tail_cluster) => tail_cluster,
            None => {
                // This is the first cluster in the session
                debug_assert!(*range_base_local == -1);
                let new_cluster = session_space.add_cluster(
                    session_space_ref,
                    self.cluster_next_base_final_id,
                    *range_base_local,
                    self.cluster_capacity,
                );
                self.cluster_next_base_final_id += self.cluster_capacity;
                self.final_space.add_cluster(&new_cluster);
                // uuid_space.add_cluster
                new_cluster
            }
        };
        if (tail_cluster.capacity - tail_cluster.count) >= *range_len {
            // Add block to current cluster
        } else {
            // Add portion of block to current cluster up to capacity, rest to new block
        }
        // + If space in the current cluster, increment the count of that cluster to account for the new block
        // + If no space in the current cluster, check whether this is the "latest" cluster. If so, expand the cluster as needed.
        // + If no space in the current cluster and this is not the "latest" cluster, add a new cluster:
        // ++ Claim next block of final IDs
        // ++ Create a new cluster in this SessionSpace's cluster_chain (base capacity)
        // ++ Create a new cluster reference in the FinalSpace table
        // ++ Create a new entry in the UuidSpace table
    }
}

pub struct IdRange {
    id: SessionId,
    // (First LocalID in the range, count)
    range: Option<(LocalId, u64)>,
}
