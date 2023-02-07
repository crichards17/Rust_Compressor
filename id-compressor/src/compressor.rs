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

+ recompress

+ normalize_to_op_space

+ normalize_to_session_space

// TODO:
- Bit twiddling UUID math
- Revise id_types.rs

*/
pub(crate) mod tables;
use self::tables::final_space::FinalSpace;
use self::tables::session_space::{ClusterRef, SessionSpace, SessionSpaceRef, Sessions};
use self::tables::session_space_normalizer::SessionSpaceNormalizer;
use self::tables::uuid_space::UuidSpace;
use super::id_types::*;

const DEFAULT_CLUSTER_CAPACITY: u64 = 512;
pub struct IdCompressor {
    session_id: SessionId,
    local_session: SessionSpaceRef,
    generated_id_count: u64,
    next_range_base: LocalId,
    sessions: Sessions,
    final_space: FinalSpace,
    uuid_space: UuidSpace,
    session_space_normalizer: SessionSpaceNormalizer,
    cluster_capacity: u64,
    cluster_next_base_final_id: FinalId,
}

impl IdCompressor {
    pub fn new() -> Self {
        let mut sessions = Sessions::new();
        let session_id = SessionId::new();
        IdCompressor {
            session_id,
            local_session: sessions.get_or_create(session_id),
            generated_id_count: 0,
            next_range_base: LocalId::new(-1),
            sessions,
            final_space: FinalSpace::new(),
            uuid_space: UuidSpace::new(),
            session_space_normalizer: SessionSpaceNormalizer::new(),
            cluster_capacity: DEFAULT_CLUSTER_CAPACITY,
            cluster_next_base_final_id: FinalId::new(0),
        }
    }

    fn get_local_session_space(&self) -> &SessionSpace {
        self.sessions.deref_session_space(self.local_session)
    }

    pub fn set_cluster_capacity(&mut self, new_cluster_capacity: u64) {
        self.cluster_capacity = new_cluster_capacity;
    }

    pub fn generate_next_id(&mut self) -> SessionSpaceId {
        self.generated_id_count += 1;
        let tail_cluster = match self.get_local_session_space().get_tail_cluster() {
            Some(tail_cluster_ref) => self.sessions.deref_cluster(tail_cluster_ref),
            None => {
                // No cluster, return next local
                return self.generate_next_local_id().into();
            }
        };
        let cluster_offset =
            self.generated_id_count - tail_cluster.base_local_id.to_generation_count();
        if tail_cluster.capacity > cluster_offset {
            // Space in the cluster: eager final
            return (tail_cluster.base_final_id + cluster_offset).into();
        } else {
            // Not space, return next local
            return self.generate_next_local_id().into();
        }
    }

    fn generate_next_local_id(&mut self) -> LocalId {
        let new_local = LocalId::new(-(self.generated_id_count as i64));
        self.session_space_normalizer.add_local_range(new_local, 1);
        return new_local;
    }

    pub fn take_next_range(&mut self) -> IdRange {
        let count = self.generated_id_count - (self.next_range_base.to_generation_count() - 1);
        IdRange {
            id: self.session_id,
            range: if count == 0 {
                None
            } else {
                assert!(
                    count > 0,
                    "Must only allocate a positive number of IDs. Count was {}",
                    count
                );
                let next_range = Some((self.next_range_base, count));
                self.next_range_base = LocalId::from_generation_count(self.generated_id_count + 1);
                next_range
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
                    self.cluster_capacity + range_len,
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
                        let aligned_local = containing_cluster.get_aligned_local(final_id).unwrap();
                        if aligned_local < containing_cluster.max_local() {
                            // must be an id generated (allocated or finalized) by the local session, or a finalized id from a remote session
                            if containing_cluster.session_creator == compressor.local_session {
                                if compressor.session_space_normalizer.contains(aligned_local) {
                                    return Err(DecompressionError::UnobtainableId);
                                }
                                if aligned_local.to_generation_count()
                                    > compressor.generated_id_count
                                {
                                    return Err(DecompressionError::UngeneratedFinalId);
                                }
                            } else {
                                return Err(DecompressionError::UnfinalizedId);
                            }
                        }

                        Ok(compressor
                            .sessions
                            .deref_session_space(containing_cluster.session_creator)
                            .session_id()
                            .stable_from_local_offset(aligned_local))
                    }
                    None => Err(DecompressionError::UnallocatedFinalId),
                }
            }
            CompressedId::Local(local_id) => {
                if !compressor.session_space_normalizer.contains(local_id) {
                    return Err(DecompressionError::UnobtainableId);
                }
                Ok(compressor.session_id.stable_from_local_offset(local_id))
            }
        }
    }

    pub fn normalize_to_op_space(
        &self,
        compressor: &IdCompressor,
    ) -> Result<OpSpaceId, NormalizationError> {
        // Return the most final version of the given StableId
        match self.to_space() {
            CompressedId::Final(final_id) => Ok(OpSpaceId::from(final_id)),
            CompressedId::Local(local_id) => {
                if !compressor.session_space_normalizer.contains(local_id) {
                    return Err(NormalizationError::UnknownSessionSpaceId);
                } else {
                    let local_session_space = compressor
                        .sessions
                        .deref_session_space(compressor.local_session);
                    match local_session_space.try_convert_to_final(local_id) {
                        Some(converted_final) => Ok(OpSpaceId::from(converted_final)),
                        None => Ok(OpSpaceId::from(local_id)),
                    }
                }
            }
        }
    }
}

impl OpSpaceId {
    pub fn normalize_to_session_space(
        &self,
        originator: SessionId,
        compressor: &IdCompressor,
    ) -> Result<SessionSpaceId, NormalizationError> {
        match self.to_space() {
            CompressedId::Local(local_to_normalize) => {
                if originator == compressor.session_id {
                    Ok(SessionSpaceId::from(local_to_normalize))
                } else {
                    // LocalId from a foreign session
                    let foreign_session_space = match compressor.sessions.get(originator) {
                        Some(session_space) => session_space,
                        None => {
                            return Err(NormalizationError::UnknownSessionId);
                        }
                    };
                    match foreign_session_space.try_convert_to_final(local_to_normalize) {
                        Some(final_id) => Ok(SessionSpaceId::from(final_id)),
                        None => Err(NormalizationError::UnfinalizedForeignLocal),
                    }
                }
            }
            CompressedId::Final(final_to_normalize) => {
                match compressor
                    .get_local_session_space()
                    .get_cluster_by_allocated_final(final_to_normalize)
                {
                    // Exists in local cluster chain
                    Some(containing_cluster) => {
                        let aligned_local = containing_cluster
                            .get_aligned_local(final_to_normalize)
                            .unwrap();
                        if compressor.session_space_normalizer.contains(aligned_local) {
                            Ok(SessionSpaceId::from(aligned_local))
                        } else {
                            if aligned_local.to_generation_count() <= compressor.generated_id_count
                            {
                                Ok(SessionSpaceId::from(final_to_normalize))
                            } else {
                                Err(NormalizationError::UngeneratedId)
                            }
                        }
                    }
                    None => {
                        // Does not exist in local cluster chain
                        match compressor
                            .final_space
                            .get_tail_cluster(&compressor.sessions)
                        {
                            None => Err(NormalizationError::NoFinalizedRanges),
                            Some(final_space_tail_cluster) => {
                                if final_to_normalize <= final_space_tail_cluster.max_final() {
                                    Ok(SessionSpaceId::from(final_to_normalize))
                                } else {
                                    Err(NormalizationError::UnFinalizedForeignFinal)
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

impl StableId {
    pub fn recompress(
        &self,
        compressor: &IdCompressor,
    ) -> Result<SessionSpaceId, RecompressionError> {
        match compressor.uuid_space.search(*self, &compressor.sessions) {
            None => {
                let session_as_stable = StableId::from(compressor.session_id);
                if self >= &session_as_stable {
                    // TODO: WARN: UUID math
                    let gen_count_equivalent = self.sub_unsafe(session_as_stable) + 1;
                    if gen_count_equivalent <= compressor.generated_id_count as u128 {
                        // Is a locally generated ID, with or without a finalized cluster
                        let local_equivalent =
                            LocalId::from_generation_count(gen_count_equivalent as u64);
                        if compressor
                            .session_space_normalizer
                            .contains(local_equivalent)
                        {
                            return Ok(SessionSpaceId::from(local_equivalent));
                        }
                    }
                }
                Err(RecompressionError::UnallocatedStableId)
            }
            Some((cluster, originator_local)) => {
                if cluster.session_creator == compressor.local_session {
                    // Local session
                    if compressor
                        .session_space_normalizer
                        .contains(originator_local)
                    {
                        return Ok(SessionSpaceId::from(originator_local));
                    } else if originator_local.to_generation_count()
                        <= compressor.generated_id_count
                    {
                        // Id is an eager final
                        Ok(cluster
                            .get_allocated_final(originator_local)
                            .unwrap()
                            .into())
                    } else {
                        return Err(RecompressionError::UngeneratedStableId);
                    }
                } else {
                    //Not the local session
                    if originator_local.to_generation_count()
                        < cluster.base_local_id.to_generation_count() + cluster.count
                    {
                        Ok(cluster
                            .get_allocated_final(originator_local)
                            .unwrap()
                            .into())
                    } else {
                        Err(RecompressionError::UnfinalizedForeignId)
                    }
                }
            }
        }
    }
}

// TODO: comment each one about how it can happen
#[derive(Debug)]
pub enum DecompressionError {
    UnfinalizedId,
    UnallocatedFinalId,
    UnobtainableId,
    UngeneratedFinalId,
}

#[derive(Debug)]
pub enum RecompressionError {
    UnallocatedStableId,
    UngeneratedStableId,
    UnfinalizedForeignId,
}

#[derive(Debug)]
pub enum NormalizationError {
    UnknownSessionSpaceId,
    UnknownSessionId,
    UngeneratedId,
    UnfinalizedForeignLocal,
    UnFinalizedForeignFinal,
    NoFinalizedRanges,
}

pub struct IdRange {
    id: SessionId,
    // (First LocalID in the range, count)
    range: Option<(LocalId, u64)>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_complex() {
        let mut compressor = IdCompressor::new();

        compressor.set_cluster_capacity(3);

        // Before first cluster creation
        let session_space_id_1 = compressor.generate_next_id();
        let session_space_id_2 = compressor.generate_next_id();
        assert!(session_space_id_1.is_local());
        assert!(session_space_id_2.is_local());

        // Take initial range
        let out_range = compressor.take_next_range();

        // Finalize initial range
        compressor.finalize_range(&out_range);

        let session_space_id_3 = compressor.generate_next_id();
        let session_space_id_4 = compressor.generate_next_id();
        let session_space_id_5 = compressor.generate_next_id();
        let session_space_id_6 = compressor.generate_next_id();
        let session_space_id_7 = compressor.generate_next_id();

        // 3-5 are within initial cluster capacity (intialized to 3 + 2 capacity)
        assert!(session_space_id_3.is_final());
        assert!(session_space_id_4.is_final());
        assert!(session_space_id_5.is_final());

        // 6 and 7 are outside of initial cluster capacity
        assert!(session_space_id_6.is_local());
        assert!(session_space_id_7.is_local());

        let mut offset: usize = 0;
        let op_space_ids = [0, 1, 2, 3, 4, -6, -7];
        for id in [
            session_space_id_1,
            session_space_id_2,
            session_space_id_3,
            session_space_id_4,
            session_space_id_5,
            session_space_id_6,
            session_space_id_7,
        ] {
            let stable_id = StableId::from(compressor.session_id).offset_by(offset as u64);
            assert_eq!(id.decompress(&compressor).unwrap(), stable_id,);
            assert_eq!(stable_id.recompress(&compressor).unwrap(), id);

            let op_space_id = id.normalize_to_op_space(&compressor).unwrap();
            assert_eq!(
                id,
                op_space_id
                    .normalize_to_session_space(compressor.session_id, &compressor)
                    .unwrap()
            );
            if op_space_ids[offset] < 0 {
                assert_eq!(
                    op_space_id,
                    OpSpaceId::from(LocalId::new(op_space_ids[offset]))
                );
            } else {
                assert_eq!(
                    op_space_id,
                    OpSpaceId::from(FinalId::new(op_space_ids[offset] as u64))
                );
            }
            offset += 1;
        }
    }

    #[test]
    fn test_decompress_recompress() {
        let mut compressor = IdCompressor::new();

        let session_space_id = compressor.generate_next_id();

        let stable_id = StableId::from(compressor.session_id);
        assert_eq!(session_space_id.decompress(&compressor).unwrap(), stable_id,);
        assert_eq!(stable_id.recompress(&compressor).unwrap(), session_space_id);
    }
}
