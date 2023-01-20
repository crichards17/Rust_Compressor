/*
This is an acceleration structure for the final_space_table.
*/

use super::session_space::{ClusterRef, IdCluster, SessionSpace, SessionSpaceRef};
use crate::compressor::utils::Dereferencer;
use crate::id_types::{SessionId, StableId};
use std::collections::BTreeMap;

type D = dyn Dereferencer<SessionSpaceRef, SessionSpace, ClusterRef, IdCluster>;

pub struct UuidSpace<'a> {
    uuid_to_cluster: BTreeMap<StableId, &'a IdCluster>,
}

impl<'a> UuidSpace<'a> {
    pub fn new() -> UuidSpace<'a> {
        UuidSpace {
            uuid_to_cluster: BTreeMap::new(),
        }
    }

    pub fn add_cluster(
        &mut self,
        session_id: SessionId,
        new_cluster: &'a IdCluster,
        dereferencer: D,
    ) {
        let base_stable = session_id + new_cluster.base_local_id;
        self.uuid_to_cluster.insert(base_stable, new_cluster);
    }
}
