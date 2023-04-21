// #[derive(Debug)]
// pub enum DecompressionError {
//     UnableToDecompress,
// }

// #[derive(Debug)]
// pub enum RecompressionError {
//     InvalidStableId,
// }

// #[derive(Error, Debug)]
// pub enum FinalizationError {
//     #[error("Ranges finalized out of order.")]
//     RangeFinalizedOutOfOrder,
//     #[error("Malformed ID Range")]
//     MalformedIdRange,
//     #[error("Cluster collision detected.")]
//     ClusterCollision,
// }

// #[derive(Error, Debug)]
// pub enum ClusterCapacityError {
//     #[error("Cluster size must be a non-zero integer.")]
//     InvalidClusterCapacity,
// }

// #[derive(Error, Debug)]
// pub enum NormalizationError {
//     #[error("Unknown session space ID.")]
//     UnknownSessionSpaceId,
//     #[error("No IDs have ever been finalized by the supplied session.")]
//     NoTokenForSession,
// }

// #[derive(Error, Debug)]
// /// Error enum for generating SessionId_s.
// pub enum UuidGenerationError {
//     #[error("Invalid Uuid String")]
//     /// Invalid Uuid String
//     InvalidUuidString,
//     #[error("Invalid Version or Variant")]
//     /// Invalid Version or Variant
//     InvalidVersionOrVariant,
// }

/// Enumerated error variants for core allocator.
pub enum AllocatorError {
    /// Malformed string passed for UUID creation.
    InvalidUuidString,

    /// Resulting UUID is not V4 variant 1.
    InvalidVersionOrVariant,

    /// Cluster size must be a non-zero integer.
    InvalidClusterCapacity,

    /// ID Range not in sequential order when finalizing.
    RangeFinalizedOutOfOrder,

    /// Invalid ID Range data.
    MalformedIdRange,

    /// New cluster may collide.
    ClusterCollision,

    /// Failed to recompress StableId.
    InvalidStableId,

    /// Failed to decompress or normalize SessionSpaceId.
    InvalidSessionSpaceId,

    /// Failed to normalize to session space.
    InvalidOpSpaceId,

    /// Attempted to normalize an ID from an unknown session.
    NoTokenForSession,
}

impl AllocatorError {
    /// Returns the string representation for the error variant.
    pub fn to_string(&self) -> &str {
        match self {
            AllocatorError::InvalidUuidString => "String is not a valid UUID.",
            AllocatorError::InvalidVersionOrVariant => "String is not a V4 variant 1 UUID.",
            AllocatorError::InvalidClusterCapacity => "Cluster size must be a non-zero integer.",
            AllocatorError::RangeFinalizedOutOfOrder => "Ranges finalized out of order.",
            AllocatorError::MalformedIdRange => "Malformed ID Range.",
            AllocatorError::ClusterCollision => "Cluster collision detected.",
            AllocatorError::InvalidStableId => "Unknown stable ID.",
            AllocatorError::InvalidSessionSpaceId => "Unknown session space ID.",
            AllocatorError::InvalidOpSpaceId => "Unknown op space ID.",
            AllocatorError::NoTokenForSession => {
                "No IDs have ever been finalized by the supplied session."
            }
        }
    }
}
