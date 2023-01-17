/*
The local/UUID space within an individual Session.
Effectively represents the cluster chain for a given session.
*/
use crate::id_types::*;
use std::cell::{RefCell, RefMut};
use std::rc::Rc;

pub struct SessionSpace {
    session_id: SessionId,
    // Sorted on LocalId.
    cluster_chain: Vec<Rc<RefCell<IdCluster>>>,
    rc_self: Option<Rc<RefCell<SessionSpace>>>,
}

impl SessionSpace {
    pub fn new(session_id: SessionId) -> Rc<RefCell<SessionSpace>> {
        let session_space = RefCell::new(SessionSpace {
            session_id,
            cluster_chain: Vec::new(),
            rc_self: None,
        });
        let rc_session_space = Rc::new(session_space);
        session_space.borrow_mut().rc_self = Some(rc_session_space.clone());
        rc_session_space
    }

    pub fn get_tail_cluster(&mut self) -> Option<RefMut<IdCluster>> {
        if self.cluster_chain.is_empty() {
            return None;
        }
        let len = self.cluster_chain.len();
        Some(self.cluster_chain[len - 1].borrow_mut())
    }

    pub fn add_cluster(&self) -> Rc<RefCell<IdCluster>> {
        let new_cluster: IdCluster = IdCluster {
            session_creator: self.rc_self.unwrap().clone(),
            base_final_id: FinalId { id: 5 },
            base_local_id: LocalId { id: -5 },
            capacity: 5,
            count: 5,
        };
        let rc_new_cluster = Rc::new(RefCell::new(new_cluster));
        self.cluster_chain.push(rc_new_cluster);
        rc_new_cluster
    }
}

pub struct IdCluster {
    session_creator: Rc<RefCell<SessionSpace>>,
    base_final_id: FinalId,
    base_local_id: LocalId,
    capacity: u64,
    count: u64,
}
