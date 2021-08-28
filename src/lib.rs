//! DOGS implementation of the Graph Coloring problem

// #![warn(clippy::all, clippy::pedantic)]
// useful additional warnings if docs are missing, or crates imported but unused, etc.
#![warn(missing_debug_implementations)]
#![warn(missing_docs)]
#![warn(trivial_casts, trivial_numeric_casts)]
#![warn(unsafe_code)]
#![warn(unused_extern_crates)]
#![warn(variant_size_differences)]

// not sure if already by default in clippy
#![warn(clippy::similar_names)]
#![warn(clippy::shadow_unrelated)]
#![warn(clippy::shadow_same)]
#![warn(clippy::shadow_reuse)]


/// coloring instance base trait, solutions and checker
pub mod color;

/// read/write DIMACS formats
pub mod dimacs;

/// read/write CGSHOP instances & solutions (specialized for very large coloring instances)
pub mod cgshop;

/// compact instance (use this for small-medium sized instances)
pub mod compact_instance;

/// helper and utility methods for executables
pub mod util;

/// search spaces for the graph coloring problem
pub mod search;