/*
The local/UUID space within an individual Session.
Effectively represents the cluster chain for a given session.
*/
use crate::id_types::*;

pub struct SessionSpace<'a> {
    session_id: SessionId,
    // Sorted on LocalId.
    cluster_chain: Vec<IdCluster<'a>>,
}

impl<'a> SessionSpace<'a> {
    pub fn new(session_id: SessionId) -> SessionSpace<'a> {
        SessionSpace {
            session_id,
            cluster_chain: Vec::new(),
        }
    }

    pub fn get_tail_cluster(&mut self) -> Option<&mut IdCluster<'a>> {
        if self.cluster_chain.is_empty() {
            return None;
        }
        let len = self.cluster_chain.len();
        Some(&mut self.cluster_chain[len - 1])
    }

    pub fn add_cluster(&mut self) -> &mut IdCluster<'a> {
        let new_cluster: IdCluster<'a> = IdCluster {
            session_creator: self,
            base_final_id: FinalId { id: 5 },
            base_local_id: LocalId { id: -5 },
            capacity: 5,
            count: 5,
        };
        self.cluster_chain.push(new_cluster);
        &mut new_cluster
    }
}

pub struct IdCluster<'a> {
    session_creator: &'a SessionSpace<'a>,
    base_final_id: FinalId,
    base_local_id: LocalId,
    capacity: u64,
    count: u64,
}
