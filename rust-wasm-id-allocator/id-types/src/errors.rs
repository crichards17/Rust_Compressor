#[derive(Debug)]
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

/// Defines a way to get an error string.
pub trait ErrorString {
    /// Returns the error string.
    fn to_error_string(&self) -> &str;
}

impl ErrorString for AllocatorError {
    /// Returns the string representation for the error variant.
    fn to_error_string(&self) -> &str {
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
