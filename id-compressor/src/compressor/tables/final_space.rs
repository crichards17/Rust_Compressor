/*
Propose: rename to final_space_table
Vec will contain references to cluster chains

*/
use super::session_space::{ClusterRef, Sessions};

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

    pub fn add_cluster(&mut self, new_cluster_ref: ClusterRef, sessions: &Sessions) {
        #[cfg(debug_assertions)]
        if self.clusters.len() != 0 {
            let new_cluster_base_final = sessions.deref_cluster(new_cluster_ref).base_final_id;
            let last_cluster_base_final = sessions
                .deref_cluster(self.clusters[self.clusters.len() - 1])
                .base_final_id;
            assert!(
                new_cluster_base_final > last_cluster_base_final,
                "Cluster insert to final_space is out of order."
            )
        }

        self.clusters.push(new_cluster_ref);
    }

    pub fn is_last(&self, cluster_ref: ClusterRef) -> bool {
        cluster_ref == self.clusters[self.clusters.len() - 1]
    }
}
