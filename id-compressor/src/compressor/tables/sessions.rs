/*  Hash map to "Session Object"
    Session Object holds the other type of object, a Session Space Table,
        which is a cluster chain for that session.
*/

use crate::compressor::tables::session_space::SessionSpace;
use crate::id_types::SessionId;
use std::cell::{RefCell, RefMut};
use std::collections::HashMap;
use std::rc::Rc;

pub struct Sessions {
    sessions: HashMap<SessionId, Rc<RefCell<SessionSpace>>>,
}

impl Sessions {
    pub fn new() -> Sessions {
        Sessions {
            sessions: HashMap::new(),
        }
    }

    pub fn get_or_create(&mut self, session_id: SessionId) -> Rc<RefCell<SessionSpace>> {
        (self
            .sessions
            .entry(session_id)
            .or_insert(SessionSpace::new(session_id)))
        .clone()
    }

    pub fn get(&self, session_id: SessionId) -> Option<Rc<RefCell<SessionSpace>>> {
        match self.sessions.get(&session_id) {
            None => None,
            Some(rc) => Some(rc.clone()),
        }
    }
}
