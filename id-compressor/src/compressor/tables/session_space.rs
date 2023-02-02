/*
The local/UUID space within an individual Session.
Effectively represents the cluster chain for a given session.
*/
use crate::id_types::SessionId;
use crate::id_types::*;
use std::collections::HashMap;

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

    pub fn get_or_create(&mut self, session_id: SessionId) -> SessionSpaceRef {
        match self.session_map.get(&session_id) {
            None => {
                let new_session_space_index = self.session_list.len();
                let new_session_space_ref = SessionSpaceRef {
                    index: new_session_space_index,
                };
                let new_session_space = SessionSpace::new(session_id, new_session_space_ref);
                self.session_list.push(new_session_space);
                self.session_map.insert(session_id, new_session_space_ref);
                new_session_space_ref
            }
            Some(session_ref) => *session_ref,
        }
    }

    pub fn get(&mut self, session_id: SessionId) -> Option<&SessionSpace> {
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
}

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

    pub fn add_cluster(
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
        self.cluster_chain.push(new_cluster);
        let tail_index = self.cluster_chain.len() - 1;
        ClusterRef {
            session_space_ref: self.self_ref,
            cluster_chain_index: tail_index,
        }
    }
}

pub struct IdCluster {
    pub(crate) session_creator: SessionSpaceRef,
    pub(crate) base_final_id: FinalId,
    pub(crate) base_local_id: LocalId,
    pub(crate) capacity: u64,
    pub(crate) count: u64,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct SessionSpaceRef {
    index: usize,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct ClusterRef {
    session_space_ref: SessionSpaceRef,
    cluster_chain_index: usize,
}
