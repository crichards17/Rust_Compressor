use id_types::FinalId;

/*
Propose: rename to final_space_table
Vec will contain references to cluster chains

*/
use super::session_space::{ClusterRef, IdCluster, Sessions};
use std::cmp::Ordering;

#[derive(Debug)]
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

    pub fn add_cluster(&mut self, new_cluster_ref: ClusterRef, _sessions: &Sessions) {
        #[cfg(debug_assertions)]
        if self.clusters.len() != 0 {
            let new_cluster_base_final = _sessions.deref_cluster(new_cluster_ref).base_final_id;
            let last_cluster_base_final = _sessions
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

    // Searches the Final table for a cluster whose capacity would include the given Final.
    //   Does not guarantee that the Final has been allocated to the returned cluster.
    pub fn search<'a>(
        &self,
        target_final: FinalId,
        sessions: &'a Sessions,
    ) -> Option<&'a IdCluster> {
        self.clusters
            .binary_search_by(|current_cluster_ref| {
                let current_cluster = sessions.deref_cluster(*current_cluster_ref);
                let cluster_base_final = current_cluster.base_final_id;
                let cluster_max_final = cluster_base_final + (current_cluster.capacity - 1);
                if cluster_max_final < target_final {
                    return Ordering::Less;
                } else if cluster_base_final > target_final {
                    return Ordering::Greater;
                } else {
                    Ordering::Equal
                }
            })
            .ok()
            .map(|index| sessions.deref_cluster(self.clusters[index]))
    }

    pub fn get_tail_cluster<'a>(&self, sessions: &'a Sessions) -> Option<&'a IdCluster> {
        if self.clusters.is_empty() {
            return None;
        }
        Some(sessions.deref_cluster(self.clusters[self.clusters.len() - 1]))
    }

    pub fn get_clusters<'a>(
        &'a self,
        sessions: &'a Sessions,
    ) -> impl Iterator<Item = &'a IdCluster> {
        self.clusters
            .iter()
            .map(|cluster_ref| sessions.deref_cluster(*cluster_ref))
    }

    #[cfg(debug_assertions)]
    pub(crate) fn equals_test_only(
        &self,
        other: &FinalSpace,
        sessions_self: &Sessions,
        sessions_other: &Sessions,
    ) -> bool {
        if self.clusters.len() != other.clusters.len() {
            return false;
        }
        for i in 0..self.clusters.len() {
            if !self.clusters[i].equals_test_only(&other.clusters[i], sessions_self, sessions_other)
            {
                return false;
            }
        }
        return true;
    }
}
