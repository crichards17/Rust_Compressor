/*
Propose: rename to final_space_table
Vec will contain references to cluster chains

*/
use super::session_space::ClusterRef;

pub struct FinalSpace {
    // Sorted on final ID. Stores references to clusters held in some session space table.
    clusters: Vec<ClusterRef>,
}

impl FinalSpace {
    pub fn new() -> FinalSpace {
        FinalSpace {
            clusters: Vec::new(),
        }
    }

    pub fn add_cluster(&mut self, new_cluster: ClusterRef) {
        self.clusters.push(new_cluster);
    }
}
