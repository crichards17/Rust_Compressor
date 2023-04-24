/*
The local/UUID space within an individual Session.
Effectively represents the cluster chain for a given session.
*/
use avl::AvlTreeMap;
use id_types::session_id::from_stable_id;
use id_types::{FinalId, LocalId, SessionId, StableId};
use std::cmp::Ordering;
use std::ops::Bound;

#[derive(Debug)]
pub struct Sessions {
    session_map: AvlTreeMap<SessionId, SessionSpaceRef>,
    session_list: Vec<SessionSpace>,
}

impl Sessions {
    pub fn new() -> Sessions {
        Sessions {
            session_map: AvlTreeMap::new(),
            session_list: Vec::new(),
        }
    }

    pub fn get_or_create(&mut self, session_id: SessionId) -> SessionSpaceRef {
        match self.session_map.get(&session_id) {
            None => {
                let new_session_space_index = self.session_list.len();
                let new_session_space_ref = SessionSpaceRef {
                    index: new_session_space_index,
                };
                let new_session_space = SessionSpace::new(session_id, new_session_space_ref);
                self.session_map.insert(session_id, new_session_space_ref);
                self.session_list.push(new_session_space);
                new_session_space_ref
            }
            Some(session_ref) => *session_ref,
        }
    }

    pub fn get(&self, session_id: SessionId) -> Option<&SessionSpace> {
        match self.session_map.get(&session_id) {
            None => None,
            Some(session_space_ref) => Some(self.deref_session_space(*session_space_ref)),
        }
    }

    pub fn deref_session_space_mut(
        &mut self,
        session_space_ref: SessionSpaceRef,
    ) -> &mut SessionSpace {
        &mut self.session_list[session_space_ref.index]
    }

    pub fn deref_session_space(&self, session_space_ref: SessionSpaceRef) -> &SessionSpace {
        debug_assert!(
            session_space_ref.index < self.session_list.len(),
            "Out of bounds session space ref."
        );
        &self.session_list[session_space_ref.index]
    }

    pub fn deref_cluster_mut(&mut self, cluster_ref: ClusterRef) -> &mut IdCluster {
        &mut self
            .deref_session_space_mut(cluster_ref.session_space_ref)
            .cluster_chain[cluster_ref.cluster_chain_index]
    }

    pub fn deref_cluster(&self, cluster_ref: ClusterRef) -> &IdCluster {
        &self
            .deref_session_space(cluster_ref.session_space_ref)
            .cluster_chain[cluster_ref.cluster_chain_index]
    }

    pub fn get_session_spaces(&self) -> impl Iterator<Item = &SessionSpace> {
        self.session_list.iter()
    }

    pub fn get_containing_cluster(&self, query: StableId) -> Option<(&IdCluster, LocalId)> {
        let mut range = self
            .session_map
            .range((
                Bound::Excluded(SessionId::nil()),
                Bound::Included(from_stable_id(query)),
            ))
            .rev();
        match range.next() {
            None => None,
            Some((_, &session_space_ref)) => {
                let session_space = self.deref_session_space(session_space_ref);
                if query > session_space.get_max_allocated_stable() {
                    return None;
                }
                let delta: u128 = query - StableId::from(session_space.session_id);
                let aligned_local = LocalId::from_generation_count(delta as u64 + 1);
                match session_space.get_cluster_by_local(aligned_local, true) {
                    Some(cluster_match) => {
                        let result_session_id = self
                            .deref_session_space(cluster_match.session_creator)
                            .session_id();
                        let cluster_min_stable = result_session_id + cluster_match.base_local_id;
                        let cluster_max_stable = cluster_min_stable + cluster_match.capacity;
                        if query >= cluster_min_stable && query <= cluster_max_stable {
                            let originator_local = LocalId::from_id(
                                -((query - StableId::from(result_session_id)) as i64) - 1,
                            );
                            Some((cluster_match, originator_local))
                        } else {
                            None
                        }
                    }
                    None => None,
                }
            }
        }
    }

    pub fn range_collides(
        &self,
        originator: SessionId,
        range_base: StableId,
        range_max: StableId,
    ) -> bool {
        let mut range = self
            .session_map
            .range((
                Bound::Excluded(SessionId::nil()),
                Bound::Included(from_stable_id(range_max)),
            ))
            .rev();
        match range.next() {
            None => false,
            Some((_, &session_space_ref)) => {
                let result_session_space = self.deref_session_space(session_space_ref);
                originator != result_session_space.session_id
                    && range_base <= result_session_space.get_max_allocated_stable()
            }
        }
    }

    #[cfg(debug_assertions)]
    pub(crate) fn equals_test_only(&self, other: &Sessions) -> bool {
        fn get_sorted_sessions(session_space: &Sessions) -> impl Iterator<Item = &SessionSpace> {
            let mut filtered: Vec<&SessionSpace> = session_space
                .session_list
                .iter()
                .filter(|&session_space| !session_space.cluster_chain.is_empty())
                .collect();
            filtered.sort_by(|session_space_a, session_space_b| {
                session_space_a.session_id.cmp(&session_space_b.session_id)
            });
            filtered.into_iter()
        }
        let mut filtered_a = get_sorted_sessions(self);
        let mut filtered_b = get_sorted_sessions(other);
        loop {
            let session_space_a = filtered_a.next();
            let session_space_b = filtered_b.next();
            if session_space_a.is_none() != session_space_b.is_none() {
                return false;
            }
            if session_space_a.is_none() && session_space_b.is_none() {
                return true;
            }
            let session_space_a = session_space_a.unwrap();
            let session_space_b = session_space_b.unwrap();
            if session_space_a.session_id != session_space_b.session_id
                || session_space_a.cluster_chain != session_space_b.cluster_chain
            {
                return false;
            }
            if !self.session_map.contains_key(&session_space_a.session_id)
                || !other.session_map.contains_key(&session_space_b.session_id)
            {
                return false;
            }
        }
    }
}

#[derive(Debug)]
pub struct SessionSpace {
    session_id: SessionId,
    self_ref: SessionSpaceRef,
    // Sorted on LocalId.
    cluster_chain: Vec<IdCluster>,
}

impl SessionSpace {
    pub fn new(session_id: SessionId, self_ref: SessionSpaceRef) -> SessionSpace {
        SessionSpace {
            session_id,
            self_ref,
            cluster_chain: Vec::new(),
        }
    }

    pub fn session_id(&self) -> SessionId {
        self.session_id
    }

    pub fn self_ref(&self) -> SessionSpaceRef {
        self.self_ref
    }

    pub fn get_tail_cluster(&self) -> Option<ClusterRef> {
        if self.cluster_chain.is_empty() {
            return None;
        }
        Some(ClusterRef {
            session_space_ref: self.self_ref,
            cluster_chain_index: self.cluster_chain.len() - 1,
        })
    }

    fn get_max_allocated_stable(&self) -> StableId {
        let tail_cluster = match self.get_tail_cluster() {
            Some(cluster_ref) => &self.cluster_chain[cluster_ref.cluster_chain_index],
            None => return self.session_id.into(),
        };
        self.session_id + tail_cluster.max_allocated_local()
    }

    pub fn add_empty_cluster(
        &mut self,
        base_final_id: FinalId,
        base_local_id: LocalId,
        capacity: u64,
    ) -> ClusterRef {
        let new_cluster = IdCluster {
            session_creator: self.self_ref,
            base_final_id,
            base_local_id,
            capacity,
            count: 0,
        };
        self.add_cluster(new_cluster)
    }

    pub fn add_cluster(&mut self, new_cluster: IdCluster) -> ClusterRef {
        self.cluster_chain.push(new_cluster);
        let tail_index = self.cluster_chain.len() - 1;
        ClusterRef {
            session_space_ref: self.self_ref,
            cluster_chain_index: tail_index,
        }
    }

    pub fn try_convert_to_final(
        &self,
        search_local: LocalId,
        include_allocated: bool,
    ) -> Option<FinalId> {
        match self.get_cluster_by_local(search_local, include_allocated) {
            Some(found_cluster) => Some(found_cluster.get_allocated_final(search_local).unwrap()),
            None => None,
        }
    }

    fn get_cluster_by_local(
        &self,
        search_local: LocalId,
        include_allocated: bool,
    ) -> Option<&IdCluster> {
        let last_valid_local: fn(current_cluster: &IdCluster) -> u64 = if include_allocated {
            |current_cluster| current_cluster.capacity - 1
        } else {
            |current_cluster| current_cluster.count - 1
        };

        match self.cluster_chain.binary_search_by(|current_cluster| {
            let cluster_last_local =
                current_cluster.base_local_id - last_valid_local(current_cluster);
            if cluster_last_local > search_local {
                Ordering::Less
            } else if current_cluster.base_local_id < search_local {
                return Ordering::Greater;
            } else {
                Ordering::Equal
            }
        }) {
            Ok(index) => Some(&self.cluster_chain[index]),
            Err(_) => None,
        }
    }

    // TODO: include contract about allocated not finalized
    pub fn get_cluster_by_allocated_final(&self, search_final: FinalId) -> Option<&IdCluster> {
        match self.cluster_chain.binary_search_by(|current_cluster| {
            let cluster_base_final = current_cluster.base_final_id;
            let cluster_last_final = cluster_base_final + (current_cluster.capacity - 1);
            if cluster_last_final < search_final {
                Ordering::Less
            } else if cluster_base_final > search_final {
                return Ordering::Greater;
            } else {
                Ordering::Equal
            }
        }) {
            Ok(found_cluster_index) => Some(&self.cluster_chain[found_cluster_index]),
            Err(_) => None,
        }
    }
}

#[derive(Debug)]
pub struct IdCluster {
    pub(crate) session_creator: SessionSpaceRef,
    pub(crate) base_final_id: FinalId,
    pub(crate) base_local_id: LocalId,
    pub(crate) capacity: u64,
    pub(crate) count: u64,
}

impl IdCluster {
    pub fn get_allocated_final(&self, local_within: LocalId) -> Option<FinalId> {
        let cluster_offset =
            local_within.to_generation_count() - self.base_local_id.to_generation_count();
        if cluster_offset < self.capacity {
            Some(self.base_final_id + cluster_offset)
        } else {
            None
        }
    }

    pub fn get_aligned_local(&self, contained_final: FinalId) -> Option<LocalId> {
        if contained_final < self.base_final_id || contained_final > self.max_allocated_final() {
            return None;
        }
        let final_delta = contained_final - self.base_final_id;
        Some(self.base_local_id - final_delta as u64)
    }

    pub fn max_allocated_final(&self) -> FinalId {
        self.base_final_id + (self.capacity - 1)
    }

    pub fn max_local(&self) -> LocalId {
        self.base_local_id - (self.count - 1)
    }

    pub fn max_allocated_local(&self) -> LocalId {
        self.base_local_id - (self.capacity - 1)
    }
}

impl PartialEq for IdCluster {
    fn eq(&self, other: &Self) -> bool {
        self.base_final_id == other.base_final_id
            && self.base_local_id == other.base_local_id
            && self.capacity == other.capacity
            && self.count == other.count
    }
}

// Maps to an index in the session_list
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct SessionSpaceRef {
    index: usize,
}

impl SessionSpaceRef {
    pub fn get_index(&self) -> usize {
        self.index
    }

    pub fn create_from_token(token: i64) -> SessionSpaceRef {
        debug_assert!(token >= 0, "Nil token passed as session space ref.");
        SessionSpaceRef {
            index: token as usize,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ClusterRef {
    session_space_ref: SessionSpaceRef,
    cluster_chain_index: usize,
}

#[cfg(debug_assertions)]
impl ClusterRef {
    pub(crate) fn equals_test_only(
        &self,
        other: &ClusterRef,
        sessions_self: &Sessions,
        sessions_other: &Sessions,
    ) -> bool {
        let session_id_a = sessions_self
            .deref_session_space(self.session_space_ref)
            .session_id();
        let session_id_b = sessions_other
            .deref_session_space(other.session_space_ref)
            .session_id();
        session_id_a == session_id_b && self.cluster_chain_index == other.cluster_chain_index
    }
}
