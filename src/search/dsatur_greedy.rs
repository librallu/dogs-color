use std::cmp::max;

use ordered_float::OrderedFloat;
use bit_set::BitSet;

use crate::cgshop::{CGSHOPInstance, CGSHOPSolution};

/** implements a greedy DSATUR algorithm. This algorithm should be able to handle large scale
instances (up to 1M nodes and 1T edges).
    1. choose an uncolored node that sees the most colors (break ties by the largest degree)
    2. add the segment to the first color available
    3. mark all its neighbors seeing this color
    4. repeat until a proper coloring is found
*/