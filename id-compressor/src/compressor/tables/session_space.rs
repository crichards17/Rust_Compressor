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
                let new_session_space = SessionSpace::new(session_id);
                self.session_list.push(new_session_space);
                let new_session_space_index = self.session_list.len() - 1;
                let new_session_space_ref = SessionSpaceRef {
                    index: new_session_space_index,
                };
                self.session_map.insert(session_id, new_session_space_ref);
                new_session_space_ref
            }
            Some(session_ref) => *session_ref,
        }
    }

    pub fn get(&mut self, session_id: SessionId) -> Option<&SessionSpace> {
        match self.session_map.get(&session_id).copied() {
            None => None,
            Some(session_space_ref) => Some(self.deref(&session_space_ref)),
        }
    }

    pub fn deref(&mut self, session_space_ref: &SessionSpaceRef) -> &mut SessionSpace {
        &mut self.session_list[session_space_ref.index]
    }
}

pub struct SessionSpace {
    session_id: SessionId,
    // Sorted on LocalId.
    cluster_chain: Vec<IdCluster>,
}

impl SessionSpace {
    pub fn new(session_id: SessionId) -> SessionSpace {
        SessionSpace {
            session_id,
            cluster_chain: Vec::new(),
        }
    }

    pub fn get_tail_cluster(&mut self) -> Option<&mut IdCluster> {
        if self.cluster_chain.is_empty() {
            return None;
        }
        let len = self.cluster_chain.len();
        Some(&mut self.cluster_chain[len - 1])
    }

    pub fn add_cluster(
        &mut self,
        session_creator: SessionSpaceRef,
        base_final_id: FinalId,
        base_local_id: LocalId,
        capacity: u64,
    ) -> &mut IdCluster {
        let new_cluster = IdCluster {
            session_creator,
            base_final_id,
            base_local_id,
            capacity,
            count: 0,
        };
        self.cluster_chain.push(new_cluster);
        let tail_index = self.cluster_chain.len() - 1;
        &mut self.cluster_chain[tail_index]
    }
}

pub struct IdCluster {
    session_creator: SessionSpaceRef,
    base_final_id: FinalId,
    base_local_id: LocalId,
    capacity: u64,
    count: u64,
}

#[derive(Clone, Copy)]
pub struct SessionSpaceRef {
    index: usize,
}
