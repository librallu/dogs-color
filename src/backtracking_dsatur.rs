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
    /// cost defined as (degree saturation, degree)
    pub cost: (usize, usize)
}

impl Ord for VertexOrderingInfo {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.ordering {
            OrderingType::DSATUR => {
                self.cost.0.cmp(&other.cost.0)
                    .then_with(|| self.cost.1.cmp(&other.cost.1))
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
    pub fn new(inst:Rc<Instance>) -> Self {
        let mut ranked_vertices:Vec<VertexId> = (0..inst.n()).collect();
        ranked_vertices.sort_by_key(|v| -(inst.adj(*v).len() as i64));
        let mut vertex_ranks = vec![0;inst.n()];
        for (i,v) in ranked_vertices.iter().enumerate() {
            vertex_ranks[*v] = i;
        }
        Self {
            inst,
            ordered_vertices,
            colors,
        }
    }
}
