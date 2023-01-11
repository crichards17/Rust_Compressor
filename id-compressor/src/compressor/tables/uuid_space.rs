/*
This is an acceleration structure for the final_space_table.
*/

use crate::id_types::StableId;
use std::collections::BTreeMap;

use super::session_space::IdCluster;

pub struct UuidSpace<'a> {
    uuid_to_cluster: BTreeMap<StableId, &'a IdCluster<'a>>,
}
