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
use self::tables::sessions::Sessions;

pub struct IdCompressor<'a> {
    // state
    session_id: SessionId,
    local_id_count: i64,
    last_taken_local_id_count: i64,
    sessions: Sessions<'a>,
    // final_space: tables::final_space::FinalSpace,
}

impl<'a> IdCompressor<'a> {
    // TODO: Update to match final state
    pub fn new() -> Self {
        IdCompressor {
            session_id: SessionId::new(),
            local_id_count: 0,
            last_taken_local_id_count: 0,
            sessions: Sessions::new(),
        }
    }

    // TODO: Eager finals
    pub fn generate_next_id(&mut self) -> SessionSpaceId {
        self.local_id_count += 1;
        SessionSpaceId {
            id: -self.local_id_count,
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
                Some((LocalId::new(-self.last_taken_local_id_count), count))
            },
        }
    }

    pub fn finalize_range(&mut self, id_range: &IdRange) {
        // Check if the block has IDs
        let range = match &id_range.range {
            None => return,
            Some(range) => range,
        };

        // Check for space in this Session's current allocated cluster
        let session_space = self.sessions.get_or_create(id_range.id);
        // Get cluster chain's tail cluster
        let tail_cluster = match session_space.get_tail_cluster() {
            Some(tail_cluster) => tail_cluster,
            None => {
                // Create new cluster in session_space
            }
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
    range: Option<(LocalId, i64)>,
}
