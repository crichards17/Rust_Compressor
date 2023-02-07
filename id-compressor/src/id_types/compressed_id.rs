use super::{FinalId, LocalId};

pub enum CompressedId {
    Local(LocalId),
    Final(FinalId),
}
