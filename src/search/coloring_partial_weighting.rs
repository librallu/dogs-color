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
        sol_filename
    );
    solution
}


#[cfg(test)]
mod tests {
    use super::*;

    use dogs::search_algorithm::TimeStoppingCriterion;
    
    use crate::{cgshop::CGSHOPInstance, search::greedy_dsatur::greedy_dsatur};

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
            "./insts/cgshop22/rvispecn17968.instance.json"
            // "./insts/cgshop22/sqrp7730.instance.json"
            // "./insts/cgshop22/sqrpecn18520.instance.json"
            // "./insts/cgshop22/sqrpecn32073.instance.json"
            // "./insts/cgshop22/sqrp20166.instance.json"
            // "./insts/cgshop_22_examples/visp_5K.instance.json"
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



/*
$ i=rvispecn17968 && ./bazel-bin/minimumplanepartitionsolver/main -v -i "./data/cgshop2022/instances/${i}.instance.json" --reduce -c "${i}.solution.json" -a localsearch_rowweighting
Instance:            rvispecn17968
Number of vertices:  27058
Number of edges:     17968
Maximum degree:      6
Number of dominated edges: 156
*** localsearch_rowweighting ***
T (s)       UB          LB          GAP         GAP (%)     
127.063     17969       0           17969       inf         
127.961     269         0           269         inf         greedy orientation
134.956     243         0           243         inf         greedy dsatur
139.776     242         0           242         inf         
139.89      241         0           241         inf         
139.975     240         0           240         inf         
140.083     239         0           239         inf         
140.168     238         0           238         inf         
140.258     237         0           237         inf         
140.371     236         0           236         inf         
140.463     235         0           235         inf         
140.554     234         0           234         inf         
140.67      233         0           233         inf         
140.768     232         0           232         inf         
140.853     231         0           231         inf         
140.968     230         0           230         inf         
141.057     229         0           229         inf         
141.153     228         0           228         inf         
141.262     227         0           227         inf         
141.374     226         0           226         inf         
141.476     225         0           225         inf         
141.636     224         0           224         inf         
141.798     223         0           223         inf         
141.937     222         0           222         inf         
142.419     221         0           221         inf         
370.894     220         0           220         inf         
374.453     219         0           219         inf         
374.783     218         0           218         inf         
386.475     217         0           217         inf         
386.981     216         0           216         inf         
388.408     215         0           215         inf         
389.525     214         0           214         inf         
390.75      213         0           213         inf         
452.31      212         0           212         inf         
455.062     211         0           211         inf         
456.066     210         0           210         inf         
461.661     209         0           209         inf         
466.111     208         0           208         inf         
467.358     207         0           207         inf         
476.46      206         0           206         inf         
478.112     205         0           205         inf         
481.611     204         0           204         inf         
485.795     203         0           203         inf         
493.377     202         0           202         inf         
503.736     201         0           201         inf         
552.916     200         0           200         inf         
559.894     199         0           199         inf         
577.864     198         0           198         inf         
582.352     197         0           197         inf         
593.217     196         0           196         inf         
721.128     195         0           195         inf         
779.644     194         0           194         inf         
834.019     193         0           193         inf         
846.482     192         0           192         inf         
873.633     191         0           191         inf         
999.835     190         0           190         inf         
1138.08     189         0           189         inf         
1156.72     188         0           188         inf         
1199.65     187         0           187         inf         
1523.91     186         0           186         inf         
1576.8      185         0           185         inf         
1640.89     184         0           184         inf         
2621.6      183         0           183         inf         
3127.13     182         0           182         inf         
3581.15     181         0           181         inf         
7210.1      180         0           180         inf         
14495.4     179         0           179         inf  
*/

/*
$ i=rvispecn13421 && ./bazel-bin/minimumplanepartitionsolver/main -v -i "./data/cgshop2022/instances/${i}.instance.json" --reduce -c "${i}.solution.json" -a localsearch_rowweighting
Instance:            rvispecn13421
Number of vertices:  20102
Number of edges:     13421
Maximum degree:      6
Number of dominated edges: 146
*** localsearch_rowweighting ***
T (s)       UB          LB          GAP         GAP (%)     
100.163     13422       0           13422       inf         
101.031     249         0           249         inf         greedy orientation
108.513     227         0           227         inf         greedy dsatur
115.013     226         0           226         inf         
115.091     225         0           225         inf         
115.216     224         0           224         inf         
115.381     223         0           223         inf         
115.628     222         0           222         inf         
115.868     221         0           221         inf         
116.034     220         0           220         inf         
116.269     219         0           219         inf         
116.515     218         0           218         inf         
116.699     217         0           217         inf         
116.931     216         0           216         inf         
117.191     215         0           215         inf         
117.37      214         0           214         inf         
117.632     213         0           213         inf         
117.858     212         0           212         inf         
118.134     211         0           211         inf         
118.443     210         0           210         inf         
118.778     209         0           209         inf         
119.116     208         0           208         inf         
119.512     207         0           207         inf         
119.971     206         0           206         inf         
120.545     205         0           205         inf         
121.405     204         0           204         inf         
121.77      203         0           203         inf         
122.04      202         0           202         inf         
604.621     201         0           201         inf         
605.423     200         0           200         inf         
606.777     199         0           199         inf         
609.211     198         0           198         inf         
610.469     197         0           197         inf         
613.23      196         0           196         inf         
614.602     195         0           195         inf         
616.379     194         0           194         inf         
618.733     193         0           193         inf         
621.425     192         0           192         inf         
641.171     191         0           191         inf         
791.344     190         0           190         inf         
796.652     189         0           189         inf         
797.495     188         0           188         inf         
801.274     187         0           187         inf         
806.64      186         0           186         inf         
808.79      185         0           185         inf         
811.157     184         0           184         inf         
828.807     183         0           183         inf         
835.656     182         0           182         inf         
840.805     181         0           181         inf         
849.538     180         0           180         inf         
863.741     179         0           179         inf         
880.247     178         0           178         inf         
883.37      177         0           177         inf         
1303.66     176         0           176         inf         
1338.04     175         0           175         inf         
1375.71     174         0           174         inf         
1469.92     173         0           173         inf         
1508.48     172         0           172         inf         
1532.34     171         0           171         inf         
1587.05     170         0           170         inf         
1799.16     169         0           169         inf         
1978.01     168         0           168         inf         
2254.8      167         0           167         inf         
2369.95     166         0           166         inf         
2422.98     165         0           165         inf         
3342.01     164         0           164         inf         
3520.31     163         0           163         inf         
3639.35     162         0           162         inf     
 */