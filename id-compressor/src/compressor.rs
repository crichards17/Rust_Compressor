/*
on compressor:

+ generate_next_id

+ take_next_block

+ finalize_range

+ setClusterSize

serialize

deserialize

----------------------
on id types:

+ decompress

recompress

normalize_to_op_space

normalize_to_session_space

// TODO:
1. Review eager finals
2. Write some decompression tests :)

*/
use super::id_types::*;
pub(crate) mod tables;
pub(crate) mod utils;
use self::tables::final_space::FinalSpace;
use self::tables::session_space::{ClusterRef, SessionSpace, SessionSpaceRef, Sessions};
use self::tables::uuid_space::UuidSpace;

const DEFAULT_CLUSTER_CAPACITY: u64 = 512;
pub struct IdCompressor {
    session_id: SessionId,
    local_id_count: u64,
    last_taken_local_id_count: u64,
    sessions: Sessions,
    final_space: FinalSpace,
    uuid_space: UuidSpace,
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
            uuid_space: UuidSpace::new(),
            cluster_capacity: DEFAULT_CLUSTER_CAPACITY,
            cluster_next_base_final_id: FinalId { id: (0) },
        }
    }

    pub fn set_cluster_capacity(&mut self, new_cluster_capacity: u64) {
        self.cluster_capacity = new_cluster_capacity;
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

    pub fn finalize_range(
        &mut self,
        &IdRange {
            id: session_id,
            range,
        }: &IdRange,
    ) {
        // Check if the block has IDs
        let (range_base_local, range_len) = match range {
            None => return,
            Some((_, 0)) => {
                return;
            }
            Some(range) => range,
        };

        let session_space_ref = self.sessions.get_or_create(session_id);
        let tail_cluster_ref = match self
            .sessions
            .deref_session_space_mut(session_space_ref)
            .get_tail_cluster()
        {
            Some(tail_cluster) => tail_cluster,
            None => {
                // This is the first cluster in the session
                debug_assert!(range_base_local == -1);
                self.add_empty_cluster(
                    session_space_ref,
                    range_base_local,
                    session_id,
                    self.cluster_capacity,
                )
            }
        };
        let tail_cluster = self.sessions.deref_cluster_mut(tail_cluster_ref);
        let remaining_capacity = tail_cluster.capacity - tail_cluster.count;
        if remaining_capacity >= range_len {
            // The current IdBlock range fits in the existing cluster
            tail_cluster.count += range_len;
        } else {
            let overflow = range_len - remaining_capacity;
            let new_claimed_final_count = overflow + self.cluster_capacity;
            if self.final_space.is_last(tail_cluster_ref) {
                // Tail_cluster is the last cluster, and so can be expanded.
                self.cluster_next_base_final_id += new_claimed_final_count;
                tail_cluster.capacity += new_claimed_final_count;
                tail_cluster.count += range_len;
            } else {
                // Tail_cluster is not the last cluster. Fill and overflow to new.
                tail_cluster.count = tail_cluster.capacity;
                let new_cluster_ref = self.add_empty_cluster(
                    session_space_ref,
                    range_base_local - remaining_capacity,
                    session_id,
                    new_claimed_final_count,
                );
                self.sessions.deref_cluster_mut(new_cluster_ref).count += overflow;
            }
        }
    }

    fn add_empty_cluster(
        &mut self,
        session_space_ref: SessionSpaceRef,
        base_local: LocalId,
        session_id: SessionId,
        capacity: u64,
    ) -> ClusterRef {
        let session_space = self.sessions.deref_session_space_mut(session_space_ref);
        let new_cluster_ref =
            session_space.add_cluster(self.cluster_next_base_final_id, base_local, capacity);
        self.cluster_next_base_final_id += capacity;
        self.final_space
            .add_cluster(new_cluster_ref, &self.sessions);
        self.uuid_space
            .add_cluster(session_id, new_cluster_ref, &self.sessions);
        new_cluster_ref
    }
}

impl SessionSpaceId {
    pub fn decompress(&self, compressor: &IdCompressor) -> Result<StableId, DecompressionError> {
        match self.to_space() {
            CompressedId::Final(final_id) => {
                match compressor
                    .final_space
                    .search(final_id, &compressor.sessions)
                {
                    Some(containing_cluster) => {
                        let final_delta = final_id.id - containing_cluster.base_final_id.id;
                        let aligned_local = containing_cluster.base_local_id - final_delta;
                        Ok(compressor
                            .sessions
                            .deref_session_space(containing_cluster.session_creator)
                            .session_id()
                            + aligned_local)
                    }
                    None => Err(DecompressionError::UnknownFinalId),
                }
            }
            CompressedId::Local(local_id) => Ok(compressor.session_id + local_id),
        }
    }
}

pub enum DecompressionError {
    UnknownFinalId,
}

pub struct IdRange {
    id: SessionId,
    // (First LocalID in the range, count)
    range: Option<(LocalId, u64)>,
}
