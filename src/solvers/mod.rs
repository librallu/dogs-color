//! Search spaces for the graph coloring problem.

/// CGSHOP competition specific solvers
pub mod cgshop;

/// Maximum clique (and Maximum stable) solvers
pub mod clique;

/// Vertex Coloring problem solvers
pub mod coloring;



// /// greedy DSATUR algorithm
// pub mod greedy_dsatur;

// /// Recursive Largest First algorithm (RLF)
// pub mod greedy_rlf;



// /// TABUCOL implementation for the vertex coloring problem
// pub mod tabucol;

// /// conflict weighting local search for the vertex coloring problem
// pub mod coloring_conflict_weighting;

// /// partial weighting local search for the vertex coloring problem
// pub mod coloring_partial_weighting;

// /// backtracking DSATUR for the vertex coloring problem
// pub mod coloring_dsatur_backtrack;

// /// Admissible Orientation Greedy algorithm for the CGSHOP competition
// pub mod cgshop_aog;

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