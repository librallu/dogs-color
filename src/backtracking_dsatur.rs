use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::rc::Rc;
use std::cell::RefCell;

use ordered_float::OrderedFloat;
use bit_set::BitSet;

use dogs::search_space::{SearchSpace, TotalNeighborGeneration, GuidedSpace, ToSolution};
use dogs::tree_search::beam_search::BeamSearch;
use dogs::search_algorithm::{NeverStoppingCriterion, SearchAlgorithm};

use crate::color::{Instance, Solution, VertexId, checker, CheckerResult};


/** Vertex ordering type */
#[derive(Clone,Debug,Eq,PartialEq)]
pub enum OrderingType {
    /** DSATUR: choose first the vertex that has in its neighborhood the most colors.
    breaks ties by the degree. */
    DSATUR
}

/** cost & vertex for the vertex ordering */
#[derive(Debug,Clone,PartialEq,Eq)]
struct VertexOrderingInfo {
    /// ordering type
    pub ordering: OrderingType,
    /// Vertex ID
    pub v: VertexId,
    /// degree of saturation
    pub dsat: usize,
    /// degree
    pub d: usize,
}

impl Ord for VertexOrderingInfo {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.ordering {
            OrderingType::DSATUR => {
                self.dsat.cmp(&other.dsat)
                    .then_with(|| self.d.cmp(&other.d))
            }
        }
    }
}

// `PartialOrd` needs to be implemented as well.
impl PartialOrd for VertexOrderingInfo {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}



/**
Implements a backtracking search space for DSATUR.
 - 
*/
#[derive(Debug)]
pub struct BacktrackingDsaturSpace {
    /// instance
    inst: Rc<Instance>,
    /// ordered vertices (according to the ordering)
    ordered_vertices: BinaryHeap<VertexOrderingInfo>,
    /// colors[i]: color assigned to vertex i
    colors: Vec<Option<usize>>
}

/** represents a node structure */
#[derive(Debug, Clone)]
pub struct Node {
    /// number of colored nodes
    nb_colored: usize,
}

impl BacktrackingDsaturSpace {
    /** creates a new backtracking Dsatur search space */
    pub fn new(inst:Rc<Instance>, ordering:OrderingType) -> Self {
        let n = inst.n();
        let colors = vec![None ; n];
        let mut ordered_vertices = BinaryHeap::with_capacity(n);
        for i in 0..n {
            ordered_vertices.push(VertexOrderingInfo {
                ordering: ordering.clone(),
                v: i,
                dsat: 0,
                d: inst.adj(i).len(),
            });
        }
        Self {
            inst,
            ordered_vertices,
            colors,
        }
    }
}
