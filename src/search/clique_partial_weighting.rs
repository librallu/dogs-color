use std::{cell::RefCell, rc::Rc};

use bit_set::BitSet;
use rand::{Rng, prelude::ThreadRng};

use dogs::{
    combinators::{
        helper::tabu_tenure::TabuTenure,
        stats::StatTsCombinator
    },
    metric_logger::MetricLogger,
    search_algorithm::SearchAlgorithm,
    search_algorithm::StoppingCriterion,
    search_space::{SearchSpace, TotalNeighborGeneration, GuidedSpace, ToSolution}, tree_search::greedy::Greedy
};

use crate::{color::{ColoringInstance, VertexId}, util::{clique_vec_to_vecvec, export_results}};

type Weight = u32;

/// models a decision within the local search.
#[derive(Debug,Clone,Eq,PartialEq,Hash)]
struct Node {
    pub vertex_in:Option<VertexId>, // vertex to include inside the candidate clique
    pub total_weight:Weight, // total weight after the decision
}


/** simple tabu tenure that stores the decisions taken */
#[derive(Debug,Clone)]
struct CliqueTenure {
    /// tabu fixed size
    l:usize,
    /// tabu dynamic size
    lambda: f64,
    /// number of iterations since the beginning of the search
    nb_iter: usize,
    /// decisions[c]: last iteration in which vertex c was inserted
    decisions: Vec<Option<usize>>,
    /// random number generator
    rng: ThreadRng,
}

impl TabuTenure<usize, usize> for CliqueTenure {
    fn insert(&mut self, _n:&usize, d:usize) {
        self.decisions[d] = Some(self.nb_iter);
    }

    fn contains(&mut self, n:&usize, d:&usize) -> bool {
        match self.decisions[*d] {
            None => false,
            Some(i) => {
                let rand_l = self.rng.gen_range(0..=self.l);
                let threshold = rand_l + (self.lambda * (*n as f64)) as usize;
                threshold > self.nb_iter || i >= self.nb_iter - threshold
            }
        }
    }
}

impl CliqueTenure {
    /** creates a tabu tenure given:
     - l: fixed tabu size
     - λ: variable tabu size
     - n: nb vertices
    */
    pub fn new(l:usize, lambda: f64, n:usize) -> Self {
        Self {
            l, lambda,
            nb_iter: 0,
            decisions: vec![None ; n],
            rng: rand::thread_rng(),
        }
    }

    /// increases the number of iterations of the underlying search process
    pub fn increase_iter(&mut self) {
        self.nb_iter += 1;
    }
}


/** implements a partial weighting local search */
#[derive(Debug)]
struct PartialWeightingLocalSearch {
    /// instance object
    inst:Rc<dyn ColoringInstance>,
    /// weights[v]: weight for the vertex v
    weights:Vec<Weight>,
    /// current best feasible solution 
    current_sol:Vec<VertexId>,
    /// inside_clique[v] = true iff v is in the current "clique"
    inside_clique:BitSet,
    /// total weight of the candidate clique
    total_weight:Weight,
    /// cost of weights by inserting v in the clique
    weight_cost_inserting:Vec<Weight>,
    /// nb_non_adj_clique[v]: number of non-adjacent vertices in the clique for v
    nb_non_adj_clique:Vec<usize>,
    /// tabu tenure
    tabu:CliqueTenure,
    /// counts the number of iterations
    nb_iter:u64,
    /// configuration for each vertex
    configuration:BitSet,
    /// aspiration criterion
    aspiration:usize,
}

impl PartialWeightingLocalSearch {

    /// initializes the data-structure from an initial solution 
    fn initialize(inst:Rc<dyn ColoringInstance>, sol:&[VertexId]) -> Self {
        // println!("{:?}", sol);
        let n = inst.nb_vertices();
        // build data-structures
        let mut inside_clique = BitSet::with_capacity(n);
        let mut weight_cost_inserting:Vec<Weight> = vec![0 ; n];
        let mut nb_non_adj_clique:Vec<usize> = vec![0 ; n];
        for v in sol {
            inside_clique.insert(*v);
            for u in inst.vertices().filter(|u| u!=v && !inst.are_adjacent(*u, *v)) {
                weight_cost_inserting[u] += 1;
                nb_non_adj_clique[u] += 1;
            }
        }
        let mut configuration = BitSet::default();
        for i in 0..n { configuration.insert(i); }
        Self {
            inst,
            weights: vec![1 ; n],
            current_sol: sol.to_vec(),
            inside_clique,
            total_weight:sol.len() as Weight,
            weight_cost_inserting,
            nb_non_adj_clique,
            tabu:CliqueTenure::new(n/5, 0.0, n),
            nb_iter: 0,
            configuration,
            aspiration: sol.len(),
        }
    }

    /// removes a vertex v from the clique
    fn remove_vertex(&mut self, v:VertexId) {
        debug_assert!(self.inside_clique.contains(v));
        self.inside_clique.remove(v);
        self.total_weight -= self.get_weight(v);
        // update weight cost inserting
        for w in self.inst.vertices().filter(|w| *w!=v) {
            if !self.inst.are_adjacent(v, w) {
                self.weight_cost_inserting[w] -= self.get_weight(v);
                self.nb_non_adj_clique[w] -= 1;
            }
        }
    } 

    /// adds a vertex u to the clique
    fn add_vertex(&mut self, u:VertexId) {
        self.configuration.clear();
        for v in self.inst.neighbors(u) {
            self.configuration.insert(v);
        }
        // remove non-neighbors of u from the clique
        let clique_vec:Vec<VertexId> = self.inside_clique.iter().collect();
        for v in clique_vec {
            if !self.inst.are_adjacent(u, v) {
                self.remove_vertex(v);
            }
        }
        // insert u & update total weight
        // self.increase_weight(u);
        self.inside_clique.insert(u);
        let u_weight = self.get_weight(u);
        self.total_weight += u_weight;
        // for each vertex non-adjacent to u, increase its weight cost
        for v in self.inst.vertices().filter(|v| *v!=u) {
            if !self.inst.are_adjacent(u, v) {
                self.weight_cost_inserting[v] += u_weight;
                self.nb_non_adj_clique[v] += 1;
            }
        }
        // if improving the current-best-known solution, update it
        if self.inside_clique.len() > self.current_sol.len() {
            self.current_sol = self.inside_clique.iter().collect();
        }
        self.nb_iter += 1;
    }

    // fn check_weights(&self) {
    //     let n = self.inst.nb_vertices();
    //     // check total weight
    //     let mut total_weight = 0;
    //     // check weight_cost_inserting
    //     let mut weight_cost_inserting = vec![0 ; n];
    //     for u in self.inside_clique.iter() {
    //         total_weight += self.get_weight(u);
    //         for v in self.inst.vertices() {
    //             if u!=v && !self.inst.are_adjacent(u, v) {
    //                 weight_cost_inserting[v] += self.get_weight(u);
    //             }
    //         }
    //     }
    //     assert_eq!(total_weight, self.total_weight);
    //     for (i,v) in weight_cost_inserting.iter().enumerate() {
    //         assert_eq!(*v, self.weight_cost_inserting[i], "vertex {}", i);
    //     }
    // }

    /// applies a move (coloring a vertex with a color)
    fn commit(&mut self, node:&Node) {
        // println!("{:?}", node);
        // self.check_weights();
        match node.vertex_in {
            None => {},
            Some(v) => {
                self.add_vertex(v);
                self.tabu.insert(&0, v);
                self.tabu.increase_iter();
            }
        };
        // self.check_weights();
    }

    /// get the learned weight of an edge
    fn get_weight(&self, u:VertexId) -> Weight { self.weights[u] }

    // /// increase the learned weight of an edge
    // fn increase_weight(&mut self, u:VertexId) {
    //     self.weights[u] += 1;
    // }
}

impl GuidedSpace<Node, i64> for PartialWeightingLocalSearch {
    fn guide(&mut self, node: &Node) -> i64 { -(node.total_weight as i64) }
}

impl ToSolution<Node, Vec<Vec<VertexId>>> for PartialWeightingLocalSearch {
    fn solution(&mut self, _: &mut Node) -> Vec<Vec<VertexId>> {
        clique_vec_to_vecvec(&self.current_sol, self.inst.nb_vertices())
    }
}

impl SearchSpace<Node, i32> for PartialWeightingLocalSearch {
    fn initial(&mut self) -> Node {
        Node {
            vertex_in: None,
            total_weight: self.total_weight,
        }
    }
    fn bound(&mut self, _node: &Node) -> i32 { -(self.current_sol.len() as i32) }
    fn goal(&mut self, _n: &Node) -> bool { true } // every node is a feasible solution
    fn g_cost(&mut self, _n: &Node) -> i32 { 0 }
}

impl TotalNeighborGeneration<Node> for PartialWeightingLocalSearch {
    fn neighbors(&mut self, node: &mut Node) -> Vec<Node> {
        self.commit(node);
        // iterate over vertices that are not inside the clique, and try to add them
        let mut res:Vec<Node> = Vec::new();
        for u in self.configuration.iter() {
            // if !self.inside_clique.contains(u) {
            if !self.inside_clique.contains(u)
            && self.inst.degree(u) >= self.current_sol.len()
            && (res.is_empty() || !self.tabu.contains(&(node.total_weight as usize), &u)) {
                let u_node = Node {
                    vertex_in: Some(u),
                    total_weight:self.total_weight + self.get_weight(u)
                        - self.weight_cost_inserting[u]
                };
                if res.is_empty() {
                    res.push(u_node);
                } else if u_node.total_weight > res[0].total_weight {
                    res.clear();
                    res.push(u_node);
                } else if res[0].total_weight == u_node.total_weight { res.push(u_node); }
            }
        }
        res
    }
}


/** performs a partial weighting local search. */
pub fn clique_partial_weighting<Stopping:StoppingCriterion>(
    inst:Rc<dyn ColoringInstance>,
    sol:&[VertexId],
    perf_filename:Option<String>,
    sol_filename:Option<String>,
    stop:Stopping
) -> Vec<Vec<VertexId>> {
    let mut solution:Vec<Vec<VertexId>> = clique_vec_to_vecvec(sol, inst.nb_vertices());
    let logger = Rc::new(MetricLogger::default());
    let space = Rc::new(RefCell::new(
        StatTsCombinator::new(
            PartialWeightingLocalSearch::initialize(inst.clone(), &solution[0]),
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
        false,
    );
    solution
}


#[cfg(test)]
mod tests {
    use super::*;

    use dogs::search_algorithm::TimeStoppingCriterion;
    
    use crate::{cgshop::CGSHOPInstance, search::greedy_clique::greedy_clique};

    #[test]
    fn test_cwls() {
        let inst = Rc::new(CGSHOPInstance::from_file(
            // "./insts/cgshop22/vispecn2518.instance.json"
            // "./insts/cgshop22/vispecn13806.instance.json"
            "./insts/cgshop22/vispecn50715.instance.json"
            // "./insts/cgshop22/rvispecn6048.instance.json"
            // "./insts/cgshop22/rvispecn17968.instance.json"
            // "./insts/cgshop22/reecn3382.instance.json"
            // "./insts/cgshop22/visp26405.instance.json"
            // "./insts/cgshop22/rvisp3499.instance.json"
            // "./insts/cgshop22/reecn9674.instance.json"
            // "./insts/cgshop22/reecn12588.instance.json"
            // "./insts/cgshop22/reecn25913.instance.json"
            // "./insts/cgshop22/reecn31126.instance.json"
            // "./insts/cgshop22/reecn58325.instance.json"
            // "./insts/cgshop22/reecn73116.instance.json"
            // "./insts/cgshop22/sqrp12451.instance.json"
            // "./insts/cgshop_22_examples/visp_5K.instance.json"
            // "./insts/cgshop_22_examples/visp_50K.instance.json"
            // "./insts/cgshop_22_examples/visp_100K.instance.json"
            // "./insts/cgshop_22_examples/sqrm_50K_6.instance.json"
        ));
        let greedy_sol = greedy_clique(inst.clone());
        println!("initial solution: {}", greedy_sol.len());
        let stopping_criterion:TimeStoppingCriterion = TimeStoppingCriterion::new(36000.);
        let sol_ls = clique_partial_weighting(
            inst, &greedy_sol, None, None , stopping_criterion
        );
        println!("after ls: {}", sol_ls.len());
    }

    #[test]
    fn test_cwls2() {
        let inst = Rc::new(CGSHOPInstance::from_file(
            // "./insts/cgshop22/vispecn2518.instance.json"
            // "./insts/cgshop22/vispecn5478.instance.json"
            // "./insts/cgshop22/rvispecn6048.instance.json"
            // "./insts/cgshop22/rvispecn17968.instance.json"
            // "./insts/cgshop22/reecn3382.instance.json"
            // "./insts/cgshop22/visp26405.instance.json"
            // "./insts/cgshop22/visp31334.instance.json"
            "./insts/cgshop22/visp50129.instance.json"
            // "./insts/cgshop22/rvisp3499.instance.json"
            // "./insts/cgshop22/reecn9674.instance.json"
            // "./insts/cgshop22/reecn12588.instance.json"
            // "./insts/cgshop22/reecn31126.instance.json"
            // "./insts/cgshop22/reecn73116.instance.json"
            // "./insts/cgshop22/sqrp12451.instance.json"
            // "./insts/cgshop22/rsqrp23406.instance.json"
            // "./insts/cgshop22/rsqrp24641.instance.json"
            // "./insts/cgshop_22_examples/visp_5K.instance.json"
            // "./insts/cgshop_22_examples/sqrm_50K_1.instance.json"
        ));
        let greedy_sol = greedy_clique(inst.clone());
        println!("initial solution: {}", greedy_sol.len());
        let stopping_criterion:TimeStoppingCriterion = TimeStoppingCriterion::new(3600.);
        let sol_ls = clique_partial_weighting(
            inst, &greedy_sol, None, None , stopping_criterion
        );
        println!("after ls: {}", sol_ls.len());
    }

    #[test]
    fn test_cwls3() {
        let inst = Rc::new(CGSHOPInstance::from_file(
            // "./insts/cgshop22/vispecn2518.instance.json"
            // "./insts/cgshop22/vispecn17665.instance.json"
            // "./insts/cgshop22/rvispecn6048.instance.json"
            // "./insts/cgshop22/rvispecn17968.instance.json"
            // "./insts/cgshop22/reecn3382.instance.json"
            // "./insts/cgshop22/visp26405.instance.json"
            // "./insts/cgshop22/visp31334.instance.json"
            // "./insts/cgshop22/visp48558.instance.json"
            // "./insts/cgshop22/rvisp3499.instance.json"
            // "./insts/cgshop22/rvisp14562.instance.json"
            // "./insts/cgshop22/rvisp24116.instance.json"
            // "./insts/cgshop22/reecn9674.instance.json"
            // "./insts/cgshop22/reecn12588.instance.json"
            // "./insts/cgshop22/reecn31126.instance.json"
            // "./insts/cgshop22/reecn73116.instance.json"
            "./insts/cgshop22/sqrpecn71571.instance.json"
            // "./insts/cgshop22/sqrp12451.instance.json"
            // "./insts/cgshop_22_examples/visp_5K.instance.json"
            // "./insts/cgshop_22_examples/sqrm_10K_5.instance.json"
        ));
        let greedy_sol = greedy_clique(inst.clone());
        println!("initial solution: {}", greedy_sol.len());
        let stopping_criterion:TimeStoppingCriterion = TimeStoppingCriterion::new(3600.);
        let sol_ls = clique_partial_weighting(
            inst, &greedy_sol, None, None , stopping_criterion
        );
        println!("after ls: {}", sol_ls.len());
    }


    #[test]
    fn test_cwls4() {
        let inst = Rc::new(CGSHOPInstance::from_file(
            // "./insts/cgshop22/vispecn2518.instance.json"
            // "./insts/cgshop22/rvispecn6048.instance.json"
            // "./insts/cgshop22/rvispecn17968.instance.json"
            // "./insts/cgshop22/reecn3382.instance.json"
            // "./insts/cgshop22/visp26405.instance.json"
            // "./insts/cgshop22/visp31334.instance.json"
            // "./insts/cgshop22/rvisp3499.instance.json"
            // "./insts/cgshop22/reecn9674.instance.json"
            // "./insts/cgshop22/reecn12588.instance.json"
            // "./insts/cgshop22/reecn31126.instance.json"
            // "./insts/cgshop22/reecn73116.instance.json"
            // "./insts/cgshop22/sqrp12451.instance.json"
            // "./insts/cgshop22/sqrp20166.instance.json"
            "./insts/cgshop22/sqrp69435.instance.json"
            // "./insts/cgshop_22_examples/visp_5K.instance.json"
            // "./insts/cgshop_22_examples/sqrm_10K_5.instance.json"
        ));
        let greedy_sol = greedy_clique(inst.clone());
        println!("initial solution: {}", greedy_sol.len());
        let stopping_criterion:TimeStoppingCriterion = TimeStoppingCriterion::new(3600.);
        let sol_ls = clique_partial_weighting(
            inst, &greedy_sol, None, None , stopping_criterion
        );
        println!("after ls: {}", sol_ls.len());
    }

    #[test]
    fn test_cwls5() {
        let inst = Rc::new(CGSHOPInstance::from_file(
            // "./insts/cgshop22/vispecn2518.instance.json"
            // "./insts/cgshop22/rvispecn6048.instance.json"
            // "./insts/cgshop22/rvispecn17968.instance.json"
            // "./insts/cgshop22/reecn3382.instance.json"
            // "./insts/cgshop22/visp26405.instance.json"
            // "./insts/cgshop22/visp31334.instance.json"
            // "./insts/cgshop22/rvisp3499.instance.json"
            // "./insts/cgshop22/reecn9674.instance.json"
            // "./insts/cgshop22/reecn12588.instance.json"
            // "./insts/cgshop22/reecn31126.instance.json"
            // "./insts/cgshop22/reecn73116.instance.json"
            "./insts/cgshop22/reecn58325.instance.json"
            // "./insts/cgshop22/sqrp12451.instance.json"
            // "./insts/cgshop22/sqrp20166.instance.json"
            // "./insts/cgshop22/sqrpecn18520.instance.json"
            // "./insts/cgshop_22_examples/visp_5K.instance.json"
            // "./insts/cgshop_22_examples/sqrm_10K_5.instance.json"
        ));
        let greedy_sol = greedy_clique(inst.clone());
        println!("initial solution: {}", greedy_sol.len());
        let stopping_criterion:TimeStoppingCriterion = TimeStoppingCriterion::new(3600.);
        let sol_ls = clique_partial_weighting(
            inst, &greedy_sol, None, None , stopping_criterion
        );
        println!("after ls: {}", sol_ls.len());
    }

    #[test]
    fn test_cwls6() {
        let inst = Rc::new(CGSHOPInstance::from_file(
            // "./insts/cgshop22/vispecn2518.instance.json"
            // "./insts/cgshop22/vispecn26914.instance.json"
            // "./insts/cgshop22/rvispecn6048.instance.json"
            // "./insts/cgshop22/rvispecn17968.instance.json"
            // "./insts/cgshop22/reecn3382.instance.json"
            // "./insts/cgshop22/visp26405.instance.json"
            // "./insts/cgshop22/visp31334.instance.json"
            // "./insts/cgshop22/rvisp3499.instance.json"
            // "./insts/cgshop22/reecn9674.instance.json"
            // "./insts/cgshop22/reecn12588.instance.json"
            // "./insts/cgshop22/reecn31126.instance.json"
            // "./insts/cgshop22/reecn73116.instance.json"
            // "./insts/cgshop22/sqrp12451.instance.json"
            // "./insts/cgshop22/sqrp20166.instance.json"
            "./insts/cgshop22/sqrp69435.instance.json"
            // "./insts/cgshop22/sqrpecn18520.instance.json"
            // "./insts/cgshop_22_examples/visp_5K.instance.json"
            // "./insts/cgshop_22_examples/sqrm_10K_5.instance.json"
        ));
        let greedy_sol = greedy_clique(inst.clone());
        println!("initial solution: {}", greedy_sol.len());
        let stopping_criterion:TimeStoppingCriterion = TimeStoppingCriterion::new(3600.);
        let sol_ls = clique_partial_weighting(
            inst, &greedy_sol, None, None , stopping_criterion
        );
        println!("after ls: {}", sol_ls.len());
    }

    #[test]
    fn test_cwls8() {
        let inst = Rc::new(CGSHOPInstance::from_file(
            // "./insts/cgshop22/vispecn2518.instance.json"
            // "./insts/cgshop22/vispecn10178.instance.json"
            // "./insts/cgshop22/vispecn33280.instance.json"
            // "./insts/cgshop22/vispecn26914.instance.json"
            // "./insts/cgshop22/rvispecn6048.instance.json"
            // "./insts/cgshop22/rvispecn17968.instance.json"
            // "./insts/cgshop22/reecn3382.instance.json"
            // "./insts/cgshop22/visp26405.instance.json"
            // "./insts/cgshop22/visp31334.instance.json"
            // "./insts/cgshop22/rvisp3499.instance.json"
            // "./insts/cgshop22/reecn9674.instance.json"
            // "./insts/cgshop22/reecn12588.instance.json"
            // "./insts/cgshop22/reecn31126.instance.json"
            // "./insts/cgshop22/reecn73116.instance.json"
            // "./insts/cgshop22/sqrp12451.instance.json"
            // "./insts/cgshop22/sqrp20166.instance.json"
            // "./insts/cgshop22/sqrpecn18520.instance.json"
            "./insts/cgshop22/sqrpecn71571.instance.json"
            // "./insts/cgshop_22_examples/visp_5K.instance.json"
            // "./insts/cgshop_22_examples/sqrm_10K_5.instance.json"
        ));
        let greedy_sol = greedy_clique(inst.clone());
        println!("initial solution: {}", greedy_sol.len());
        let stopping_criterion:TimeStoppingCriterion = TimeStoppingCriterion::new(3600.);
        let sol_ls = clique_partial_weighting(
            inst, &greedy_sol, None, None , stopping_criterion
        );
        println!("after ls: {}", sol_ls.len());
    }

    #[test]
    fn test_cwls9() {
        let inst = Rc::new(CGSHOPInstance::from_file(
            // "./insts/cgshop22/vispecn2518.instance.json"
            // "./insts/cgshop22/vispecn10178.instance.json"
            // "./insts/cgshop22/vispecn15912.instance.json"
            // "./insts/cgshop22/vispecn33280.instance.json"
            // "./insts/cgshop22/vispecn26914.instance.json"
            // "./insts/cgshop22/rvispecn6048.instance.json"
            // "./insts/cgshop22/rvispecn17968.instance.json"
            // "./insts/cgshop22/reecn3382.instance.json"
            // "./insts/cgshop22/visp26405.instance.json"
            "./insts/cgshop22/visp66498.instance.json"
            // "./insts/cgshop22/rvisp14562.instance.json"
            // "./insts/cgshop22/visp31334.instance.json"
            // "./insts/cgshop22/rvisp3499.instance.json"
            // "./insts/cgshop22/reecn9674.instance.json"
            // "./insts/cgshop22/reecn12588.instance.json"
            // "./insts/cgshop22/reecn31126.instance.json"
            // "./insts/cgshop22/reecn73116.instance.json"
            // "./insts/cgshop22/sqrp12451.instance.json"
            // "./insts/cgshop22/sqrp20166.instance.json"
            // "./insts/cgshop22/sqrpecn18520.instance.json"
            // "./insts/cgshop_22_examples/visp_5K.instance.json"
            // "./insts/cgshop_22_examples/sqrm_10K_5.instance.json"
        ));
        let greedy_sol = greedy_clique(inst.clone());
        println!("initial solution: {}", greedy_sol.len());
        let stopping_criterion:TimeStoppingCriterion = TimeStoppingCriterion::new(3600.);
        let sol_ls = clique_partial_weighting(
            inst, &greedy_sol, None, None , stopping_criterion
        );
        println!("after ls: {}", sol_ls.len());
    }

    #[test]
    fn test_cwls10() {
        let inst = Rc::new(CGSHOPInstance::from_file(
            // "./insts/cgshop22/vispecn2518.instance.json"
            // "./insts/cgshop22/vispecn10178.instance.json"
            "./insts/cgshop22/vispecn50715.instance.json"
            // "./insts/cgshop22/vispecn26914.instance.json"
            // "./insts/cgshop22/rvispecn6048.instance.json"
            // "./insts/cgshop22/rvispecn17968.instance.json"
            // "./insts/cgshop22/reecn3382.instance.json"
            // "./insts/cgshop22/reecn25913.instance.json"
            // "./insts/cgshop22/visp26405.instance.json"
            // "./insts/cgshop22/visp31334.instance.json"
            // "./insts/cgshop22/rvisp3499.instance.json"
            // "./insts/cgshop22/reecn9674.instance.json"
            // "./insts/cgshop22/reecn12588.instance.json"
            // "./insts/cgshop22/reecn31126.instance.json"
            // "./insts/cgshop22/reecn73116.instance.json"
            // "./insts/cgshop22/sqrp12451.instance.json"
            // "./insts/cgshop22/sqrp20166.instance.json"
            // "./insts/cgshop22/sqrpecn18520.instance.json"
            // "./insts/cgshop_22_examples/visp_5K.instance.json"
            // "./insts/cgshop_22_examples/sqrm_10K_5.instance.json"
        ));
        let greedy_sol = greedy_clique(inst.clone());
        println!("initial solution: {}", greedy_sol.len());
        let stopping_criterion:TimeStoppingCriterion = TimeStoppingCriterion::new(3600.);
        let sol_ls = clique_partial_weighting(
            inst, &greedy_sol, None, None , stopping_criterion
        );
        println!("after ls: {}", sol_ls.len());
    }

}
