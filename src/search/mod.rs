//! Search spaces for the graph coloring problem.

/// greedy DSATUR algorithm
pub mod greedy_dsatur;

/// Recursive Largest First algorithm (RLF)
pub mod greedy_rlf;

/// greedy that finds a clique of "large" size
pub mod greedy_clique;

/// TABUCOL implementation 
pub mod tabucol;

/// Admissible Orientation Greedy algorithm for the CGSHOP competition
pub mod cgshop_aog;

/// Stable generation based algorithm for the CGSHOP competition
pub mod cgshop_stable_generation;

/// Ejection chains. Removes the smallest color, and try to insert it to the minimum conflicting color.
pub mod ejection_chains;

/// utility solvers
pub mod util;

// /// backtracking-based dsatur
// pub mod backtracking_dsatur;

// /// copying-based dsatur (experimental) 
// pub mod dsatur;

// /// PARTIALCOL implementation
// pub mod partialcol;

// /// DSATUR adapted for the large CGSHOP instances
// pub mod cgshop_dsatur;