use std::rc::Rc;

use ordered_float::OrderedFloat;
use bit_set::BitSet;

use dogs::search_space::{GuidedSpace, Identifiable, SearchSpace, ToSolution, TotalNeighborGeneration};

use crate::color::{ColoringInstance, VertexId};


/**
Implements a CLIQUE search space.
root: no vertex is added
decisions: add a vertex to the CLIQUE. mark its non-neighbors as non-candidates
*/
#[derive(Debug)]
pub struct CLIQUESpace {
    /// instance for the space
    inst: Rc<dyn ColoringInstance>,
}


/** represents a node (stores the vertices in the CLIQUE and f-bound) */
#[derive(Debug, Clone)]
pub struct Node {
    /// vertices in the CLIQUE
    clique:Vec<usize>,
    /// candidate nodes
    candidates: BitSet,
    /// degree between candidate nodes
    candidate_degrees: i64,
    /// nb candidates
    nb_candidates: i64,
}

impl CLIQUESpace {

    /** CLIQUE space constructor */
    pub fn new(inst:Rc<dyn ColoringInstance>) -> Self {
        Self { inst }
    }

    /** adds a vertex v to the current clique, returns a new node */
    pub fn add_vertex(&self, n:&Node, v:VertexId) -> Node {
        assert!(n.candidates.contains(v), "trying to insert a node not in the candidate list");
        assert!(n.candidate_degrees >= 0, "candidate degrees should always be >= 0 (current:{}", n.candidate_degrees);
        assert!(n.nb_candidates >= 0, "nb_candidates should always be >= 0");
        let mut res = n.clone();
        res.clique.push(v);
        res.candidates.remove(v);
        res.nb_candidates -= 1;
        res.candidate_degrees -= self.inst.degree(v) as i64;
        // remove non-neighbors of v
        for u in n.candidates.iter() {
            if u!=v && !self.inst.are_adjacent(u, v) {
                res.candidates.remove(u);
                res.nb_candidates -= 1;
                res.candidate_degrees -= self.inst.degree(u) as i64;
            }
        }
        res
    }
}

impl GuidedSpace<Node, OrderedFloat<f64>> for CLIQUESpace {
    fn guide(&mut self, node: &Node) -> OrderedFloat<f64> {
        let m = (node.candidate_degrees as f64/2.).floor(); // ∑ d(v) = 2m <=> m = (∑ d(v))/2
        let h = -((1. + (1.+8.*m).sqrt())/2.).floor() as i64; 
        OrderedFloat((self.g_cost(node)+h) as f64)
    }
}


impl ToSolution<Node, Vec<VertexId>> for CLIQUESpace {
    fn solution(&mut self, node: &mut Node) -> Vec<VertexId> {
        debug_assert!(self.goal(node));
        node.clique.clone()
    }
}

impl Identifiable<Node, BitSet> for CLIQUESpace {
    fn id(&self, n: &mut Node) -> BitSet {
        n.candidates.clone()
    }
}


impl SearchSpace<Node, i64> for CLIQUESpace {

    fn initial(&mut self) -> Node {
        let mut candidates:BitSet = BitSet::new();
        let mut candidate_degrees:i64 = 0;
        for i in 0..self.inst.nb_vertices() {
            candidates.insert(i);
            candidate_degrees += self.inst.degree(i) as i64;
        }
        Node {
            clique: Vec::new(),
            candidates,
            candidate_degrees,
            nb_candidates: self.inst.nb_vertices() as i64
        }
    }

    fn g_cost(&mut self, node: &Node) -> i64 { -(node.clique.len() as i64) }

    fn bound(&mut self, node: &Node) -> i64 {
        let g  = self.g_cost(node);
        let m = (node.candidate_degrees as f64/2.).floor(); // ∑ d(v) = 2m <=> m = (∑ d(v))/2
        let h = -((1. + (1.+8.*m).sqrt())/2.).floor() as i64; 
        // let h = -node.nb_candidates;
        // let m = (node.candidate_degrees as f64/2.).floor();
        // let h = -m.sqrt() as i64;
        // let h = 0;
        g+h
    }

    fn goal(&mut self, node: &Node) -> bool {
        node.nb_candidates == 0
    }

    fn handle_new_best(&mut self, mut _node: Node) -> Node {
        // println!("clique: {}", _node.clique.len());
        // TODO checker
        _node
    }
}

impl TotalNeighborGeneration<Node> for CLIQUESpace {
    fn neighbors(&mut self, node: &mut Node) -> Vec<Node> {
        // println!("{:?}", node.clique.len());
        for u in node.clique.iter() {
            for v in node.clique.iter() {
                if u != v && !self.inst.are_adjacent(*u, *v) {
                    panic!("invalid clique! {} not adj to {}", u, v);
                }
            }
        }
        node.candidates.iter().map(|v| self.add_vertex(node, v)).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    use crate::cgshop::CGSHOPInstance;
    use dogs::combinators::bounding::BoundingCombinator;
    use dogs::combinators::gcost_dominance::GcostDominanceTsCombinator;
    use dogs::combinators::helper::discrepancy::{ConstantDiscrepancy, LinearDiscrepancy, RatioToBestDiscrepancy};
    use dogs::combinators::lds::LDSCombinator;
    use dogs::combinators::pruning::PruningCombinator;
    use dogs::metric_logger::MetricLogger;
    use dogs::{combinators::stats::StatTsCombinator};
    use dogs::tree_search::depth_first::DepthFirstSearch;
    use std::cell::RefCell;
    use dogs::search_algorithm::{NeverStoppingCriterion, SearchAlgorithm, TimeStoppingCriterion};
    
    #[test]
    pub fn test_ibs() {
        let logger = Rc::new(MetricLogger::default());
        let inst = Rc::new(CGSHOPInstance::from_file(
            // "./insts/CGSHOP_22_original/cgshop_2022_examples_01/example-instances-sqrm/sqrm_5K_1.instance.json",
            "./insts/CGSHOP_22_original/cgshop_2022_examples_01/example-instances-sqrm/sqrm_10K_1.instance.json",
            // "./insts/CGSHOP_22_original/cgshop_2022_examples_01/example_instances_visp/visp_5K.instance.json",
            // "./insts/CGSHOP_22_original/cgshop_2022_examples_01/example_instances_visp/visp_10K.instance.json",
            // "./insts/cgshop_22_examples/tiny10.instance.json",
            true
        ));
        let space = Rc::new(RefCell::new(
            LDSCombinator::new(
                StatTsCombinator::new(
                    BoundingCombinator::new(
                        PruningCombinator::new(
                            GcostDominanceTsCombinator::new(
                                CLIQUESpace::new(inst)
                            )
                        )
                    ).bind_logger(Rc::downgrade(&logger))
                ).bind_logger(Rc::downgrade(&logger))
            , 5.0, LinearDiscrepancy{})
        ));
        logger.display_headers();
        // let mut search = create_iterative_beam_search(space, 1.0, 2.0);
        let mut search = DepthFirstSearch::new(space.clone());
        // search.run(NeverStoppingCriterion::default());
        search.run(TimeStoppingCriterion::new(60.));
        space.borrow_mut().display_statistics();
    } 

}