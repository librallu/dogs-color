use std::{cell::RefCell, rc::Rc};

use bit_set::BitSet;
use fastrand::Rng;

use dogs::{combinators::{helper::tabu_tenure::TabuTenure, stats::StatTsCombinator}, data_structures::sparse_set::SparseSet, metric_logger::MetricLogger, search_algorithm::SearchAlgorithm, search_algorithm::StoppingCriterion, search_space::{SearchSpace, TotalNeighborGeneration, GuidedSpace, ToSolution}, tree_search::greedy::Greedy};

use crate::{
    color::{ColoringInstance, VertexId},
    util::export_results
};

type Weight = i32;

/// models a decision within the local search.
#[derive(Debug,Clone,Eq,PartialEq,Hash)]
struct Node {
    pub vertex:Option<VertexId>, // vertex to change
    pub color:usize, // color to use
    pub total_weight:Weight, // total Weight associated with the decision
    pub nb_uncolored:usize, // number of uncolored vertices
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
    decisions: Vec<Vec<i64>>,
    /// random number generator
    rng: Rng,
    /// threshold value for a given iteration
    threshold: i64,
}

impl TabuTenure<Node, Node> for TabuColTenure {
    fn insert(&mut self, n:&Node, d:Node) {
        if d.vertex.is_some() {
            self.decisions[d.vertex.unwrap()][d.color] = self.nb_iter;
            self.threshold = self.rng.i64(0..self.l as i64) + (self.lambda * (n.nb_uncolored as f64)) as i64;
        }
    }

    fn contains(&mut self, _n:&Node, d:&Node) -> bool {
        self.decisions[d.vertex.unwrap()][d.color] >= self.nb_iter - self.threshold
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
            decisions: vec![vec![i64::MIN ; c] ; n],
            rng: Rng::new(),
            threshold: 0, // will be changed later
        }
    }

    /// increases the number of iterations of the tabu tenure
    pub fn increment_iter(&mut self) { self.nb_iter += 1; }
}

/** implements a partial weighting local search */
#[derive(Debug)]
struct PartialWeightingLocalSearch {
    /// instance object
    inst:Rc<dyn ColoringInstance>,
    /// weights[u]: weight learned for vertex u
    weights:Vec<Weight>,
    /// current best feasible solution 
    current_sol:Vec<Vec<VertexId>>,
    /// colors[v]: color of vertex v
    colors:Vec<Option<usize>>,
    /// colors_vertices[c]: bitset of vertices using color c
    colors_vertices:Vec<BitSet>,
    /// colors_vertex_number[c]: number of vertices using color c
    colors_vertex_number:Vec<usize>,
    /// number of colors used
    nb_colors:usize,
    /// number of initial colors
    nb_initial_colors:usize,
    /// best number of colors in the best-so-far solution
    nb_colors_best_so_far:usize,
    /// total weight in the current state
    total_weight:Weight,
    /// set of uncolored vertices
    uncolored_vertices:SparseSet,
    /// cost_coloring[u][c]: cost of coloring vertex u with color c
    cost_coloring:Vec<Vec<Weight>>,
    /// tabu list
    tabu:TabuColTenure,
    /// threshold on the number of conflicts to disable the tabu tenure
    aspiration_criterion:i64,
    /// number of iterations
    nb_iter:i64,
    /// random number generator
    rng:Rng,
}

impl PartialWeightingLocalSearch {

    /// initializes the data-structure from an initial solution 
    fn initialize(inst:Rc<dyn ColoringInstance>, sol:&[Vec<VertexId>]) -> Self {
        // build colors & colors_bitsets
        let n = inst.nb_vertices();
        let nb_colors = sol.len();
        let mut colors = vec![None ; n];
        let mut colors_vertex_number = vec![0 ; n];
        let mut colors_vertices = vec![BitSet::new() ; n];
        for (i,c) in sol.iter().enumerate() {
            for v in c {
                colors[*v] = Some(i);
                colors_vertices[i].insert(*v);
                colors_vertex_number[i] += 1;
            }
        }
        let mut cost_coloring = vec![vec![0 ; sol.len()] ; n];
        for u in inst.vertices() {
            for (c,c_vertices) in sol.iter().enumerate() {
                for v in c_vertices {
                    if inst.are_adjacent(u, *v) {
                        cost_coloring[u][c] += 1;
                    }
                }
            }
        }
        Self {
            inst,
            weights: vec![1 ; n],
            current_sol: sol.to_vec(),
            colors,
            colors_vertices,
            colors_vertex_number,
            nb_colors: sol.len(),
            nb_initial_colors: sol.len(),
            nb_colors_best_so_far: sol.len(),
            total_weight: 0,
            uncolored_vertices: SparseSet::new(n),
            cost_coloring,
            tabu: TabuColTenure::new(10, 0.01, n, nb_colors),
            aspiration_criterion: i64::MAX,
            nb_iter: 0,
            rng: Rng::default(),
        }
    }

    /// uncolors a vertex
    fn uncolor_vertex(&mut self, u:VertexId) {
        let previous_color:usize = self.colors[u]
            .unwrap_or_else(|| panic!("{} should have a color", u));
        self.colors_vertex_number[previous_color] -= 1;
        self.colors_vertices[previous_color].remove(u);
        self.total_weight += self.weights[u];
        self.colors[u] = None;
        self.uncolored_vertices.insert(u);
        // decrease the coloring cost of the neighbors of u
        for v in self.inst.neighbors(u) {
            self.cost_coloring[v][previous_color] -= self.weights[u];
        }
        // make the insertion of u tabu
        let node  = Node {vertex:Some(u), color: previous_color, total_weight: 0, nb_uncolored: 0};
        self.tabu.insert(&node, node.clone()); // make the decision tabu
    }

    /// colors a vertex
    fn color_vertex(&mut self, u:VertexId, c:usize) {
        // update the weight
        self.total_weight -= self.weights[u];
        // increase the weight
        self.weights[u] += 1;
        // colors a vertex
        self.colors[u] = Some(c);
        self.colors_vertex_number[c] += 1;
        self.colors_vertices[c].insert(u);
        self.uncolored_vertices.remove(u);
        // increase the coloring cost of the neighbors of u
        for v in self.inst.neighbors(u) {
            self.cost_coloring[v][c] += self.weights[u];
        }
        // uncolors conflicting vertices
        let mut to_uncolor = Vec::new();
        for v in self.colors_vertices[c].iter() {
            if self.inst.are_adjacent(u, v) {
                to_uncolor.push(v);
            }
        }
        for v in to_uncolor {
            self.uncolor_vertex(v);
        }
    }

    fn check_weights(&self) {
        // let n = self.inst.nb_vertices();
        // let c = self.nb_initial_colors;
        // let mut total_weight:Weight = 0;
        // let mut cost_coloring:Vec<Vec<Weight>> = vec![vec![0 ; c] ; n];
        // for (v,c) in self.colors.iter().enumerate() {
        //     match c {
        //         None => {
        //             total_weight += self.weights[v];
        //         },
        //         Some(_) => {}
        //     };
        // }
        // for u in self.inst.vertices() {
        //     for v in self.inst.neighbors(u) {
        //         match self.colors[v] {
        //             None => {},
        //             Some(c) => {
        //                 cost_coloring[u][c] += self.weights[v];
        //             }
        //         }
        //     }
        // }
        // assert_eq!(cost_coloring, self.cost_coloring);
        // assert_eq!(total_weight, self.total_weight);
    }

    /// removes the color using the leas number of vertices
    fn delete_color(&mut self) {
        // find min-color
        let c_min = self.colors_vertex_number.iter().enumerate()
            .filter(|(_,c)| **c > 0) // (otherwise, the color is already not used)
            .max_by_key(|(_,c)| **c).unwrap().0;
        for i in self.inst.vertices() {
            match self.colors[i] {
                None => {},
                Some(c) => if c == c_min {
                    self.uncolor_vertex(i);
                }
            }
        }
        self.nb_colors -= 1;
    }   

    /// applies a move (coloring a vertex with a color)
    fn commit(&mut self, node:&Node) {
        // // mark the move tabu
        self.tabu.increment_iter();
        match node.vertex {
            None => {},
            Some(vertex) => {
                self.nb_iter += 1;
                self.color_vertex(vertex, node.color);
            }
        }
        self.check_weights();
        // assert_eq!(self.total_weight, node.total_weight);
        if self.is_goal() {
            self.update_current_solution();
            self.delete_color();
        }
    }

    /// update the current solution
    fn update_current_solution(&mut self) {
        assert!(self.is_goal());
        let mut new_solution = vec![vec![] ; self.nb_initial_colors];
        for (v,c) in self.colors.iter().enumerate() {
            new_solution[c.unwrap()].push(v);
        }
        self.current_sol = new_solution;
        self.nb_colors_best_so_far -= 1;
    }

    /// true iff state is feasible
    fn is_goal(&self) -> bool { self.total_weight == 0 }
}

impl GuidedSpace<Node, i64> for PartialWeightingLocalSearch {
    fn guide(&mut self, node: &Node) -> i64 { node.total_weight as i64 }
}

impl ToSolution<Node, Vec<Vec<VertexId>>> for PartialWeightingLocalSearch {
    fn solution(&mut self, _: &mut Node) -> Vec<Vec<VertexId>> {
        self.current_sol.iter().filter(|e| !e.is_empty()).cloned().collect()
    }
}

impl SearchSpace<Node, i32> for PartialWeightingLocalSearch {
    fn initial(&mut self) -> Node {
        Node {
            vertex: None,
            color: 0,
            total_weight: 0,
            nb_uncolored: 0,
        }
    }
    fn bound(&mut self, _node: &Node) -> i32 { self.nb_colors_best_so_far as i32 }
    fn goal(&mut self, n: &Node) -> bool { n.total_weight == 0 }
    fn g_cost(&mut self, _n: &Node) -> i32 { 0 }
}

impl TotalNeighborGeneration<Node> for PartialWeightingLocalSearch {
    fn neighbors(&mut self, node: &mut Node) -> Vec<Node> {
        self.commit(node);
        let mut best_nodes = vec![
            Node {
                vertex:None,
                color:0,
                total_weight:Weight::MAX,
                nb_uncolored:self.uncolored_vertices.len()
            }
        ];
        // if self.nb_iter % 30_000 == 0 {
        //     // uncolor 10% of the vertices
        //     println!("Before Kick: {}\t initial weight {}\t uncolored {}", self.nb_iter, self.total_weight, self.uncolored_vertices.len());
        //     for v in self.inst.vertices() {
        //         if !self.uncolored_vertices.contains(v) {
        //             let r:i64 = self.rng.i64(0..10);
        //             if r == 0 {
        //                 self.uncolor_vertex(v);
        //             }
        //         }
        //     }
        //     println!("After Kick: {}\t initial weight {}\t uncolored {}", self.nb_iter, self.total_weight, self.uncolored_vertices.len());
        // }
        // for every uncolored vertex, try a possible color
        for u in self.uncolored_vertices.iter() {
            for (c,_) in self.colors_vertex_number.iter().enumerate().filter(|(_,n)| **n > 0) {
                let weight = self.total_weight + self.cost_coloring[u][c] - self.weights[u];
                if weight <= best_nodes[0].total_weight {
                    let current_node = Node {
                        vertex: Some(u),
                        color: c,
                        total_weight: weight,
                        nb_uncolored: self.uncolored_vertices.len(),
                    };
                    let is_tabu = self.tabu.contains(&current_node, &current_node);
                    if best_nodes[0].total_weight == Weight::MAX || !is_tabu {
                        if weight < best_nodes[0].total_weight {
                            best_nodes.clear();
                        }
                        best_nodes.push(current_node);
                    }
                }
            }
        }
        best_nodes
    }
}


/** performs a partial weighting local search. */
pub fn coloring_partial_weighting<Stopping:StoppingCriterion>(
inst:Rc<dyn ColoringInstance>,
sol:&[Vec<VertexId>],
perf_filename:Option<String>,
sol_filename:Option<String>,
stop:Stopping
) -> Vec<Vec<VertexId>> {
    let mut solution:Vec<Vec<VertexId>> = sol.to_vec();
    let logger = Rc::new(MetricLogger::default());
    let space = Rc::new(RefCell::new(
        StatTsCombinator::new(
            PartialWeightingLocalSearch::initialize(inst.clone(), &solution),
        ).bind_logger(Rc::downgrade(&logger)),
    ));
    let mut ts = Greedy::new(space.clone());
    logger.display_headers();
    ts.run(stop);
    // display the results afterwards
    space.borrow_mut().display_statistics();
    // check that the last solution is valid
    match ts.get_manager().best() {
        None => {
            println!("\tlocal search failed improving...");
        }
        Some(node) => {
            assert_eq!(node.total_weight, 0);
            solution = space.borrow_mut().solution(&mut node.clone());
        }  
    }
    let mut stats = serde_json::Value::default();
    space.borrow_mut().json_statistics(&mut stats);
    export_results(
        inst,
        &solution,
        &stats,
        perf_filename,
        sol_filename,
        true
    );
    solution
}


#[cfg(test)]
mod tests {
    use super::*;

    use dogs::search_algorithm::TimeStoppingCriterion;
    
    use crate::{cgshop::CGSHOPInstance, solvers::coloring::greedy_dsatur::greedy_dsatur};

    #[test]
    fn test_pwls() {
        let inst = Rc::new(CGSHOPInstance::from_file(
            // "./insts/cgshop22/vispecn2518.instance.json"
            // "./insts/cgshop22/reecn3382.instance.json"
            // "./insts/cgshop22/rvisp3499.instance.json"
            // "./insts/cgshop22/rvisp14562.instance.json"
            // "./insts/cgshop22/reecn9674.instance.json"
            // "./insts/cgshop22/reecn12588.instance.json"
            // "./insts/cgshop22/reecn25913.instance.json"
            // "./insts/cgshop22/reecn31126.instance.json"
            // "./insts/cgshop22/reecn73116.instance.json"
            // "./insts/cgshop22/rvispecn17968.instance.json"
            // "./insts/cgshop22/sqrp7730.instance.json"
            // "./insts/cgshop22/sqrpecn18520.instance.json"
            // "./insts/cgshop22/sqrpecn32073.instance.json"
            // "./insts/cgshop22/sqrp20166.instance.json"
            // "./insts/cgshop_22_examples/visp_5K.instance.json"
            "./insts/cgshop_22_examples/visp_100K.instance.json"
            // "./insts/cgshop_22_examples/sqrm_10K_5.instance.json"
        ));
        let greedy_sol = greedy_dsatur(inst.clone(), false);
        println!("initial solution: {}", greedy_sol.len());
        let stopping_criterion:TimeStoppingCriterion = TimeStoppingCriterion::new(180.);
        coloring_partial_weighting(
            inst, &greedy_sol, None, None , stopping_criterion
        );
    }

    #[test]
    fn test_pwls2() {
        let inst = Rc::new(CGSHOPInstance::from_file(
            // "./insts/cgshop22/vispecn2518.instance.json"
            // "./insts/cgshop22/reecn3382.instance.json"
            // "./insts/cgshop22/rvisp3499.instance.json"
            // "./insts/cgshop22/rvisp14562.instance.json"
            // "./insts/cgshop22/reecn9674.instance.json"
            // "./insts/cgshop22/reecn12588.instance.json"
            // "./insts/cgshop22/reecn25913.instance.json"
            // "./insts/cgshop22/reecn31126.instance.json"
            // "./insts/cgshop22/reecn73116.instance.json"
            "./insts/cgshop22/rvispecn17968.instance.json"
            // "./insts/cgshop22/vispecn17665.instance.json"
            // "./insts/cgshop22/visp32354.instance.json"
            // "./insts/cgshop_22_examples/visp_5K.instance.json"
            // "./insts/cgshop_22_examples/sqrm_10K_5.instance.json"
        ));
        let greedy_sol = greedy_dsatur(inst.clone(), false);
        println!("initial solution: {}", greedy_sol.len());
        let stopping_criterion:TimeStoppingCriterion = TimeStoppingCriterion::new(3600.);
        coloring_partial_weighting(
            inst, &greedy_sol, None, None , stopping_criterion
        );
    }

    #[test]
    fn test_pwls3() {
        let inst = Rc::new(CGSHOPInstance::from_file(
            // "./insts/cgshop22/vispecn2518.instance.json"
            // "./insts/cgshop22/reecn3382.instance.json"
            // "./insts/cgshop22/rvisp3499.instance.json"
            // "./insts/cgshop22/rvisp14562.instance.json"
            // "./insts/cgshop22/reecn9674.instance.json"
            // "./insts/cgshop22/reecn12588.instance.json"
            // "./insts/cgshop22/reecn25913.instance.json"
            // "./insts/cgshop22/reecn31126.instance.json"
            // "./insts/cgshop22/reecn73116.instance.json"
            "./insts/cgshop22/vispecn5478.instance.json"
            // "./insts/cgshop22/rvispecn17968.instance.json"
            // "./insts/cgshop22/vispecn17665.instance.json"
            // "./insts/cgshop22/visp26405.instance.json"
            // "./insts/cgshop_22_examples/visp_5K.instance.json"
            // "./insts/cgshop_22_examples/sqrm_10K_5.instance.json"
        ));
        let greedy_sol = greedy_dsatur(inst.clone(), false);
        println!("initial solution: {}", greedy_sol.len());
        let stopping_criterion:TimeStoppingCriterion = TimeStoppingCriterion::new(3600.);
        coloring_partial_weighting(
            inst, &greedy_sol, None, None , stopping_criterion
        );
    }

    #[test]
    fn test_pwls4() {
        let inst = Rc::new(CGSHOPInstance::from_file(
            // "./insts/cgshop22/vispecn2518.instance.json"
            // "./insts/cgshop22/reecn3382.instance.json"
            // "./insts/cgshop22/rvisp3499.instance.json"
            // "./insts/cgshop22/rvisp14562.instance.json"
            // "./insts/cgshop22/reecn9674.instance.json"
            // "./insts/cgshop22/reecn12588.instance.json"
            // "./insts/cgshop22/reecn25913.instance.json"
            // "./insts/cgshop22/reecn31126.instance.json"
            // "./insts/cgshop22/reecn73116.instance.json"
            // "./insts/cgshop22/rvispecn17968.instance.json"
            // "./insts/cgshop22/vispecn17665.instance.json"
            "./insts/cgshop22/vispecn13806.instance.json"
            // "./insts/cgshop22/visp26405.instance.json"
            // "./insts/cgshop_22_examples/visp_5K.instance.json"
            // "./insts/cgshop_22_examples/sqrm_10K_5.instance.json"
        ));
        let greedy_sol = greedy_dsatur(inst.clone(), false);
        println!("initial solution: {}", greedy_sol.len());
        let stopping_criterion:TimeStoppingCriterion = TimeStoppingCriterion::new(3600.);
        coloring_partial_weighting(
            inst, &greedy_sol, None, None , stopping_criterion
        );
    }

    #[test]
    fn test_pwls5() {
        let inst = Rc::new(CGSHOPInstance::from_file(
            // "./insts/cgshop22/vispecn2518.instance.json"
            // "./insts/cgshop22/reecn3382.instance.json"
            // "./insts/cgshop22/rvisp3499.instance.json"
            // "./insts/cgshop22/rvisp14562.instance.json"
            // "./insts/cgshop22/reecn9674.instance.json"
            // "./insts/cgshop22/reecn12588.instance.json"
            // "./insts/cgshop22/reecn25913.instance.json"
            // "./insts/cgshop22/reecn31126.instance.json"
            // "./insts/cgshop22/reecn73116.instance.json"
            // "./insts/cgshop22/rvispecn17968.instance.json"
            // "./insts/cgshop22/vispecn17665.instance.json"
            // "./insts/cgshop22/vispecn13806.instance.json"
            "./insts/cgshop22/visp48558.instance.json"
            // "./insts/cgshop_22_examples/visp_5K.instance.json"
            // "./insts/cgshop_22_examples/sqrm_10K_5.instance.json"
        ));
        let greedy_sol = greedy_dsatur(inst.clone(), false);
        println!("initial solution: {}", greedy_sol.len());
        let stopping_criterion:TimeStoppingCriterion = TimeStoppingCriterion::new(3600.);
        coloring_partial_weighting(
            inst, &greedy_sol, None, None , stopping_criterion
        );
    }

    #[test]
    fn test_pwls6() {
        let inst = Rc::new(CGSHOPInstance::from_file(
            // "./insts/cgshop22/vispecn2518.instance.json"
            // "./insts/cgshop22/reecn3382.instance.json"
            // "./insts/cgshop22/rvisp3499.instance.json"
            // "./insts/cgshop22/rvisp14562.instance.json"
            // "./insts/cgshop22/reecn9674.instance.json"
            // "./insts/cgshop22/reecn12588.instance.json"
            // "./insts/cgshop22/reecn25913.instance.json"
            // "./insts/cgshop22/reecn31126.instance.json"
            // "./insts/cgshop22/reecn73116.instance.json"
            // "./insts/cgshop22/rvispecn17968.instance.json"
            // "./insts/cgshop22/vispecn17665.instance.json"
            // "./insts/cgshop22/vispecn13806.instance.json"
            "./insts/cgshop22/vispecn37349.instance.json"
            // "./insts/cgshop22/visp26405.instance.json"
            // "./insts/cgshop_22_examples/visp_5K.instance.json"
            // "./insts/cgshop_22_examples/sqrm_10K_5.instance.json"
        ));
        let greedy_sol = greedy_dsatur(inst.clone(), false);
        println!("initial solution: {}", greedy_sol.len());
        let stopping_criterion:TimeStoppingCriterion = TimeStoppingCriterion::new(3600.);
        coloring_partial_weighting(
            inst, &greedy_sol, None, None , stopping_criterion
        );
    }

}
