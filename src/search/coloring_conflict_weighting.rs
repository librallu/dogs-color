use std::{cell::RefCell, rc::Rc};

use bit_set::BitSet;
use fastrand::Rng;

use dogs::{
    combinators::{helper::tabu_tenure::TabuTenure, stats::StatTsCombinator},
    data_structures::sparse_set::SparseSet,
    metric_logger::MetricLogger,
    search_algorithm::StoppingCriterion,
    search_space::{SearchSpace, TotalNeighborGeneration, GuidedSpace, ToSolution},
    tree_search::greedy::Greedy,
    search_algorithm::SearchAlgorithm
};

use crate::{
    color::{ColoringInstance, VertexId},
    util::export_results
};

type Weight = u16;

/// models a decision within the local search.
#[derive(Debug,Clone,Eq,PartialEq,Hash)]
struct Node {
    pub vertex:VertexId, // vertex to change
    pub previous_color:usize, // previous color of vertex v
    pub next_color:usize, // next color of vertex v
    pub total_penalties:Weight, // total Weight associated with the decision
    pub nb_conflicts:i64, // number of conflicts
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
        self.decisions[d.vertex][d.previous_color] = self.nb_iter;
        self.threshold = self.rng.i64(0..self.l as i64) + (self.lambda * (n.nb_conflicts as f64)) as i64;
    }

    fn contains(&mut self, _n:&Node, d:&Node) -> bool {
        self.decisions[d.vertex][d.next_color] >= self.nb_iter - self.threshold
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

/** implements a conflict weighting local search */
#[derive(Debug)]
struct ConflictWeightingLocalSearch {
    /// instance object
    inst:Rc<dyn ColoringInstance>,
    /// weights[u][v]: weight learned for the edge (u,v)
    weights:Vec<Vec<Weight>>,
    /// current best feasible solution 
    current_sol:Vec<Vec<VertexId>>,
    /// colors[v]: color of vertex v
    colors:Vec<usize>,
    /// colors_bitset[c]: color c vertices
    colors_bitsets:Vec<BitSet>,
    /// colors_vertex_number[c]: number of vertices coloried with c
    colors_vertex_number:Vec<usize>,
    /// weights_neigh_colors[v][c]: weights of neighbors of v that are assigned color c
    weights_neigh_colors:Vec<Vec<Weight>>,
    /// conflicting_vertices: list of vertices that have some conflict
    conflicting_vertices:SparseSet,
    /// vertex_nb_conflicts[v]: number of conflicts for the vertex v
    vertex_nb_conflicts:Vec<i64>,
    /// number of conflicting edges
    nb_conflicting_edges:i64,
    /// total weight in the current state
    total_weight:Weight,
    /// tabu list
    tabu:TabuColTenure,
    /// threshold on the number of conflicts to disable the tabu tenure
    aspiration_criterion:i64,
    /// number of iterations
    nb_iter:i64,
    /// number of colors at the beginning of the search
    nb_colors:usize,
    /// number of colors removed since the beginning of the search (best-so-far coloring)
    best_so_far_colors:usize,
}

impl ConflictWeightingLocalSearch {

    /// initializes the data-structure from an initial solution 
    fn initialize(inst:Rc<dyn ColoringInstance>, sol:&[Vec<VertexId>]) -> Self {
        // build colors & colors_bitsets
        let n = inst.nb_vertices();
        let nb_colors = sol.len();
        let mut colors = vec![0 ; n];
        let mut colors_bitsets = vec![BitSet::with_capacity(n); nb_colors];
        let mut colors_vertex_number = vec![0 ; nb_colors];
        for (i,c) in sol.iter().enumerate() {
            for v in c {
                colors[*v] = i;
                colors_bitsets[i].insert(*v);
                colors_vertex_number[i] += 1;
            }
        }
        // build weights_neigh_colors
        let mut weights_neigh_colors = vec![vec![ 0 ; nb_colors] ; n];
        for u in inst.vertices() {
            for v in inst.neighbors(u) {
                weights_neigh_colors[u][colors[v]] += 1;
            }
        }
        Self {
            inst,
            weights: (0..n).map(|i| vec![1 ; i]).collect(),
            current_sol: sol.to_vec(),
            colors,
            colors_bitsets,
            colors_vertex_number,
            weights_neigh_colors,
            conflicting_vertices: SparseSet::new(n),
            vertex_nb_conflicts: vec![0 ; n],
            nb_conflicting_edges: 0,
            total_weight: 0,
            tabu: TabuColTenure::new(10, 0.6, n, nb_colors),
            aspiration_criterion: i64::MAX,
            nb_iter: 0,
            nb_colors,
            best_so_far_colors: nb_colors,
        }
    }

    /// merges 2 colors
    fn merge_colors(&mut self) {
        loop { // invariant: self.current_sol is feasible
            // find the best color merge (couple of colors that minimize the conflict penalties)
            let mut best_weight:Weight = Weight::MAX;
            let mut current_best_color_merge:Option<(VertexId,VertexId)> = None;
            for (i1, color1) in self.current_sol.iter().enumerate() {
                for (i2, color2) in self.current_sol.iter().enumerate() {
                    if i2 < i1 && !color1.is_empty() && !color2.is_empty() {
                        let current_weight = color1.iter().map(|u| self.weights_neigh_colors[*u][i2]).sum();
                        if current_weight < best_weight || current_best_color_merge.is_none() {
                            best_weight = current_weight;
                            current_best_color_merge = Some((i2, i1));
                        }
                    }
                }
            }
            // perform the merge
            let (c1,c2) = current_best_color_merge.unwrap();
            let c_min; let c_max;
            if self.current_sol[c1] < self.current_sol[c2] {
                c_min = c1; c_max = c2;
            } else {
                c_min = c2; c_max = c1;
            }
            // for every vertex in the minimum color, change it to the maximum color
            for v in self.current_sol[c_min].clone() {
                self.change_vertex_color(v, c_max);
            }
            // quit if there are some conflicts, otherwise, repeat
            if self.is_goal() { // if goal, update the solution
                self.update_current_solution();
            } else { break; }
        }
    }

    /// applies a move (coloring a vertex with a color)
    fn commit(&mut self, node:&Node) {
        // mark the move tabu
        self.tabu.insert(node, node.clone()); // make the decision tabu
        self.tabu.increment_iter();
        self.nb_iter += 1;
        self.change_vertex_color(node.vertex, node.next_color);
    }


    /// change the color of vertex v, to color c
    fn change_vertex_color(&mut self, v:VertexId, next_color:usize) {
        let previous_color = self.colors[v];
        self.colors[v] = next_color; // change colors
        self.colors_bitsets[previous_color].remove(v);
        self.colors_bitsets[next_color].insert(v);
        self.colors_vertex_number[previous_color] -= 1;
        self.colors_vertex_number[next_color] += 1;
        self.total_weight = self.total_weight +
            self.weights_neigh_colors[v][next_color] - self.weights_neigh_colors[v][previous_color];
        // update weights & vertex_nb_conflicts
        for neigh in self.inst.neighbors(v) {
            let weight = self.get_weight(neigh, v);
            self.weights_neigh_colors[neigh][previous_color] -= weight;
            self.weights_neigh_colors[neigh][next_color] += weight;
            if self.colors[neigh] == previous_color { // remove conflict
                self.vertex_nb_conflicts[neigh] -= 1;
                self.vertex_nb_conflicts[v] -= 1;
                self.nb_conflicting_edges -= 1;
            }
            if self.colors[neigh] == next_color { // add conflict
                self.vertex_nb_conflicts[neigh] += 1;
                self.vertex_nb_conflicts[v] += 1;
                self.nb_conflicting_edges += 1;
                // update weights when conflicts enter
                self.increase_weight(neigh, v);
                self.weights_neigh_colors[neigh][next_color] += 1;
                self.weights_neigh_colors[v][next_color] += 1;
                self.conflicting_vertices.insert(neigh);
                self.conflicting_vertices.insert(v);
                self.total_weight += 1;
            }
        }
        self.aspiration_criterion = std::cmp::min(self.aspiration_criterion, self.nb_conflicting_edges);
        if self.is_goal() { // if goal, update the solution
            self.update_current_solution();
        }
    }

    /// update the current solution
    fn update_current_solution(&mut self) {
        assert!(self.is_goal());
        let mut new_solution = vec![vec![] ; self.nb_colors];
        for (v,c) in self.colors.iter().enumerate() {
            new_solution[*c].push(v);
        }
        self.current_sol = new_solution;
        self.best_so_far_colors = self.current_sol.iter().filter(|e| !e.is_empty()).count();
    }

    /// get the learned weight of an edge
    fn get_weight(&self, u:VertexId, v:VertexId) -> Weight {
        if u < v { self.weights[v][u] }
        else { self.weights[u][v] }
    }

    /// increase the learned weight of an edge
    fn increase_weight(&mut self, u:VertexId, v:VertexId) {
        if u < v { self.weights[v][u] += 1 }
        else { self.weights[u][v] += 1 }
    }

    /// true iff state is feasible
    fn is_goal(&self) -> bool { self.total_weight == 0 }
}

impl GuidedSpace<Node, i64> for ConflictWeightingLocalSearch {
    fn guide(&mut self, node: &Node) -> i64 {
        node.total_penalties as i64
    }
}

impl ToSolution<Node, Vec<Vec<VertexId>>> for ConflictWeightingLocalSearch {
    fn solution(&mut self, _: &mut Node) -> Vec<Vec<VertexId>> {
        self.current_sol.iter().filter(|e| !e.is_empty()).cloned().collect()
    }
}

impl SearchSpace<Node, i32> for ConflictWeightingLocalSearch {
    fn initial(&mut self) -> Node {
        Node {
            vertex: 0,
            previous_color: self.colors[0],
            next_color: self.colors[0],
            total_penalties: 0,
            nb_conflicts: 0,
        }
    }
    fn bound(&mut self, _node: &Node) -> i32 { self.best_so_far_colors as i32 }
    fn goal(&mut self, n: &Node) -> bool { n.nb_conflicts == 0 }
    fn g_cost(&mut self, _n: &Node) -> i32 { 0 }
}

impl TotalNeighborGeneration<Node> for ConflictWeightingLocalSearch {
    fn neighbors(&mut self, node: &mut Node) -> Vec<Node> {
        if node.previous_color != node.next_color { // if not a dummy decision, commit it
            self.commit(node);
        }
        if self.goal(node) { // if no conflict, merge some colors
            assert!(self.is_goal()); // the search state should be a goal here
            self.merge_colors();
        }
        let mut best_nodes = vec![
            Node {vertex:0, previous_color:0, next_color:0, total_penalties:Weight::MAX, nb_conflicts:0}
        ];
        let mut i = 0;
        while i < self.conflicting_vertices.len() { // iterate over conflicting vertices
            let u = self.conflicting_vertices.nth(i);
            if self.vertex_nb_conflicts[u] > 0 { // u has indeed some conflicts
                // for each vertex, try changing its color by an existing other color
                for c in 0..self.nb_colors {
                    if c != self.colors[u] && self.colors_vertex_number[c] > 0 {
                        let current_penalties:Weight = self.total_weight +
                            self.weights_neigh_colors[u][c] - self.weights_neigh_colors[u][self.colors[u]];
                        if current_penalties <= best_nodes[0].total_penalties {
                            let current_node = Node {
                                vertex:u,
                                previous_color:self.colors[u],
                                next_color:c,
                                total_penalties:current_penalties,
                                nb_conflicts: self.nb_conflicting_edges
                            };
                            let is_tabu = self.tabu.contains(&current_node, &current_node);
                            if !is_tabu || self.nb_conflicting_edges < self.aspiration_criterion {
                                if current_penalties < best_nodes[0].total_penalties {
                                    best_nodes.clear();
                                }
                                best_nodes.push(current_node); 
                            }
                        }
                    }
                }
                i += 1;
            } else {
                self.conflicting_vertices.remove(u); // update conflicting_vertices if it has no conflict
            }
        }
        best_nodes
    }
}


/** performs a conflict weighting local search. */
pub fn coloring_conflict_weighting<Stopping:StoppingCriterion>(
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
            ConflictWeightingLocalSearch::initialize(inst.clone(), &solution),
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
            assert_eq!(node.nb_conflicts, 0);
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
    
    use crate::{cgshop::CGSHOPInstance, search::greedy_dsatur::greedy_dsatur};

    #[test]
    fn test_cwls() {
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
            // "./insts/cgshop22/sqrp20166.instance.json"
            // "./insts/cgshop22/sqrpecn18520.instance.json"
            "./insts/cgshop22/sqrpecn32073.instance.json"
            // "./insts/cgshop_22_examples/visp_5K.instance.json"
            // "./insts/cgshop_22_examples/sqrm_10K_5.instance.json"
        ));
        let greedy_sol = greedy_dsatur(inst.clone(), false);
        println!("initial solution: {}", greedy_sol.len());
        let stopping_criterion:TimeStoppingCriterion = TimeStoppingCriterion::new(3600.);
        coloring_conflict_weighting(
            inst, &greedy_sol, None, None , stopping_criterion
        );
    }

}

