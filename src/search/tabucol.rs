use std::rc::Rc;
use std::cell::RefCell;

use dogs::combinators::stats::StatTsCombinator;
use dogs::metric_logger::MetricLogger;
use fastrand::Rng;
use bit_set::BitSet;

use dogs::search_algorithm::{SearchAlgorithm, StoppingCriterion};
use dogs::combinators::helper::tabu_tenure::TabuTenure;
use dogs::search_space::{
    SearchSpace, TotalNeighborGeneration, GuidedSpace, ToSolution, DecisionSpace
};
use dogs::combinators::tabu::TabuCombinator;
use dogs::tree_search::greedy::Greedy;

use crate::color::{ColoringInstance, Solution, VertexId, checker, CheckerResult};


/**
Decision of changing the color of vertex v by c
*/
#[derive(Debug,Clone,Hash,Eq,PartialEq)]
pub struct Decision {
    /// vertex to color
    pub v: VertexId,
    /// previously used color for v
    pub c_prev: usize,
    /// color to use
    pub c_next: usize,
}

impl Default for Decision {
    fn default() -> Self {
        unimplemented!()
    }
}


/**
Implements a search tree node.
Stores a decision and a number of conflicts
*/
#[derive(Debug,Clone)]
pub struct Node {
    /// decision taken by the node
    decision:Option<Decision>,
    /// number of conflicting edges
    nb_conflicts: i64,
}

/** implements a specific tabu tenure for the graph coloring
Is parametrized by:
 - L: minimum size of the tabu tenure (example value: 10). We use a random number between 0 and L.
 - λ: (example value: 0.6)
keeps all moves that are between L+λ.F(c) where F(c) is the number of conflicts.
Maintains the current iteration number and the last iteration in which
the decision have been taken. When checking if a conflict exists,
checks that its last access is greater than current_iter - L+λ.F(c)
*/
#[derive(Debug)]
pub struct TabuColTenure {
    /// tabu fixed size
    l:usize,
    /// tabu dynamic size
    lambda: f64,
    /// number of iterations since the beginning of the search
    nb_iter: i64,
    /// decisions[v][c]: last iteration in which the decision have been taken
    decisions: Vec<Vec<Option<i64>>>,
    /// random number generator
    rng: Rng,
}

impl TabuTenure<Node, Decision> for TabuColTenure {
    fn insert(&mut self, _n:&Node, d:Decision) {
        self.decisions[d.v][d.c_prev] = Some(self.nb_iter);
        self.nb_iter += 1;
    }

    fn contains(&mut self, n:&Node, d:&Decision) -> bool {
        match self.decisions[d.v][d.c_next] {
            None => false,
            Some(i) => {
                let rand_l:i64 = self.rng.i64(0..self.l as i64);
                let threshold = rand_l + (self.lambda * (n.nb_conflicts as f64)) as i64;
                i >= self.nb_iter - threshold
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
            rng: Rng::new(),
        }
    }
}


/** (see https://doi.org/10.1016/j.cor.2005.07.028)
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
    inst: Rc<dyn ColoringInstance>,
    /// colors[v]: color of the vertex v
    colors: Vec<usize>,
    /// number of colors used
    nb_colors: usize,
    /// nb_neigh_colors[v][c]: number of neighbors of v that are assigned color c
    nb_neigh_colors: Vec<Vec<usize>>,
    /// conflicting_edges[i].contains(j) -> the edges (i,j) are conflicting 
    conflicting_edges: Vec<BitSet>,
    /// number of conflicting edges
    nb_conflicting_edges: i64,
    /// last valid solution seen
    last_solution: Vec<Vec<VertexId>>,
    /// random number generator
    rng:Rng,
}


impl SearchState {

    /** removes a color from the current solution (should be feasible). */
    fn remove_color(&mut self) {
        let n = self.inst.nb_vertices();
        let sol = self.last_solution.clone();
        let (mini_color_index,_) = sol.iter().enumerate().min_by_key(|(_,e)| e.len()).unwrap();
        let mut new_sol:Vec<Vec<VertexId>> = sol.to_vec();
        let removed_vertices = new_sol.remove(mini_color_index);
        let nb_colors = new_sol.len();
        // populate colors
        let mut colors = vec![0 ; n];
        for (c,vertices) in new_sol.iter().enumerate() {
            for v in vertices {
                colors[*v] = c
            }
        }
        // removed vertices are colored
        for v in removed_vertices.iter() {
            colors[*v] = self.rng.usize(0..nb_colors);
        }
        // compute nb neigh colors
        let mut nb_neigh_colors = vec![ vec![0 ; nb_colors] ; self.inst.nb_vertices()];
        for i in self.inst.vertices() {
            for j in self.inst.neighbors(i) {
                nb_neigh_colors[j][colors[i]] += 1;
            }
        }
        // compute conflicting edges
        let mut conflicting_edges = vec![BitSet::with_capacity(n) ; n];
        let mut nb_conflicting_edges = 0;
        for u in removed_vertices.iter() {
            for v in self.inst.neighbors(*u) {
                if colors[*u] == colors[v] {
                    nb_conflicting_edges += 1;
                    conflicting_edges[v].insert(*u);
                    conflicting_edges[*u].insert(v);
                }
            }
        }
        // define attributes
        self.colors = colors;
        self.nb_colors = nb_colors;
        self.nb_neigh_colors = nb_neigh_colors;
        self.conflicting_edges = conflicting_edges;
        self.nb_conflicting_edges = nb_conflicting_edges;
    }

    /** Creates a new search state, starting by a feasible solution, removing the color
    with the less vertices.
    */
    pub fn from_solution(inst:Rc<dyn ColoringInstance>, sol:&[Vec<VertexId>]) -> Self {
        assert_eq!(checker(inst.clone(), sol), CheckerResult::Ok(sol.len()));
        let mut res = Self {
            inst,
            colors: Vec::new(),
            nb_colors: sol.len()-1,
            nb_neigh_colors: Vec::new(),
            conflicting_edges: Vec::new(),
            nb_conflicting_edges: 0,
            last_solution: sol.to_vec(),
            rng: Rng::new(),
        };
        res.remove_color();
        res
    }

    /** applies a decision to the search state */
    pub fn commit(&mut self, decision:&Decision) {
        self.apply_decision(decision);
    }

    /** just applies the decision (either called from restore or commit) */
    fn apply_decision(&mut self, decision:&Decision) {
        assert!(decision.c_next < self.nb_colors);
        // update nb_neigh_color
        let previous_color = self.colors[decision.v];
        for neigh in self.inst.neighbors(decision.v) {
            debug_assert!(self.nb_neigh_colors[neigh][previous_color] > 0);
            self.nb_neigh_colors[neigh][previous_color] -= 1;
            self.nb_neigh_colors[neigh][decision.c_next] += 1;
        }
        // update colors
        self.colors[decision.v] = decision.c_next;
        // update conflicting edges
        for u in self.inst.neighbors(decision.v) {
            if self.colors[u] == previous_color { // remove conflict
                self.conflicting_edges[u].remove(decision.v);
                self.conflicting_edges[decision.v].remove(u);
                self.nb_conflicting_edges -= 1;
            }
            if self.colors[u] == decision.c_next { // add conflict
                self.conflicting_edges[u].insert(decision.v);
                self.conflicting_edges[decision.v].insert(u);
                self.nb_conflicting_edges += 1;
            }
        }
    }

    fn build_solution(&self) -> Solution {
        let mut sol:Solution = vec![vec![]; self.nb_colors];
        for (i,v) in self.colors.iter().enumerate() {
            sol[*v].push(i);
        }
        let res:Solution = sol.iter().filter(|e| !e.is_empty())
            .cloned().collect();
        assert_eq!(checker(self.inst.clone(), &res), CheckerResult::Ok(res.len()));
        res
    }

    fn nb_conflicting_edges(&self) -> i64 {
        // let correct:i64 = self.conflicting_edges.iter().map(|e| e.len() as i64).sum::<i64>() / 2;
        // // correct*2
        // // println!("correct: {}\t current: {}", correct, self.nb_conflicting_edges);
        // assert_eq!(correct, self.nb_conflicting_edges);
        self.nb_conflicting_edges
    }
}


impl GuidedSpace<Node, i64> for SearchState {
    fn guide(&mut self, node: &Node) -> i64 {
        node.nb_conflicts
    }
}

impl ToSolution<Node, Solution> for SearchState {
    fn solution(&mut self, _: &mut Node) -> Solution {
        let res = self.last_solution.clone();
        assert_eq!(checker(self.inst.clone(), &res), CheckerResult::Ok(res.len()));
        res
    }
}

impl SearchSpace<Node, i32> for SearchState {
    fn initial(&mut self) -> Node {
        Node {
            decision: None,
            nb_conflicts: self.nb_conflicting_edges()
        }
    }
    fn bound(&mut self, _node: &Node) -> i32 { self.nb_colors as i32 }
    fn goal(&mut self, n: &Node) -> bool { n.nb_conflicts == 0 }
    fn g_cost(&mut self, _n: &Node) -> i32 { 0 }
}


impl DecisionSpace<Node, Decision> for SearchState {
    fn decision(&self, n:&Node) -> Option<Decision> { n.decision.clone() }
    fn aspiration_criterion(&self, n:&Node) -> bool { n.nb_conflicts == 0 }
}


impl TotalNeighborGeneration<Node> for SearchState {
    fn neighbors(&mut self, node: &mut Node) -> Vec<Node> {
        // apply decision within the node
        match &node.decision {
            None => {
                if node.nb_conflicts == 0 {
                    self.last_solution = self.build_solution();
                    // println!("removing one color (nb_conflicts:{})", node.nb_conflicts);
                    self.remove_color();
                }
            },
            Some(d) => self.commit(d)
        };
        // for each conflicting edge, mark endpoints as to visit
        let mut vertices_to_change:BitSet<u64> = BitSet::default();
        let nb_conflicts = self.nb_conflicting_edges();
        if nb_conflicts == 0 {
            return vec![Node { decision:None, nb_conflicts:0 }];
        }
        // println!("{:?}", nb_conflicts);
        for u in self.inst.vertices() {
            for v in self.conflicting_edges[u].iter() {
                if u < v { // for each conflicting edge (u,v), allows changing u and v
                    if !vertices_to_change.contains(u) {
                        vertices_to_change.insert(u);
                    }
                    if !vertices_to_change.contains(v) {
                        vertices_to_change.insert(v);
                    }
                }
            }
        }
        // for each vertex to try changing and other color (all but the original one)
        let mut res = Vec::new();
        for v in vertices_to_change.iter() {
            for c in 0..self.nb_colors {
                if self.colors[v] != c {
                    let new_nb_conflicts:i64 = nb_conflicts + 
                        self.nb_neigh_colors[v][c] as i64 - self.nb_neigh_colors[v][self.colors[v]] as i64;
                    res.push(Node {
                        decision: Some(Decision {v, c_prev: self.colors[v], c_next: c}),
                        nb_conflicts: new_nb_conflicts 
                    })
                }
            }
        }
        res
    }
}


/**
Runs a tabucol algorithm. Given an instance and an initial number of colors, run the search algorithm until the stopping criterion is reached.
Optionnaly, a filename is given to export the solution
*/
pub fn tabucol_with_solution<Stopping:StoppingCriterion>(inst:Rc<dyn ColoringInstance>, sol:&[Vec<VertexId>], stopping_criterion:Stopping, solution_filename:Option<String>) -> Vec<Vec<VertexId>> {
    let mut solution:Vec<Vec<VertexId>> = sol.to_vec();
    let nb_colors = solution.len();
    let logger = Rc::new(MetricLogger::default());
    let space = Rc::new(RefCell::new(
        StatTsCombinator::new(
            TabuCombinator::new(
                SearchState::from_solution(inst.clone(), &solution),
            TabuColTenure::new(inst.nb_vertices()/5, 0.5, inst.nb_vertices(), nb_colors)
            // FullTabuTenure::default()
            )
        ).bind_logger(Rc::downgrade(&logger)),
    ));
    let mut ts = Greedy::new(space.clone());
    logger.display_headers();
    ts.run(stopping_criterion);
    // display the results afterwards
    space.borrow_mut().display_statistics();
    // check that the last solution is valid
    match ts.get_manager().best() {
        None => {
            println!("\ttabu search failed improving...");
        }
        Some(node) => {
            if node.nb_conflicts == 0 {
                let mut node_clone = node.clone();
                solution = space.borrow_mut().solution(&mut node_clone);
                // print output file if asked
                match &solution_filename {
                    None => {},
                    Some(filename) => {
                        inst.write_solution(filename, &solution);
                    }
                }
            }
        }
    }
    solution
}




#[cfg(test)]
mod tests {
    use super::*;
    
    use crate::cgshop::CGSHOPInstance;
    use crate::dimacs::DimacsInstance;
    use crate::search::greedy_dsatur::greedy_dsatur;

    use dogs::search_algorithm::TimeStoppingCriterion;

    #[test]
    fn test_tabu_with_solution_le450_15a() {
        let inst = Rc::new(DimacsInstance::from_file("insts/instances-dimacs1/le450_15a.col"));
        let greedy_sol = greedy_dsatur(inst.clone(), false);
        println!("initial solution: {}", greedy_sol.len());
        let stopping_criterion:TimeStoppingCriterion = TimeStoppingCriterion::new(50.);
        tabucol_with_solution(inst, &greedy_sol, stopping_criterion, None);
    }

    #[test]
    fn test_tabu_with_solution_flat1000() {
        let inst = Rc::new(DimacsInstance::from_file("insts/instances-dimacs1/flat1000_76_0.col"));
        let greedy_sol = greedy_dsatur(inst.clone(), false);
        println!("initial solution: {}", greedy_sol.len());
        let stopping_criterion:TimeStoppingCriterion = TimeStoppingCriterion::new(10.);
        tabucol_with_solution(inst, &greedy_sol, stopping_criterion, None);
    }

    #[test]
    fn test_tabu_with_greedy_cgshop() {
        let inst = Rc::new(CGSHOPInstance::from_file(
            "./insts/cgshop22/reecn3382.instance.json"
        ));
        let greedy_sol = greedy_dsatur(inst.clone(), false);
        println!("initial solution: {}", greedy_sol.len());
        let stopping_criterion:TimeStoppingCriterion = TimeStoppingCriterion::new(3000.);
        tabucol_with_solution(inst, &greedy_sol, stopping_criterion, None);
    }

    #[test]
    fn test_tabu_with_greedy_cgshop_bis() {
        let inst = Rc::new(CGSHOPInstance::from_file(
            "./insts/cgshop22/reecn3382.instance.json"
        ));
        let greedy_sol = greedy_dsatur(inst.clone(), false);
        println!("initial solution: {}", greedy_sol.len());
        let stopping_criterion:TimeStoppingCriterion = TimeStoppingCriterion::new(3000.);
        tabucol_with_solution(inst, &greedy_sol, stopping_criterion, None);
    }

    #[test]
    fn test_tabu_with_greedy_cgshop2() {
        let inst = Rc::new(CGSHOPInstance::from_file(
            "./insts/cgshop22/vispecn2518.instance.json"
        ));
        let greedy_sol = greedy_dsatur(inst.clone(), false);
        println!("initial solution: {}", greedy_sol.len());
        let stopping_criterion:TimeStoppingCriterion = TimeStoppingCriterion::new(3000.);
        tabucol_with_solution(inst, &greedy_sol, stopping_criterion, None);
    }

    #[test]
    fn test_tabu_with_greedy_cgshop3() {
        let inst = Rc::new(CGSHOPInstance::from_file(
            "./insts/cgshop_22_examples/sqrm_10K_1.instance.json"
        ));
        let greedy_sol = greedy_dsatur(inst.clone(), false);
        println!("initial solution: {}", greedy_sol.len());
        let stopping_criterion:TimeStoppingCriterion = TimeStoppingCriterion::new(3000.);
        tabucol_with_solution(inst, &greedy_sol, stopping_criterion, None);
    }

    #[test]
    fn test_tabu_with_greedy_cgshop4() {
        let inst = Rc::new(CGSHOPInstance::from_file(
            "./insts/cgshop_22_examples/sqrm_10K_6.instance.json"
        ));
        let greedy_sol = greedy_dsatur(inst.clone(), false);
        println!("initial solution: {}", greedy_sol.len());
        let stopping_criterion:TimeStoppingCriterion = TimeStoppingCriterion::new(3000.);
        tabucol_with_solution(inst, &greedy_sol, stopping_criterion, None);
    }

    #[test]
    fn test_tabu_with_greedy_cgshop5() {
        let inst = Rc::new(CGSHOPInstance::from_file(
            "./insts/cgshop_22_examples/visp_10K.instance.json"
        ));
        let greedy_sol = greedy_dsatur(inst.clone(), false);
        println!("initial solution: {}", greedy_sol.len());
        let stopping_criterion:TimeStoppingCriterion = TimeStoppingCriterion::new(3000.);
        tabucol_with_solution(inst, &greedy_sol, stopping_criterion, None);
    }
}