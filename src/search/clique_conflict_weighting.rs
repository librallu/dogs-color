use std::{cell::RefCell, rc::Rc};

use bit_set::BitSet;

use dogs::{
    combinators::stats::StatTsCombinator,
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

type Weight = u32;

/// models a decision within the local search.
#[derive(Debug,Clone,Eq,PartialEq,Hash)]
struct Node {
    pub vertex_in:VertexId, // vertex to include
    pub vertex_out:VertexId, // vertex to exclude
    pub total_weight:Weight, // total Weight associated with the decision
}


/** implements a conflict weighting local search */
#[derive(Debug)]
struct ConflictWeightingLocalSearch {
    /// instance object
    inst:Rc<dyn ColoringInstance>,
    /// weights[u][v]: weight learned for the edge (u,v)
    weights:Vec<Vec<Weight>>,
    /// current best feasible solution 
    current_sol:Vec<VertexId>,
    /// inside_clique[v] = true iff v is in the current "clique"
    inside_clique:BitSet,
    /// nb_adj_clique[v] = number of vertices in the clique that v are adjacent to
    nb_adj_clique:Vec<usize>,
    /// weight_adj_clique[v] = total weight of edges absent with clique vertices
    weight_adj_clique:Vec<Weight>,
    /// total weight of the candidate clique
    total_weight:Weight,
}

impl ConflictWeightingLocalSearch {

    /// initializes the data-structure from an initial solution 
    fn initialize(inst:Rc<dyn ColoringInstance>, sol:&[VertexId]) -> Self {
        // build data-structures
        let n = inst.nb_vertices();
        let mut inside_clique = BitSet::with_capacity(n);
        for v in sol {
            inside_clique.insert(*v);
        }
        let mut nb_adj_clique = vec![0 ; n];
        let mut weight_adj_clique = vec![0 ; n];
        for u in sol {
            for v in inst.vertices().filter(|v| v!=u) {
                if inst.are_adjacent(*u, v) {
                    nb_adj_clique[v] += 1;
                } else {
                    weight_adj_clique[v] += 1;
                }
            }
        }
        Self {
            inst,
            weights: (0..n).map(|i| vec![1 ; i]).collect(),
            current_sol: sol.to_vec(),
            inside_clique,
            nb_adj_clique,
            weight_adj_clique,
            total_weight:0,
        }
    }

    /// adds a vertex to the clique
    fn add_vertex(&mut self, v:VertexId, increase_weights:bool) {
        self.inside_clique.insert(v);
        for w in self.inst.vertices().filter(|w| *w!=v) {
            if self.inst.are_adjacent(v, w) {
                self.nb_adj_clique[w] += 1;
            } else { // if not neighbors, update the weight of w
                self.weight_adj_clique[w] += self.get_weight(v, w);
                if self.inside_clique.contains(w) {
                    if increase_weights {
                        self.increase_weight(v, w);
                    }
                    self.total_weight += self.get_weight(v, w);
                }
            }
        }
    }

    /// removes a vertex from the clique
    fn remove_vertex(&mut self, v:VertexId) {
        self.inside_clique.remove(v);
        for w in self.inst.vertices() {
            if self.inst.are_adjacent(v, w) {
                self.nb_adj_clique[w] -= 1;
                self.weight_adj_clique[w] -= self.get_weight(v, w);
            } else if self.inside_clique.contains(w) {
                self.total_weight -= self.get_weight(v, w);
            }
        }
    }

    /// add the vertex that has the maximum degree within the clique (break ties by degree)
    fn insert_new_vertex(&mut self) {
        loop { // repeat until the new solution is infeasible
            // select v
            let v = self.inst.vertices()
                .filter(|v| self.inside_clique.contains(*v))
                .max_by(|a,b| {
                    self.nb_adj_clique[*a].cmp(&self.nb_adj_clique[*b])
                    .then_with(|| self.inst.degree(*a).cmp(&self.inst.degree(*b)))
                }).unwrap();
            // add v to the candidate clique & update data-structures
            self.add_vertex(v, false);
            // if the new solution is infeasible, stop, otherwise, update the best-known solution
            if self.nb_adj_clique[v] == self.current_sol.len() {
                self.current_sol.push(v);
            } else {
                break;
            }
        }
    }

    /// applies a move (coloring a vertex with a color)
    fn commit(&mut self, node:&Node) {
        self.add_vertex(node.vertex_in, true);
        self.remove_vertex(node.vertex_out);
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
}

impl GuidedSpace<Node, i64> for ConflictWeightingLocalSearch {
    fn guide(&mut self, node: &Node) -> i64 {
        node.total_weight as i64
    }
}

impl ToSolution<Node, Vec<VertexId>> for ConflictWeightingLocalSearch {
    fn solution(&mut self, _: &mut Node) -> Vec<VertexId> {
        self.current_sol.clone()
    }
}

impl SearchSpace<Node, i32> for ConflictWeightingLocalSearch {
    fn initial(&mut self) -> Node {
        Node {
            vertex_in: 0,
            vertex_out: 0,
            total_weight: 0,
        }
    }
    fn bound(&mut self, _node: &Node) -> i32 { -(self.current_sol.len() as i32) }
    fn goal(&mut self, n: &Node) -> bool { n.total_weight == 0 }
    fn g_cost(&mut self, _n: &Node) -> i32 { 0 }
}

impl TotalNeighborGeneration<Node> for ConflictWeightingLocalSearch {
    fn neighbors(&mut self, node: &mut Node) -> Vec<Node> {
        if node.vertex_in != node.vertex_out { // if not a dummy decision, commit it
            self.commit(node);
        }
        if self.goal(node) { // if no conflict, merge some colors
            self.insert_new_vertex();
        }
        // select the node with the largest weight inside the clique
        let u = self.inside_clique.iter().max_by(|u,v| {
            self.weight_adj_clique[*u].cmp(&self.weight_adj_clique[*v])
                .then_with(|| self.inst.degree(*u).cmp(&self.inst.degree(*v)))
        }).unwrap();
        // select the node with the smallest weight outside the clique
        let v = self.inst.vertices().filter(|v| !self.inside_clique.contains(*v))
            .min_by(|u,v| {
                self.weight_adj_clique[*u].cmp(&self.weight_adj_clique[*v])
                    .then_with(|| self.inst.degree(*u).cmp(&self.inst.degree(*v)))
            }).unwrap();
        // return the swap
        vec![Node {
            vertex_in:v,
            vertex_out:u,
            total_weight:self.total_weight - self.weight_adj_clique[u] + self.weight_adj_clique[v]
        }]
    }
}


/** performs a conflict weighting local search. */
pub fn clique_conflict_weighting<Stopping:StoppingCriterion>(
inst:Rc<dyn ColoringInstance>,
sol:&[VertexId],
perf_filename:Option<String>,
sol_filename:Option<String>,
stop:Stopping
) -> Vec<VertexId> {
    let mut solution:Vec<VertexId> = sol.to_vec();
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
            assert_eq!(node.total_weight, 0);
            solution = space.borrow_mut().solution(&mut node.clone());
        }  
    }
    let mut stats = serde_json::Value::default();
    space.borrow_mut().json_statistics(&mut stats);
    export_results(
        inst,
        &[solution.clone()],
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
    
    use crate::{cgshop::CGSHOPInstance, search::clique_bnb::greedy_clique};

    #[test]
    fn test_cwls() {
        let inst = Rc::new(CGSHOPInstance::from_file(
            "./insts/cgshop22/vispecn2518.instance.json"
            // "./insts/cgshop22/reecn3382.instance.json"
            // "./insts/cgshop22/rvisp3499.instance.json"
            // "./insts/cgshop22/reecn9674.instance.json"
            // "./insts/cgshop22/reecn12588.instance.json"
            // "./insts/cgshop22/reecn31126.instance.json"
            // "./insts/cgshop22/reecn73116.instance.json"
            // "./insts/cgshop22/rvispecn17968.instance.json"
            // "./insts/cgshop_22_examples/visp_5K.instance.json"
            // "./insts/cgshop_22_examples/sqrm_10K_5.instance.json"
        ));
        let greedy_sol = greedy_clique(inst.clone());
        println!("initial solution: {}", greedy_sol.len());
        let stopping_criterion:TimeStoppingCriterion = TimeStoppingCriterion::new(30.);
        clique_conflict_weighting(
            inst, &greedy_sol, None, None , stopping_criterion
        );
    }

}
