use std::num::NonZeroU32;

use id_types::{
    final_id::{final_id_from_id, get_id_from_final_id},
    FinalId, LocalId,
};

#[derive(PartialEq, Debug)]
pub struct IdCluster {
    cluster: ClusterType,
}

#[derive(PartialEq, Debug)]
struct SmallCluster {
    base_final_id: u32,
    base_local_gen_count: NonZeroU32,
    capacity: NonZeroU32,
    count: u32,
}

#[derive(PartialEq, Debug)]
enum ClusterType {
    Big(Box<ClusterProperties>),
    Small(SmallCluster),
}

#[derive(PartialEq, Clone, Copy, Debug)]
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
        if get_id_from_final_id(base_final_id) > u32::MAX as u64
            || base_local_id.to_generation_count() > u32::MAX as u64
            || capacity > u32::MAX as u64
        {
            IdCluster {
                cluster: ClusterType::Big(Box::new(ClusterProperties {
                    base_final_id,
                    base_local_id,
                    capacity,
                    count,
                })),
            }
        } else {
            IdCluster {
                cluster: ClusterType::Small(SmallCluster {
                    base_final_id: get_id_from_final_id(base_final_id) as u32,
                    base_local_gen_count: NonZeroU32::new(
                        base_local_id.to_generation_count() as u32
                    )
                    .unwrap(),
                    capacity: NonZeroU32::new(capacity as u32).unwrap(),
                    count: count as u32,
                }),
            }
        }
    }

    pub fn base_final_id(&self) -> FinalId {
        match &self.cluster {
            ClusterType::Big(boxed) => boxed.base_final_id,
            ClusterType::Small(small) => final_id_from_id(small.base_final_id as u64),
        }
    }

    pub fn base_local_id(&self) -> LocalId {
        match &self.cluster {
            ClusterType::Big(boxed) => boxed.base_local_id,
            ClusterType::Small(small) => {
                LocalId::from_generation_count(small.base_local_gen_count.get() as u64)
            }
        }
    }

    pub fn capacity(&self) -> u64 {
        match &self.cluster {
            ClusterType::Big(boxed) => boxed.capacity,
            ClusterType::Small(small) => small.capacity.get() as u64,
        }
    }

    pub fn count(&self) -> u64 {
        match &self.cluster {
            ClusterType::Big(boxed) => boxed.count,
            ClusterType::Small(small) => small.count as u64,
        }
    }

    pub fn set_capacity(&mut self, capacity: u64) {
        match &mut self.cluster {
            ClusterType::Big(boxed) => boxed.capacity = capacity,
            ClusterType::Small(small) => {
                if capacity > u32::MAX.into() {
                    self.cluster = ClusterType::Big(Box::new(self.properties()));
                    self.set_capacity(capacity)
                } else {
                    small.capacity = NonZeroU32::new(capacity as u32).unwrap()
                }
            }
        }
    }

    pub fn set_count(&mut self, count: u64) {
        match &mut self.cluster {
            ClusterType::Big(boxed) => boxed.count = count,
            ClusterType::Small(small) => {
                if count > u32::MAX.into() {
                    self.cluster = ClusterType::Big(Box::new(self.properties()));
                    self.set_count(count)
                } else {
                    small.count = count as u32
                }
            }
        }
    }

    pub fn properties(&self) -> ClusterProperties {
        match &self.cluster {
            ClusterType::Big(boxed) => *boxed.as_ref(),
            ClusterType::Small(small) => ClusterProperties {
                base_final_id: final_id_from_id(small.base_final_id as u64),
                base_local_id: LocalId::from_generation_count(
                    small.base_local_gen_count.get() as u64
                ),
                capacity: small.capacity.get() as u64,
                count: small.count as u64,
            },
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
