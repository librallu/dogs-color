use std::rc::Rc;

use rand::distributions::{Distribution, Uniform};
use bit_set::BitSet;

use dogs::{combinators::helper::tabu_tenure::TabuTenure, search_space::{SearchSpace, TotalNeighborGeneration, GuidedSpace, ToSolution, DecisionSpace}};

use crate::color::{Instance, Solution, VertexId, checker};

/**
Implements a search tree node.
Stores a decision and a number of conflicts
*/
#[derive(Debug,Clone)]
pub struct Node {
    /// decision taken by the node
    decision:Option<Decision>,
    /// number of conflicting edges
    nb_conflicts: usize,
}

/** implements a specific tabu tenure for the graph coloring
Is parametrized by:
 - L: minimum size of the tabu tenure (example value: 5)
 - λ: (example value: 0.6)
keeps all moves that are between L+λ.F(c) where F(c) is the number of conflicts.
Maintains the current iteration number and the last iteration in which
the decision have been taken. When checking if a conflict exists,
checks that its last access is greater than current_iter - L+λ.F(c)
*/
pub struct TabuColTenure {
    /// tabu fixed size
    l:usize,
    /// tabu dynamic size
    lambda: f64,
    /// number of iterations since the beginning of the search
    nb_iter: usize,
    /// decisions[v][c]: last iteration in which the decision have been taken
    decisions: Vec<Vec<Option<usize>>>
}

impl TabuTenure<Node, Decision> for TabuColTenure {
    fn insert(&mut self, _n:&Node, d:Decision) {
        self.decisions[d.v][d.c] = Some(self.nb_iter);
        self.nb_iter += 1;
    }

    fn contains(&self, n:&Node, d:&Decision) -> bool {
        match self.decisions[d.v][d.c] {
            None => false,
            Some(i) => {
                i >= self.nb_iter - (self.l + (self.lambda * (n.nb_conflicts as f64)) as usize)
            }
        }
    }
}

impl TabuColTenure {
    /** creates a tabucol tenure given:
     - l: fixed tabu size
     - λ: variable tabu size
     - n: the number of vertices in the graph
     - c: the maximum number of colors
    */
    pub fn new(l:usize, lambda: f64, n:usize, c:usize) -> Self {
        Self {
            l, lambda,
            nb_iter: 0,
            decisions: vec![vec![None ; c] ; n],
        }
    }
}


/** (see https://www.sciencedirect.com/science/article/pii/S0305054805002315 for more details)
Implements a local search procedure for the graph coloring (TabuCol).
Starts with an initial solution
  - either invalid: the local search aims to make it valid
  - either valid: in this case, the search removes one color (with the least colored vertices)
    and try to make it valid (thus using one less color)
each decision taken (assign a color to a vertex is memorized and cannot be done again)

It makes changes in the coloring to minimize the number of conflicts.

main procedure:
 1. iterate over edges, mark conflicting vertices as "move candidates"
 2. for each end point of a conflicting edge, try to change its color
*/
#[derive(Debug)]
pub struct SearchState {
    /// reference instance
    inst: Rc<Instance>,
    /// colors[v]: color of the vertex v
    colors: Vec<usize>,
    /// history of decisions to revert changes (should be used as a stack)
    revert_decisions: Vec<Decision>,
    /// number of colors used
    nb_colors: usize,
}

/**
Decision of changing the color of vertex v by c
*/
#[derive(Debug,Clone,Hash,Eq,PartialEq)]
pub struct Decision {
    pub v: VertexId,
    pub c: usize,
}

impl Default for Decision {
    fn default() -> Self {
        unimplemented!()
    }
}

impl SearchState {

    /**
    Creates a new search state with a random solution using nb_colors
    */
    pub fn random_solution(inst:Rc<Instance>, nb_colors:usize) -> Self {
        let mut rng = rand::thread_rng();
        let uniform_distribution = Uniform::from(0..nb_colors+1);
        let mut colors:Vec<usize> = Vec::with_capacity(inst.n());
        for _ in 0..inst.n() {
            colors.push(uniform_distribution.sample(&mut rng));
        }
        Self {
            inst,
            colors,
            revert_decisions: Vec::new(),
            nb_colors,
        }
    }

    /**
    Creates a new search state from an existing solution.
    Removes the color with the less vertices and replace it by random other colors
    */
    pub fn from_solution(inst:&Instance, sol:Solution) -> Self {
        todo!()
    }

    /** applies a decision to the search state */
    pub fn commit(&mut self, decision:&Decision) {
        // create a restore decision and add it to the revert_decisions
        self.revert_decisions.push(Decision { v: decision.v, c: self.colors[decision.v] });
        self.apply_decision(decision);
    }

    /** restores the state before a decision to the search state */
    pub fn restore(&mut self) {
        let d = self.revert_decisions.pop()
            .expect("SearchState.restore: no restore decision. Unable to revert (internal error)");
        // apply revert decision
        self.apply_decision(&d);
    }

    /** just applies the decision (either called from restore or commit) */
    fn apply_decision(&mut self, decision:&Decision) {
        self.colors[decision.v] = decision.c;
    }

    /** returns the list of conflicting edges */
    fn conflicting_edges(&self) -> Vec<(VertexId,VertexId)> {
        self.inst.edges().iter()
            .filter(|(u,v)| self.colors[*u] == self.colors[*v])
            .cloned().collect()
    }

}


impl GuidedSpace<Node, usize> for SearchState {
    fn guide(&mut self, node: &Node) -> usize {
        node.nb_conflicts
    }
}

impl ToSolution<Node, Solution> for SearchState {
    fn solution(&mut self, node: &mut Node) -> Solution {
        assert_eq!(node.nb_conflicts, 0); // check if valid
        let mut sol:Solution = vec![vec![]; self.nb_colors];
        for (i,v) in self.colors.iter().enumerate() {
            sol[*v].push(i);
        }
        let res:Solution = sol.iter().filter(|e| !e.is_empty())
            .cloned().collect();
        assert_eq!(checker(&self.inst, &res), Some(res.len()));
        res
    }
}

impl SearchSpace<Node, i32> for SearchState {
    fn initial(&mut self) -> Node {
        Node { decision: None, nb_conflicts: self.conflicting_edges().len() }
    }

    fn bound(&mut self, _node: &Node) -> i32 { 0 }

    fn goal(&mut self, node: &Node) -> bool {
        node.nb_conflicts == 0
    }

    fn g_cost(&mut self, _n: &Node) -> i32 { 0 }
}


impl DecisionSpace<Node, Decision> for SearchState {
    fn decision(&self, n:&Node) -> Option<Decision> { n.decision.clone() }

    fn aspiration_criterion(&self, n:&Node) -> bool { n.nb_conflicts == 0 }
}


impl TotalNeighborGeneration<Node> for SearchState {
    fn neighbors(&mut self, node: &mut Node) -> Vec<Node> {
        println!("{}", node.nb_conflicts);
        // apply decision within the node
        match &node.decision {
            None => {},
            Some(d) => self.commit(d)
        };
        // for each conflicting edge, mark endpoints as to visit
        let mut vertices_to_change = Vec::new();
        let mut vertices_to_change_bitset:BitSet<u64> = BitSet::default();
        let conflicting_edges = self.conflicting_edges();
        let nb_conflicts = conflicting_edges.len();
        for (u,v) in conflicting_edges {
            if !vertices_to_change_bitset.contains(u) {
                vertices_to_change_bitset.insert(u);
                vertices_to_change.push(u);
            }
            if !vertices_to_change_bitset.contains(v) {
                vertices_to_change_bitset.insert(v);
                vertices_to_change.push(v);
            }
        }
        // for each vertex to try changing and other color (all but the original one)
        let mut res = Vec::new();
        for v in vertices_to_change {
            for c in 0..self.nb_colors {
                if self.colors[v] != c {
                    let mut new_nb_conflicts = nb_conflicts;
                    // update number of conflicts
                    for u in self.inst.adj(v) {
                        if self.colors[*u] == c {  // if new conflict
                            new_nb_conflicts += 1;
                        }
                        if self.colors[*u] == self.colors[v] {  //  if remove conflict
                            new_nb_conflicts -= 1;
                        }
                    }
                    res.push(Node { decision: Some(Decision {v, c}), nb_conflicts: new_nb_conflicts })
                }
            }
        }
        res
    }
}


#[cfg(test)]
mod tests {

    use super::*;

    use std::cell::RefCell;

    use dogs::metric_logger::MetricLogger;
    use dogs::search_algorithm::{SearchAlgorithm, NeverStoppingCriterion};
    use dogs::combinators::stats::StatTsCombinator;
    use dogs::combinators::tabu::TabuCombinator;
    use dogs::combinators::helper::tabu_tenure::FullTabuTenure;
    use dogs::tree_search::greedy::Greedy;

    #[test]
    fn test_root_node() {
        let inst = Rc::new(Instance::from_file("insts/instances-dimacs1/le450_15a.col"));
        let mut search_state = SearchState::random_solution(inst, 20);
        let mut initial_node = search_state.initial();
        println!("{:?}", initial_node);
        let mut neighbors = search_state.neighbors(&mut initial_node);
        neighbors.sort_by_key(|e| e.nb_conflicts);
        println!("{:?}", neighbors[0]);
    }

    #[test]
    fn test_simple_descent() {
        let inst = Rc::new(Instance::from_file("insts/instances-dimacs1/le450_15a.col"));
        let mut search_state = SearchState::random_solution(inst, 17);
        let mut current_node = search_state.initial();
        println!("{:?}", current_node);
        let mut neighbors:Vec<Node> = search_state.neighbors(&mut current_node).iter()
            .filter(|e| e.nb_conflicts < current_node.nb_conflicts)
            .cloned().collect();
        while !neighbors.is_empty() {
            current_node = neighbors.iter().min_by_key(|e| e.nb_conflicts).unwrap().clone();
            println!("{:?}", current_node);
            search_state.apply_decision(&current_node.decision.clone().unwrap());
            neighbors = search_state.neighbors(&mut current_node).iter()
                .filter(|e| e.nb_conflicts < current_node.nb_conflicts)
                .cloned().collect();
        }
    }

    #[test]
    fn test_greedy() {
        let inst = Rc::new(Instance::from_file("insts/instances-dimacs1/le450_15b.col"));
        let nb_initial_colors:usize = 16;
        let logger = Rc::new(MetricLogger::default());
        let search_state = Rc::new(RefCell::new(
            StatTsCombinator::new(
                TabuCombinator::new(
                    SearchState::random_solution(inst.clone(), nb_initial_colors),
                    // FullTabuTenure::default()
                    TabuColTenure::new(50, 0.6, inst.n(), nb_initial_colors)
                )
                
            ).bind_logger(Rc::downgrade(&logger))
        ));
        let stopping_criterion = NeverStoppingCriterion::default();
        let mut ts = Greedy::new(search_state.clone());
        ts.run(stopping_criterion);
        search_state.borrow_mut().display_statistics();
    }
}