/*
Propose: rename to final_space_table
Vec will contain references to cluster chains

*/
use crate::compressor::tables::session_space::IdCluster;
use std::cell::RefCell;
use std::rc::Rc;

struct FinalSpace {
    // Sorted on final ID. Stores references to clusters held in some session space table.
    clusters: Vec<Rc<RefCell<IdCluster>>>,
}
