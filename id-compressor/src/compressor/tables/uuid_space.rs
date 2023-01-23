/*
This is an acceleration structure for the final_space_table.
*/

use super::session_space::{ClusterRef, Sessions};
use crate::id_types::{SessionId, StableId};
use std::collections::BTreeMap;

pub struct UuidSpace {
    uuid_to_cluster: BTreeMap<StableId, ClusterRef>,
}

impl UuidSpace {
    pub fn new() -> UuidSpace {
        UuidSpace {
            uuid_to_cluster: BTreeMap::new(),
        }
    }

    pub fn add_cluster(
        &mut self,
        session_id: SessionId,
        new_cluster_ref: ClusterRef,
        sessions: &Sessions,
    ) {
        let base_stable = session_id + sessions.deref_cluster(new_cluster_ref).base_local_id;
        self.uuid_to_cluster.insert(base_stable, new_cluster_ref);
    }
}
