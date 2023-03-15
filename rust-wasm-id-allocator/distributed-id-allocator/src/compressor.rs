pub(crate) mod persistence;
pub(crate) mod tables;
use self::persistence::DeserializationError;
use self::tables::final_space::FinalSpace;
use self::tables::session_space::{ClusterRef, SessionSpace, SessionSpaceRef, Sessions};
use self::tables::session_space_normalizer::SessionSpaceNormalizer;
use self::tables::uuid_space::UuidSpace;
use id_types::session_id::UuidGenerationError;
use id_types::*;

#[derive(PartialEq, Eq, Debug)]
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
}

impl IdCompressor {
    #[cfg(feature = "uuid-generation")]
    pub fn new() -> Self {
        let session_id = SessionId::new();
        IdCompressor::new_with_session_id(session_id)
    }

    pub fn new_with_session_id(session_id: SessionId) -> Self {
        let mut sessions = Sessions::new();
        IdCompressor {
            session_id,
            local_session: sessions.get_or_create(session_id),
            generated_id_count: 0,
            next_range_base: LocalId::from_id(-1),
            sessions,
            final_space: FinalSpace::new(),
            uuid_space: UuidSpace::new(),
            session_space_normalizer: SessionSpaceNormalizer::new(),
            // TODO: Refactor to consumer-passed cluster_capacity value
            cluster_capacity: persistence::DEFAULT_CLUSTER_CAPACITY,
        }
    }

    pub fn get_local_session_id(&self) -> SessionId {
        self.session_id
    }

    pub fn get_local_session_token(&self) -> usize {
        self.local_session.get_index()
    }

    fn get_local_session_space(&self) -> &SessionSpace {
        self.sessions.deref_session_space(self.local_session)
    }

    pub fn get_session_id_from_session_token(
        &self,
        index: usize,
    ) -> Result<SessionId, impl ErrorEnum> {
        if index >= self.sessions.get_sessions_count() {
            return Err(SessionTokenError::UnknownSessionToken);
        }
        let session_space_ref = SessionSpaceRef::create_from_index(index);
        Ok(self
            .sessions
            .deref_session_space(session_space_ref)
            .session_id())
    }

    pub fn get_session_token_from_session_id(
        &self,
        session_id: SessionId,
    ) -> Result<usize, impl ErrorEnum> {
        match self.sessions.get(session_id) {
            None => Err(SessionTokenError::UnknownSessionId),
            Some(session_space) => Ok(session_space.self_ref().get_index()),
        }
    }

    pub fn set_cluster_capacity(
        &mut self,
        new_cluster_capacity: u64,
    ) -> Result<(), ClusterCapacityError> {
        if new_cluster_capacity < 1 {
            Err(ClusterCapacityError::InvalidClusterCapacity)
        } else {
            self.cluster_capacity = new_cluster_capacity;
            Ok(())
        }
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
        let new_local = LocalId::from_id(-(self.generated_id_count as i64));
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
    ) -> Result<(), FinalizationError> {
        // Check if the block has IDs
        let (range_base_local, range_len) = match range {
            None => {
                return Ok(());
            }
            Some((_, 0)) => {
                return Err(FinalizationError::InvalidRange);
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
                if range_base_local != -1 {
                    return Err(FinalizationError::RangeFinalizedOutOfOrder);
                }
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
        if tail_cluster.base_local_id - tail_cluster.count != range_base_local {
            return Err(FinalizationError::RangeFinalizedOutOfOrder);
        }
        if remaining_capacity >= range_len {
            // The current IdBlock range fits in the existing cluster
            tail_cluster.count += range_len;
        } else {
            let overflow = range_len - remaining_capacity;
            let new_claimed_final_count = overflow + self.cluster_capacity;
            if self.final_space.is_last(tail_cluster_ref) {
                // Tail_cluster is the last cluster, and so can be expanded.
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
        Ok(())
    }

    fn add_empty_cluster(
        &mut self,
        session_space_ref: SessionSpaceRef,
        base_local: LocalId,
        session_id: SessionId,
        capacity: u64,
    ) -> ClusterRef {
        let next_base_final = match self.final_space.get_tail_cluster(&self.sessions) {
            Some(cluster) => cluster.base_final_id + cluster.capacity,
            None => FinalId::from_id(0),
        };
        let session_space = self.sessions.deref_session_space_mut(session_space_ref);
        let new_cluster_ref =
            session_space.add_empty_cluster(next_base_final, base_local, capacity);
        self.final_space
            .add_cluster(new_cluster_ref, &self.sessions);
        self.uuid_space
            .add_cluster(session_id, new_cluster_ref, &self.sessions);
        new_cluster_ref
    }

    pub fn serialize(&self, include_local_state: bool) -> Vec<u8> {
        if !include_local_state {
            persistence::v1::serialize(&self)
        } else {
            persistence::v1::serialize_with_local(&self)
        }
    }

    #[cfg(feature = "uuid-generation")]
    pub fn deserialize(bytes: &[u8]) -> Result<IdCompressor, DeserializationError> {
        persistence::deserialize(bytes, || SessionId::new())
    }

    pub fn deserialize_with_session_id<FMakeSession>(
        bytes: &[u8],
        make_session_id: FMakeSession,
    ) -> Result<IdCompressor, DeserializationError>
    where
        FMakeSession: FnOnce() -> SessionId,
    {
        persistence::deserialize(bytes, make_session_id)
    }

    pub fn normalize_to_op_space(
        &self,
        id: SessionSpaceId,
    ) -> Result<OpSpaceId, NormalizationError> {
        match id.to_space() {
            CompressedId::Final(final_id) => Ok(OpSpaceId::from(final_id)),
            CompressedId::Local(local_id) => {
                if !self.session_space_normalizer.contains(local_id) {
                    return Err(NormalizationError::UnknownSessionSpaceId);
                } else {
                    let local_session_space = self.sessions.deref_session_space(self.local_session);
                    match local_session_space.try_convert_to_final(local_id) {
                        Some(converted_final) => Ok(OpSpaceId::from(converted_final)),
                        None => Ok(OpSpaceId::from(local_id)),
                    }
                }
            }
        }
    }

    pub fn normalize_to_session_space(
        &self,
        id: OpSpaceId,
        originator: SessionId,
    ) -> Result<SessionSpaceId, NormalizationError> {
        match id.to_space() {
            CompressedId::Local(local_to_normalize) => {
                if originator == self.session_id {
                    Ok(SessionSpaceId::from(local_to_normalize))
                } else {
                    // LocalId from a foreign session
                    let foreign_session_space = match self.sessions.get(originator) {
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
                match self
                    .get_local_session_space()
                    .get_cluster_by_allocated_final(final_to_normalize)
                {
                    // Exists in local cluster chain
                    Some(containing_cluster) => {
                        let aligned_local =
                            match containing_cluster.get_aligned_local(final_to_normalize) {
                                None => return Err(NormalizationError::NoAlignedLocal),
                                Some(aligned_local) => aligned_local,
                            };
                        if self.session_space_normalizer.contains(aligned_local) {
                            Ok(SessionSpaceId::from(aligned_local))
                        } else {
                            if aligned_local.to_generation_count() <= self.generated_id_count {
                                Ok(SessionSpaceId::from(final_to_normalize))
                            } else {
                                Err(NormalizationError::UngeneratedId)
                            }
                        }
                    }
                    None => {
                        // Does not exist in local cluster chain
                        match self.final_space.get_tail_cluster(&self.sessions) {
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

    pub fn decompress(&self, id: SessionSpaceId) -> Result<StableId, DecompressionError> {
        match id.to_space() {
            CompressedId::Final(final_id) => {
                match self.final_space.search(final_id, &self.sessions) {
                    Some(containing_cluster) => {
                        let aligned_local = match containing_cluster.get_aligned_local(final_id) {
                            None => return Err(DecompressionError::NoAlignedLocal),
                            Some(aligned_local) => aligned_local,
                        };
                        if aligned_local < containing_cluster.max_local() {
                            // must be an id generated (allocated or finalized) by the local session, or a finalized id from a remote session
                            if containing_cluster.session_creator == self.local_session {
                                if self.session_space_normalizer.contains(aligned_local) {
                                    return Err(DecompressionError::UnobtainableId);
                                }
                                if aligned_local.to_generation_count() > self.generated_id_count {
                                    return Err(DecompressionError::UngeneratedFinalId);
                                }
                            } else {
                                return Err(DecompressionError::UnfinalizedId);
                            }
                        }

                        Ok(self
                            .sessions
                            .deref_session_space(containing_cluster.session_creator)
                            .session_id()
                            .stable_from_local_offset(aligned_local))
                    }
                    None => Err(DecompressionError::UnallocatedFinalId),
                }
            }
            CompressedId::Local(local_id) => {
                if !self.session_space_normalizer.contains(local_id) {
                    return Err(DecompressionError::UnobtainableId);
                }
                Ok(self.session_id.stable_from_local_offset(local_id))
            }
        }
    }

    pub fn recompress(&self, id: StableId) -> Result<SessionSpaceId, RecompressionError> {
        match self.uuid_space.search(id, &self.sessions) {
            None => {
                let session_as_stable = StableId::from(self.session_id);
                if id >= session_as_stable {
                    // TODO: WARN: UUID math
                    let gen_count_equivalent = id - session_as_stable + 1;
                    if gen_count_equivalent <= self.generated_id_count as u128 {
                        // Is a locally generated ID, with or without a finalized cluster
                        let local_equivalent =
                            LocalId::from_generation_count(gen_count_equivalent as u64);
                        if self.session_space_normalizer.contains(local_equivalent) {
                            return Ok(SessionSpaceId::from(local_equivalent));
                        }
                    }
                }
                Err(RecompressionError::UnallocatedStableId)
            }
            Some((cluster, originator_local)) => {
                if cluster.session_creator == self.local_session {
                    // Local session
                    if self.session_space_normalizer.contains(originator_local) {
                        return Ok(SessionSpaceId::from(originator_local));
                    } else if originator_local.to_generation_count() <= self.generated_id_count {
                        // Id is an eager final
                        match cluster.get_allocated_final(originator_local) {
                            None => return Err(RecompressionError::NoAllocatedFinal),
                            Some(allocated_final) => Ok(allocated_final.into()),
                        }
                    } else {
                        return Err(RecompressionError::UngeneratedStableId);
                    }
                } else {
                    //Not the local session
                    if originator_local.to_generation_count()
                        < cluster.base_local_id.to_generation_count() + cluster.count
                    {
                        match cluster.get_allocated_final(originator_local) {
                            None => return Err(RecompressionError::NoAllocatedFinal),
                            Some(allocated_final) => Ok(allocated_final.into()),
                        }
                    } else {
                        Err(RecompressionError::UnfinalizedForeignId)
                    }
                }
            }
        }
    }
}

pub trait ErrorEnum {
    fn get_error_string(&self) -> &'static str;
}

impl ErrorEnum for UuidGenerationError {
    fn get_error_string(&self) -> &'static str {
        match self {
            UuidGenerationError::InvalidUuidString => "Invalid Uuid String",
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
    NoAlignedLocal,
}

impl ErrorEnum for DecompressionError {
    fn get_error_string(&self) -> &'static str {
        match self {
            DecompressionError::UnfinalizedId => "UnfinalizedId",
            DecompressionError::UnallocatedFinalId => "UnallocatedFinalId",
            DecompressionError::UnobtainableId => "UnobtainableId",
            DecompressionError::UngeneratedFinalId => "UngeneratedFinalId",
            DecompressionError::NoAlignedLocal => "NoAlignedLocal",
        }
    }
}

#[derive(Debug)]
pub enum RecompressionError {
    UnallocatedStableId,
    UngeneratedStableId,
    UnfinalizedForeignId,
    NoAllocatedFinal,
}

impl ErrorEnum for RecompressionError {
    fn get_error_string(&self) -> &'static str {
        match self {
            RecompressionError::UnallocatedStableId => "UnallocatedStableId",
            RecompressionError::UngeneratedStableId => "UngeneratedStableId",
            RecompressionError::UnfinalizedForeignId => "UnfinalizedForeignId",
            RecompressionError::NoAllocatedFinal => "NoAllocatedFinal",
        }
    }
}

#[derive(Debug)]
pub enum FinalizationError {
    RangeFinalizedOutOfOrder,
    InvalidRange,
}

impl ErrorEnum for FinalizationError {
    fn get_error_string(&self) -> &'static str {
        match self {
            FinalizationError::RangeFinalizedOutOfOrder => "RangeFinalizedOutOfOrder.",
            FinalizationError::InvalidRange => "Invalid Range.",
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum ClusterCapacityError {
    InvalidClusterCapacity,
}

impl ErrorEnum for ClusterCapacityError {
    fn get_error_string(&self) -> &'static str {
        match self {
            ClusterCapacityError::InvalidClusterCapacity => "Invalid cluster capacity.",
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum SessionTokenError {
    UnknownSessionToken,
    UnknownSessionId,
}

impl ErrorEnum for SessionTokenError {
    fn get_error_string(&self) -> &'static str {
        match self {
            SessionTokenError::UnknownSessionToken => "Unknown Session Token.",
            SessionTokenError::UnknownSessionId => "UnknownSessionId.",
        }
    }
}

#[derive(Debug)]
pub enum NormalizationError {
    UnknownSessionSpaceId,
    UnknownSessionId,
    UngeneratedId,
    UnfinalizedForeignLocal,
    UnFinalizedForeignFinal,
    NoFinalizedRanges,
    NoAlignedLocal,
    NoSessionIdProvided,
}

impl NormalizationError {
    pub fn get_error_string(&self) -> &'static str {
        match self {
            NormalizationError::UnknownSessionSpaceId => "UnknownSessionSpaceId",
            NormalizationError::UnknownSessionId => "UnknownSessionId",
            NormalizationError::UngeneratedId => "UngeneratedId",
            NormalizationError::UnfinalizedForeignLocal => "UnfinalizedForeignLocal",
            NormalizationError::UnFinalizedForeignFinal => "UnFinalizedForeignFinal",
            NormalizationError::NoFinalizedRanges => "NoFinalizedRanges",
            NormalizationError::NoAlignedLocal => "NoAlignedLocal",
            NormalizationError::NoSessionIdProvided => "NoSessionIdProvided",
        }
    }
}

pub struct IdRange {
    pub id: SessionId,
    // (First LocalID in the range, count)
    pub range: Option<(LocalId, u64)>,
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use uuid::Uuid;

    use super::*;

    const LEADING_EDGE_OF_VERSION_SESSION_ID: &str = "00000000-0000-4fff-bfff-ffffffffffff";
    const TRAILING_EDGE_OF_VERSION_SESSION_ID: &str = "00000000-0001-4000-8000-000000000000";

    const _STABLE_IDS: &[&str] = &[
        "748540ca-b7c5-4c99-83ff-c1b8e02c09d6",
        "748540ca-b7c5-4c99-83ef-c1b8e02c09d6",
        "748540ca-b7c5-4c99-831f-c1b8e02c09d6",
        "0002c79e-b536-4776-b000-000266c252d5",
        "082533b9-6d05-4068-a008-fe2cc43543f7",
        "2c9fa1f8-48d5-4554-a466-000000000000",
        "2c9fa1f8-48d5-4000-a000-000000000000",
        "10000000-0000-4000-b000-000000000000",
        "10000000-0000-4000-b020-000000000000", // 2^52
        "10000000-0000-4000-b00f-ffffffffffff",
        "10000000-0000-4000-b040-000000000000",
        "f0000000-0000-4000-8000-000000000000",
        "efffffff-ffff-4fff-bfff-ffffffffffff",
        LEADING_EDGE_OF_VERSION_SESSION_ID,
    ];

    trait TestSessionSpaceId {
        fn unwrap_uuid_str(&self, compressor: &IdCompressor) -> String;
    }

    impl TestSessionSpaceId for SessionSpaceId {
        fn unwrap_uuid_str(&self, compressor: &IdCompressor) -> String {
            compressor.decompress(*self).unwrap().to_uuid_string()
        }
    }

    impl IdCompressor {
        // All helpers prefixed with 'h' to avoid polluting intellisense

        fn h_generate_n_ids(&mut self, num_ids: i32) -> Vec<SessionSpaceId> {
            let mut ids = Vec::new();
            for _ in 0..num_ids {
                ids.push(self.generate_next_id())
            }
            ids
        }

        fn h_finalize_next_range(&mut self) {
            let range = self.take_next_range();
            _ = self.finalize_range(&range);
        }
    }

    #[test]
    fn test_cluster_spans_reserved_bits() {
        let mut compressor = IdCompressor::new_with_session_id(
            SessionId::from_uuid_string(LEADING_EDGE_OF_VERSION_SESSION_ID).unwrap(),
        );

        let local_first = compressor.generate_next_id();
        assert_eq!(
            local_first.unwrap_uuid_str(&compressor),
            LEADING_EDGE_OF_VERSION_SESSION_ID
        );
        compressor.h_finalize_next_range();

        // Some eager finals, some locals
        let ids = compressor.h_generate_n_ids(10);
        compressor.h_finalize_next_range();

        let mut uuid_set = HashSet::new();
        for id in &ids {
            uuid_set.insert(id.unwrap_uuid_str(&compressor));
        }
        assert_eq!(uuid_set.len(), ids.len());
        let trailing_uuid = Uuid::try_parse(TRAILING_EDGE_OF_VERSION_SESSION_ID)
            .unwrap()
            .as_u128();
        for uuid_str in &uuid_set {
            let uuid = Uuid::try_parse(uuid_str).unwrap();
            assert!(uuid.as_u128() >= trailing_uuid);
        }
    }

    #[test]
    fn test_complex() {
        let mut compressor = IdCompressor::new();

        _ = compressor.set_cluster_capacity(3);

        // Before first cluster creation
        let session_space_id_1 = compressor.generate_next_id();
        let session_space_id_2 = compressor.generate_next_id();
        assert!(session_space_id_1.is_local());
        assert!(session_space_id_2.is_local());

        // Take initial range
        let out_range = compressor.take_next_range();

        // Finalize initial range
        assert!(compressor.finalize_range(&out_range).is_ok());

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
            let stable_id = StableId::from(compressor.session_id) + offset as u64;
            assert_eq!(compressor.decompress(id).unwrap(), stable_id,);
            assert_eq!(compressor.recompress(stable_id).unwrap(), id);

            let op_space_id = compressor.normalize_to_op_space(id).unwrap();
            assert_eq!(
                id,
                compressor
                    .normalize_to_session_space(op_space_id, compressor.session_id)
                    .unwrap()
            );
            if op_space_ids[offset] < 0 {
                assert_eq!(
                    op_space_id,
                    OpSpaceId::from(LocalId::from_id(op_space_ids[offset]))
                );
            } else {
                assert_eq!(
                    op_space_id,
                    OpSpaceId::from(FinalId::from_id(op_space_ids[offset] as u64))
                );
            }
            offset += 1;
        }
        // Serialize Deserialize
        let serialized = compressor.serialize(true);
        assert_eq!(compressor, IdCompressor::deserialize(&serialized).unwrap());
    }

    #[test]
    fn test_new_with_session_id() {
        let session_id = SessionId::new();
        let compressor = IdCompressor::new_with_session_id(session_id);
        assert_eq!(session_id, compressor.session_id);
    }

    #[test]
    fn test_cluster_capacity_validation() {
        let mut compressor = IdCompressor::new();
        assert!(compressor.set_cluster_capacity(0).is_err());
        assert!(compressor.set_cluster_capacity(1).is_ok());
        assert!(compressor.set_cluster_capacity(u64::MAX).is_ok())
    }

    #[test]
    fn test_decompress_recompress() {
        let mut compressor = IdCompressor::new();

        let session_space_id = compressor.generate_next_id();

        let stable_id = StableId::from(compressor.session_id);
        assert_eq!(compressor.decompress(session_space_id).unwrap(), stable_id,);
        assert_eq!(compressor.recompress(stable_id).unwrap(), session_space_id);
    }

    #[test]
    fn test_recompress_invalid() {
        let compressor = IdCompressor::new();
        let foreign_stable = StableId::from(SessionId::new());
        assert!(compressor.recompress(foreign_stable).is_err());
    }

    #[test]
    fn test_finalize_range_ordering() {
        let mut compressor = IdCompressor::new();
        _ = compressor.set_cluster_capacity(3);

        let _ = compressor.generate_next_id();
        let _ = compressor.generate_next_id();
        let out_range = compressor.take_next_range();

        // Finalize the same range twice
        assert!(compressor.finalize_range(&out_range).is_ok());
        assert!(compressor.finalize_range(&out_range).is_err());

        let mut compressor = IdCompressor::new();
        _ = compressor.set_cluster_capacity(3);

        let _ = compressor.generate_next_id();
        let _ = compressor.generate_next_id();
        let out_range_1 = compressor.take_next_range();
        let _ = compressor.generate_next_id();
        let _ = compressor.generate_next_id();
        let out_range_2 = compressor.take_next_range();

        // Finalize ranges out of order
        assert!(compressor.finalize_range(&out_range_2).is_err());
        assert!(compressor.finalize_range(&out_range_1).is_ok());
    }
}
