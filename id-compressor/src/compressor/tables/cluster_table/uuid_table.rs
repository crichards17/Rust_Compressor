use crate::id_types::StableId;
use std::collections::BTreeMap;

pub struct UuidTable {
    uuid_to_cluster: BTreeMap<StableId, u64>,
}
