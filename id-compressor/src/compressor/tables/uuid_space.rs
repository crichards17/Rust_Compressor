/*
This is an acceleration structure for the final_space_table.
*/

use crate::id_types::StableId;
use std::collections::BTreeMap;
use std::rc::Rc;

use super::session_space::IdCluster;

pub struct UuidSpace {
    uuid_to_cluster: BTreeMap<StableId, Rc<IdCluster>>,
}
