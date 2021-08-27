use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::rc::Rc;
use std::cell::RefCell;

use ordered_float::OrderedFloat;
use bit_set::BitSet;

use dogs::search_space::{SearchSpace, TotalNeighborGeneration, GuidedSpace, ToSolution};
use dogs::tree_search::beam_search::BeamSearch;
use dogs::search_algorithm::{NeverStoppingCriterion, SearchAlgorithm};

use crate::color::{CompactInstance, Solution, VertexId, checker, CheckerResult};


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
    inst: Rc<CompactInstance>,
    /// ordered vertices (according to the ordering)
    ordered_vertices: BinaryHeap<VertexOrderingInfo>,
    /// colors[i]: color assigned to vertex i
    colors: Vec<Option<usize>>,
    /// number of colors in the search state
    nb_colors: usize,
}

/** represents a node structure */
#[derive(Debug, Clone)]
pub struct Node {
    /// number of colored nodes
    nb_colored: usize,
}

impl BacktrackingDsaturSpace {
    /** creates a new backtracking Dsatur search space */
    pub fn new(inst:Rc<CompactInstance>, ordering:OrderingType) -> Self {
        let n = inst.n();
        let mut colors = vec![None ; n];
        let mut ordered_vertices = BinaryHeap::with_capacity(n);
        for i in 0..n {
            ordered_vertices.push(VertexOrderingInfo {
                ordering: ordering.clone(),
                v: i,
                dsat: 0,
                d: inst.adj(i).len(),
            });
        }
        // add the first vertex in the order to color 1
        let first_vertex = ordered_vertices.pop().unwrap().v;
        colors[first_vertex] = Some(0);
        // build the search space
        Self {
            inst,
            ordered_vertices,
            colors,
            nb_colors: 0,
        }
    }
}


impl GuidedSpace<Node, OrderedFloat<f64>> for BacktrackingDsaturSpace {
    fn guide(&mut self, node: &Node) -> OrderedFloat<f64> {
        OrderedFloat(0.) // no guidance strategy needed yet
    }
}

impl ToSolution<Node, Solution> for BacktrackingDsaturSpace {
    fn solution(&mut self, node: &mut Node) -> Solution {
        debug_assert!(self.goal(node));
        // build the solution (res[i]: vertices assigned color i)
        let mut res = vec![vec![]; self.nb_colors];
        for (i,color) in self.colors.iter().enumerate() {
            res[color.unwrap()].push(i);
        }
        res
    }
}

impl SearchSpace<Node, i64> for BacktrackingDsaturSpace {

    fn initial(&mut self) -> Node {
        Node {
            nb_colored: 1,
        }
    }

    fn g_cost(&mut self, node: &Node) -> i64 { self.colors.len() as i64 }

    fn bound(&mut self, node: &Node) -> i64 { self.colors.len() as i64 }

    fn goal(&mut self, node: &Node) -> bool { node.nb_colored == self.inst.n() }

    fn handle_new_best(&mut self, mut node: Node) -> Node {
        // checks that the solution is valid (call checker)
        let sol = self.solution(&mut node);
        let checker_result = checker(&self.inst, &sol);
        match &checker(&self.inst, &sol) {
            CheckerResult::Ok(v) => assert_eq!(*v, self.colors.len()),
            _ => panic!("invalid solution (error: {:?}).", checker_result)
        }
        node
    }
}