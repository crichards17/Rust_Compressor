use id_types::{FinalId, LocalId};

#[derive(Debug)]
pub struct IdCluster {
    base_final_id: FinalId,
    base_local_id: LocalId,
    capacity: u64,
    count: u64,
}

pub struct ClusterProperties {
    pub base_final_id: FinalId,
    pub base_local_id: LocalId,
    pub capacity: u64,
    pub count: u64,
}

impl IdCluster {
    pub fn new(
        base_final_id: FinalId,
        base_local_id: LocalId,
        capacity: u64,
        count: u64,
    ) -> IdCluster {
        IdCluster {
            base_final_id,
            base_local_id,
            capacity,
            count,
        }
    }

    pub fn base_final_id(&self) -> FinalId {
        self.base_final_id
    }

    pub fn base_local_id(&self) -> LocalId {
        self.base_local_id
    }

    pub fn capacity(&self) -> u64 {
        self.capacity
    }

    pub fn count(&self) -> u64 {
        self.count
    }

    pub fn set_capacity(&mut self, capacity: u64) {
        self.capacity = capacity;
    }

    pub fn set_count(&mut self, count: u64) {
        self.count = count;
    }

    pub fn properties(&self) -> ClusterProperties {
        ClusterProperties {
            base_final_id: self.base_final_id,
            base_local_id: self.base_local_id,
            capacity: self.capacity,
            count: self.count,
        }
    }
}

impl ClusterProperties {
    pub fn get_allocated_final(&self, local_within: LocalId) -> Option<FinalId> {
        let cluster_offset =
            local_within.to_generation_count() - self.base_local_id.to_generation_count();
        if cluster_offset < self.capacity {
            Some(self.base_final_id + cluster_offset)
        } else {
            None
        }
    }

    pub fn get_aligned_local(&self, contained_final: FinalId) -> Option<LocalId> {
        if contained_final < self.base_final_id || contained_final > self.max_allocated_final() {
            return None;
        }
        let final_delta = contained_final - self.base_final_id;
        Some(self.base_local_id - final_delta as u64)
    }

    pub fn max_allocated_final(&self) -> FinalId {
        self.base_final_id + (self.capacity - 1)
    }

    pub fn max_local(&self) -> LocalId {
        self.base_local_id - (self.count - 1)
    }

    pub fn max_allocated_local(&self) -> LocalId {
        self.base_local_id - (self.capacity - 1)
    }
}

impl PartialEq for IdCluster {
    fn eq(&self, other: &Self) -> bool {
        self.base_final_id == other.base_final_id
            && self.base_local_id == other.base_local_id
            && self.capacity == other.capacity
            && self.count == other.count
    }
}
