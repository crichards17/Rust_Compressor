/*
Propose: rename to final_space_table
Vec will contain references to cluster chains

*/
use crate::compressor::tables::session_space::IdCluster;

pub struct FinalSpace<'a> {
    // Sorted on final ID. Stores references to clusters held in some session space table.
    clusters: Vec<&'a IdCluster>,
}

impl<'a> FinalSpace<'a> {
    pub fn new() -> FinalSpace<'a> {
        FinalSpace {
            clusters: Vec::new(),
        }
    }
}
