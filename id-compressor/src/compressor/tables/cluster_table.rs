pub mod uuid_table;
use crate::id_types::*;
use uuid_table::UuidTable;

struct Cluster {
    session_id: SessionId, // CAN DO: Update to session table reference / index
    base_final_id: FinalId,
    base_local_id: LocalId,
    capacity: u64,
    count: u64,
}

struct ClusterTable {
    clusters: Vec<Cluster>,
    uuid_table: UuidTable,
}
