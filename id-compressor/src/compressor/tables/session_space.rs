/*
The local/UUID space within an individual Session.
Effectively represents the cluster chain for a given session.
*/
use crate::id_types::SessionId;
use crate::id_types::*;
use std::cmp::Ordering;
use std::collections::HashMap;

#[derive(PartialEq, Eq, Debug)]
pub struct Sessions {
    session_map: HashMap<SessionId, SessionSpaceRef>,
    session_list: Vec<SessionSpace>,
}

impl Sessions {
    pub fn new() -> Sessions {
        Sessions {
            session_map: HashMap::new(),
            session_list: Vec::new(),
        }
    }

    pub fn sessions_count(&self) -> usize {
        self.session_list.len()
    }

    pub fn get_or_create(&mut self, session_id: SessionId) -> SessionSpaceRef {
        match self.session_map.get(&session_id) {
            None => self.create(session_id),
            Some(session_ref) => *session_ref,
        }
    }

    pub(crate) fn create(&mut self, session_id: SessionId) -> SessionSpaceRef {
        let new_session_space_index = self.session_list.len();
        let new_session_space_ref = SessionSpaceRef {
            index: new_session_space_index,
        };
        let new_session_space = SessionSpace::new(session_id, new_session_space_ref);
        self.session_list.push(new_session_space);
        self.session_map.insert(session_id, new_session_space_ref);
        new_session_space_ref
    }

    pub fn get(&self, session_id: SessionId) -> Option<&SessionSpace> {
        match self.session_map.get(&session_id) {
            None => None,
            Some(session_space_ref) => Some(self.deref_session_space(*session_space_ref)),
        }
    }

    pub fn get_mut(&mut self, session_id: SessionId) -> Option<&mut SessionSpace> {
        match self.session_map.get(&session_id) {
            None => None,
            Some(session_space_ref) => Some(self.deref_session_space_mut(*session_space_ref)),
        }
    }

    pub fn deref_session_space_mut(
        &mut self,
        session_space_ref: SessionSpaceRef,
    ) -> &mut SessionSpace {
        &mut self.session_list[session_space_ref.index]
    }

    pub fn deref_session_space(&self, session_space_ref: SessionSpaceRef) -> &SessionSpace {
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
}

#[derive(PartialEq, Eq, Debug)]
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

    pub fn get_tail_cluster(&self) -> Option<ClusterRef> {
        if self.cluster_chain.is_empty() {
            return None;
        }
        Some(ClusterRef {
            session_space_ref: self.self_ref,
            cluster_chain_index: self.cluster_chain.len() - 1,
        })
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

    pub fn try_convert_to_final(&self, search_local: LocalId) -> Option<FinalId> {
        match self.cluster_chain.binary_search_by(|current_cluster| {
            let cluster_last_local = current_cluster.base_local_id - (current_cluster.count - 1);
            if cluster_last_local > search_local {
                return Ordering::Less;
            } else if current_cluster.base_local_id < search_local {
                return Ordering::Greater;
            } else {
                Ordering::Equal
            }
        }) {
            Ok(index) => {
                let found_cluster = &self.cluster_chain[index];
                Some(found_cluster.get_allocated_final(search_local).unwrap())
            }
            Err(_) => None,
        }
    }

    // TODO: include contract about allocated not finalized
    pub fn get_cluster_by_allocated_final(&self, search_final: FinalId) -> Option<&IdCluster> {
        match self.cluster_chain.binary_search_by(|current_cluster| {
            let cluster_base_final = current_cluster.base_final_id;
            let cluster_last_final = cluster_base_final + (current_cluster.capacity - 1);
            if cluster_last_final < search_final {
                return Ordering::Less;
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

#[derive(PartialEq, Eq, Debug)]
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
            (local_within.to_generation_count() - self.base_local_id.to_generation_count()) as u64;
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

    pub fn max_final(&self) -> FinalId {
        self.base_final_id + (self.count - 1)
    }

    pub fn max_allocated_final(&self) -> FinalId {
        self.base_final_id + (self.capacity - 1)
    }

    pub fn max_local(&self) -> LocalId {
        self.base_local_id - (self.count - 1)
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

    pub fn create_from_index(index: usize) -> SessionSpaceRef {
        SessionSpaceRef { index }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ClusterRef {
    session_space_ref: SessionSpaceRef,
    cluster_chain_index: usize,
}
