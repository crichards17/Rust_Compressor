/*
Propose: rename to final_space_table
Vec will contain references to cluster chains

*/
use crate::compressor::tables::session_space::IdCluster;

struct FinalSpace<'a> {
    // Sorted on final ID. Stores references to clusters held in some session space table.
    clusters: Vec<&'a IdCluster>,
}
