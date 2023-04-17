use super::{FinalId, LocalId};

/// An identifier (v4 UUID) that has been shortened by a distributed compression algorithm.
/// Lacks a space (session/op), meaning its scope is the same as the space-specific ID from which it was derived.
pub enum CompressedId {
    /// A local ID variant
    Local(LocalId),
    /// A final ID variant
    Final(FinalId),
}
