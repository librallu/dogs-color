//! Search spaces for the graph coloring problem.

/// utility solvers
pub mod util;

/// greedy DSATUR algorithm
pub mod greedy_dsatur;

/// Recursive Largest First algorithm (RLF)
pub mod greedy_rlf;

/// greedy that finds a clique of "large" size
pub mod greedy_clique;

/// TABUCOL implementation for the vertex coloring problem
pub mod tabucol;

/// conflict weighting local search for the vertex coloring problem
pub mod coloring_conflict_weighting;

/// partial weighting local search for the vertex coloring problem
pub mod coloring_partial_weighting;

/// backtracking DSATUR for the vertex coloring problem
pub mod coloring_dsatur_backtrack;

/// conflict weighting local search for the clique problem
pub mod clique_conflict_weighting;

/// partial weighting local search for the clique problem
pub mod clique_partial_weighting;

/// Ejection chains. Removes the smallest color, and try to insert it to the minimum conflicting color.
pub mod ejection_chains;

/// branch & bound for the CLIQUE problem
pub mod clique_bnb;

/// swap moves for the CLIQUE problem
pub mod clique_swap;

/// Admissible Orientation Greedy algorithm for the CGSHOP competition
pub mod cgshop_aog;

// /// Stable generation based algorithm for the CGSHOP competition
// pub mod cgshop_stable_generation;

// /// backtracking-based dsatur
// pub mod backtracking_dsatur;

// /// copying-based dsatur (experimental) 
// pub mod dsatur;

// /// PARTIALCOL implementation
// pub mod partialcol;

// /// DSATUR adapted for the large CGSHOP instances
// pub mod cgshop_dsatur;