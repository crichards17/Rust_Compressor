/*
The local/UUID space within an individual Session.
Effectively represents the cluster chain for a given session.
*/
use crate::id_types::*;

pub struct SessionSpace<'a> {
    session_id: SessionId,
    // Sorted on LocalId.
    cluster_chain: Vec<IdCluster<'a>>,
}

pub struct IdCluster<'a> {
    session_id: &'a SessionSpace<'a>,
    base_final_id: FinalId,
    base_local_id: LocalId,
    capacity: u64,
    count: u64,
}
