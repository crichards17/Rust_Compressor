/*
This is an acceleration structure for the final_space_table.
*/

use super::session_space::{ClusterRef, IdCluster, Sessions};
use crate::id_types::{LocalId, SessionId, StableId};
use std::collections::BTreeMap;
use std::ops::Bound;

#[derive(PartialEq, Eq, Debug)]
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
        let base_stable = session_id
            .stable_from_local_offset(sessions.deref_cluster(new_cluster_ref).base_local_id);
        self.uuid_to_cluster.insert(base_stable, new_cluster_ref);
    }

    // Returns the cluster in which the queried StableId has been allocated. Does not guarantee that this ID has been generated nor finalized.
    pub fn search<'a>(
        &self,
        query: StableId,
        sessions: &'a Sessions,
    ) -> Option<(&'a IdCluster, LocalId)> {
        let mut range = self
            .uuid_to_cluster
            .range((Bound::Excluded(StableId::null()), Bound::Included(query)))
            .rev();
        match range.next() {
            None => None,
            Some((_, &cluster_ref)) => {
                let cluster_match = sessions.deref_cluster(cluster_ref);
                let result_session_id = sessions
                    .deref_session_space(cluster_match.session_creator)
                    .session_id();
                let cluster_min_stable =
                    result_session_id.stable_from_local_offset(cluster_match.base_local_id);
                let cluster_max_stable = cluster_min_stable + cluster_match.capacity;
                if query >= cluster_min_stable && query <= cluster_max_stable {
                    let originator_local =
                        LocalId::from_id(-((query - StableId::from(result_session_id)) as i64) - 1);
                    return Some((cluster_match, originator_local));
                } else {
                    None
                }
            }
        }
    }
}
