#![deny(
    bad_style,
    dead_code,
    improper_ctypes,
    non_shorthand_field_patterns,
    no_mangle_generic_items,
    overflowing_literals,
    path_statements,
    patterns_in_fns_without_body,
    private_in_public,
    unconditional_recursion,
    unused,
    unused_allocation,
    unused_comparisons,
    unused_parens,
    while_true,
    missing_docs,
    trivial_casts,
    trivial_numeric_casts,
    unused_extern_crates,
    unused_import_braces,
    unused_qualifications,
    unused_results
)]

//! Holds the definitions and helpers for identifier types used in the compressor code.
//! It is separated from the other code to allow the wasm-id-allocator module to access internals without
//! exposing them through the distributed-id-allocator public API.

/// File-separated module for session space IDs.
pub mod session_space_id;
pub use session_space_id::SessionSpaceId;

/// File-separated module for op space IDs.
pub mod op_space_id;
pub use op_space_id::OpSpaceId;

/// File-separated module for local IDs.
pub mod local_id;
pub use local_id::LocalId;

/// File-separated module for final IDs.
pub mod final_id;
pub use final_id::FinalId;

/// File-separated module for stable IDs.
pub mod stable_id;
pub use stable_id::StableId;

/// File-separated module for session IDs.
pub mod session_id;
pub use session_id::SessionId;

/// File-separated module for compressed IDs.
pub mod compressed_id;
pub use compressed_id::CompressedId;

/// File-separated module for error enumeration
pub mod errors;
pub use errors::AllocatorError;
