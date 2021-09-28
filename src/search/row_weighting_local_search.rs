use std::{rc::Rc, time::Instant};

use fastrand::Rng;

use dogs::{
    combinators::helper::tabu_tenure::TabuTenure,
    data_structures::sparse_set::SparseSet,
    search_algorithm::StoppingCriterion
};

use crate::color::{ColoringInstance, VertexId};

type Penalty = i64;

/// models a decision within the local search.
#[derive(Debug,Clone,Eq,PartialEq,Hash)]
struct Node {
    pub vertex:VertexId, // vertex to change
    pub previous_color:usize, // previous color of vertex v
    pub next_color:usize, // next color of vertex v
    pub total_penalties:Penalty, // total penalty associated with the decision
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
        // self.nb_iter += 1;
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




pub fn row_weighting_local_search<Stopping:StoppingCriterion>(inst:Rc<dyn ColoringInstance>, sol:&[Vec<VertexId>], stopping_criterion:Stopping, solution_filename:Option<String>, perf_filename:Option<String>) -> Vec<Vec<VertexId>> {
    let n = inst.nb_vertices();
    // penalties learned for each edge. It indicates how difficult 
    let mut penalties:Vec<Vec<Penalty>> = vec![ vec![1 ; n] ; n];
    // let mut tabu = TabuColTenure::new(n/35, 0.5, n, sol.len());
    // random number generator
    // let rng:Rng = Rng::new();
    let mut current_sol = sol.to_vec();
    // perform iterations
    let mut nb_iter:i64 = 0;
    while !stopping_criterion.is_finished() {
        // invariant: current_sol is feasible
        let mut new_color_id;
        loop {
            // find the best color merge (couple of colors that minimize the conflict penalties)
            println!(
                "it: {:<15} colors: {:<15} conflicts: {:<15} penalties: {:<15}",
                nb_iter, current_sol.len()+1, 0, 0
            );
            let time_start_merging = Instant::now();
            let mut best_penalties:Penalty = Penalty::MAX;
            let mut current_best_color_merge:Option<(VertexId,VertexId)> = None;
            for (i1, color1) in current_sol.iter().enumerate() {
                for (i2, color2) in current_sol.iter().enumerate() {
                    if i2 < i1 {
                        let mut current_penalties:Penalty = 0;
                        for u in color1.iter() {
                            for v in color2.iter() {
                                if inst.are_adjacent(*u, *v) {
                                    current_penalties += penalties[*u][*v];
                                    if current_penalties >= best_penalties {
                                        break;
                                    }
                                }
                            }
                            if current_penalties >= best_penalties {
                                break;
                            }
                        }
                        if current_penalties < best_penalties {
                            best_penalties = current_penalties;
                            current_best_color_merge = Some((i1,i2));
                        }
                    }
                }
            }
            // perform the merge
            println!(
                "merging {:?}, with {} penalties ({:.3} seconds)",
                current_best_color_merge, best_penalties, time_start_merging.elapsed().as_secs_f32()
            );
            let (c1,c2) = current_best_color_merge.unwrap();
            let mut new_color = Vec::new();
            let c_min = std::cmp::min(c1, c2);
            let c_max = std::cmp::max(c1, c2);
            new_color.append(&mut current_sol.remove(c_max));
            new_color.append(&mut current_sol.remove(c_min));
            new_color_id = current_sol.len();
            current_sol.push(new_color);
            // exit only if there are some conflicting nodes
            if best_penalties > 0 { break; }
        }
        // perform the local search (at this point, there are some conflicts)
        // 1. initialize data-structures
        let nb_colors = current_sol.len();
        // colors[v]: color of vertex v
        let mut colors = vec![ 0 ; n];
        for (i,color) in current_sol.iter().enumerate() {
            for v in color {
                colors[*v] = i;
            }
        }
        // penalties_neigh_colors[v][c]: penalties of neighbors of v that are assigned color c
        let mut penalties_neigh_colors:Vec<Vec<Penalty>> = vec![ vec![0 ; nb_colors] ; n ];
        for u in inst.vertices() {
            for v in inst.neighbors(u) {
                penalties_neigh_colors[v][colors[u]] += penalties[u][v];
            }
        }
        // conflicting_vertices: list of vertices that have some conflict
        let mut conflicting_vertices:SparseSet = SparseSet::new(n);
        // for each vertex, the number of conflicts
        let mut vertex_nb_conflicts:Vec<i64> = vec![0 ; n];
        // number of conflicting edges & total penalty
        let mut nb_conflicting_edges:i64 = 0;
        let mut total_penalty:Penalty = 0;
        for u in current_sol[new_color_id].iter() {
            for v in current_sol[new_color_id].iter() {
                if u < v && inst.are_adjacent(*u,*v) {
                    conflicting_vertices.insert(*u);
                    conflicting_vertices.insert(*v);
                    vertex_nb_conflicts[*u] += 1;
                    vertex_nb_conflicts[*v] += 1;
                    nb_conflicting_edges += 1;
                    total_penalty += penalties[*u][*v];

                }
            }
        }
        // 2. perform local search iterations (for each conflicting vertex, find the best penalty)
        while nb_conflicting_edges > 0 && !stopping_criterion.is_finished() {
            assert!(nb_conflicting_edges > 0);
            nb_iter += 1;
            if nb_iter % 10000 == 0 {
                println!(
                    "it: {:<15} colors: {:<15} conflicts: {:<15} penalties: {:<15}",
                    nb_iter, current_sol.len()+1, nb_conflicting_edges, total_penalty
                );
            }
            // 2.1 find the best move
            let mut best_node:Node = Node {
                vertex:0, previous_color:0, next_color:0, total_penalties:Penalty::MAX, nb_conflicts:0
            };
            let mut i = 0;
            // println!("conflicting vertices: {:?}", conflicting_vertices.iter().collect::<Vec<VertexId>>());
            while i < conflicting_vertices.len() { // iterate over conflicting vertices
                let u = conflicting_vertices.nth(i);
                if vertex_nb_conflicts[u] > 0 { // u has indeed some conflicts
                    // for each vertex, try changing its color
                    for c in (0..nb_colors).filter(|c| *c != colors[u]) {
                        let current_penalties:Penalty = total_penalty +
                            penalties_neigh_colors[u][c] - penalties_neigh_colors[u][colors[u]];
                        if current_penalties < best_node.total_penalties { // find the best node
                            let current_node = Node {
                                vertex:u,
                                previous_color:colors[u],
                                next_color:c,
                                total_penalties:current_penalties,
                                nb_conflicts: nb_conflicting_edges
                            };
                            // if !tabu.contains(&current_node, &current_node) {
                                best_node = current_node; 
                            // }
                        }
                    }
                    i += 1;
                } else {
                    conflicting_vertices.remove(u); // update conflicting_vertices if it has no conflict
                }
            }
            // 2.2. apply move
            // println!("applying move {:?}", best_node);
            colors[best_node.vertex] = best_node.next_color; // change colors
            total_penalty = best_node.total_penalties; // update total penalty
            // mark the move tabu
            // tabu.insert(&best_node, best_node.clone()); // make the decision tabu
            // tabu.increment_iter();
            // update penalties_neigh_colors & vertex_nb_conflicts
            // println!("{:?}", best_node);
            for neigh in inst.neighbors(best_node.vertex) {
                assert!(best_node.previous_color != best_node.next_color);
                assert!(neigh != best_node.vertex);
                penalties_neigh_colors[neigh][best_node.previous_color] -= penalties[neigh][best_node.vertex];
                penalties_neigh_colors[neigh][best_node.next_color] += penalties[neigh][best_node.vertex];
                if colors[neigh] == best_node.previous_color { // remove conflict
                    vertex_nb_conflicts[neigh] -= 1;
                    vertex_nb_conflicts[best_node.vertex] -= 1;
                    nb_conflicting_edges -= 1;
                    // println!("out conflict: {} {}", neigh, best_node.vertex);
                }
                if colors[neigh] == best_node.next_color { // add conflict
                    vertex_nb_conflicts[neigh] += 1;
                    vertex_nb_conflicts[best_node.vertex] += 1;
                    nb_conflicting_edges += 1;
                    // update penalties when conflicts enter
                    penalties[neigh][best_node.vertex] += 1;
                    penalties[best_node.vertex][neigh] += 1;
                    penalties_neigh_colors[neigh][best_node.next_color] += 1;
                    penalties_neigh_colors[best_node.vertex][best_node.next_color] += 1;
                    conflicting_vertices.insert(neigh);
                    conflicting_vertices.insert(best_node.vertex);
                    total_penalty += 1;
                    // println!("in conflict: {} {}", neigh, best_node.vertex);
                }
            }
            // // TEST debug: display conflicts
            // println!("{} conflicts", nb_conflicting_edges);
            // for u in conflicting_vertices.iter() {
            //     for v in inst.neighbors(u) {
            //         if u < v && colors[u] == colors[v] {
            //             println!("{} {} conflict", u, v);
            //         }
            //     }
            // }
            // println!("===");
            // if nb_iter >= 2 {
            //     todo!();
            // }
        }
        if nb_conflicting_edges == 0 {
            // TODO check solution
            println!("need check feasible");
        }
    }
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    use dogs::search_algorithm::TimeStoppingCriterion;
    
    use crate::{
        cgshop::CGSHOPInstance,
        search::greedy_dsatur::greedy_dsatur
    };


    #[test]
    fn test_rwls() {
        let inst = Rc::new(CGSHOPInstance::from_file(
            "./insts/cgshop22/reecn3382.instance.json"
            // "./insts/cgshop22/reecn9674.instance.json"
            // "./insts/cgshop22/reecn12588.instance.json"
            // "./insts/cgshop22/reecn31126.instance.json"
            // "./insts/cgshop22/reecn73116.instance.json"
        ));
        let greedy_sol = greedy_dsatur(inst.clone(), false);
        println!("initial solution: {}", greedy_sol.len());
        let stopping_criterion:TimeStoppingCriterion = TimeStoppingCriterion::new(3000.);
        row_weighting_local_search(inst, &greedy_sol, stopping_criterion, None, None);
    }
}
