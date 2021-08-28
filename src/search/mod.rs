//! Search spaces for the graph coloring problem.

/// greedy DSATUR algorithm
pub mod greedy_dsatur;

/// Recursive Largest First algorithm (RLF)
pub mod greedy_rlf;

/// greedy that finds a clique of "large" size
pub mod greedy_clique;

// /// backtracking-based dsatur
// pub mod backtracking_dsatur;

// /// copying-based dsatur (experimental) 
// pub mod dsatur;

// /// TABUCOL implementation 
// pub mod tabucol;

// /// PARTIALCOL implementation
// pub mod partialcol;

// /// DSATUR adapted for the large CGSHOP instances
// pub mod cgshop_dsatur;