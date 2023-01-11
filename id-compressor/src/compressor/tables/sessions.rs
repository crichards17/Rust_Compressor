/*  Hash map to "Session Object"
    Session Object holds the other type of object, a Session Space Table,
        which is a cluster chain for that session.
*/

use crate::compressor::tables::session_space::SessionSpace;
use crate::id_types::SessionId;
use std::collections::HashMap;

pub struct Sessions<'a> {
    sessions: HashMap<SessionId, SessionSpace<'a>>,
}

impl<'a> Sessions<'a> {
    pub fn new() -> Sessions<'a> {
        Sessions {
            sessions: HashMap::new(),
        }
    }

    pub fn get_or_create(&mut self, session_id: SessionId) -> &mut SessionSpace<'a> {
        self.sessions
            .entry(session_id)
            .or_insert(SessionSpace::new(session_id))
    }

    pub fn get(&self, session_id: SessionId) -> Option<&SessionSpace<'a>> {
        self.sessions.get(&session_id)
    }
}
