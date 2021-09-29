use std::{rc::Rc, time::Instant};

use bit_set::BitSet;
use fastrand::Rng;

use dogs::{
    combinators::helper::tabu_tenure::TabuTenure,
    data_structures::sparse_set::SparseSet,
    search_algorithm::StoppingCriterion
};

use crate::color::{CheckerResult, ColoringInstance, VertexId, checker};

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



pub fn row_weighting_local_search<Stopping:StoppingCriterion>(inst:Rc<dyn ColoringInstance>, sol:&[Vec<VertexId>], stopping_criterion:Stopping, solution_filename:Option<String>, perf_filename:Option<String>, time_init:f32) -> Vec<Vec<VertexId>> {
    let time_ls_init = Instant::now();
    let n = inst.nb_vertices();
    // penalties learned for each edge. It indicates how difficult 
    let mut penalties:Vec<Vec<Penalty>> = vec![ vec![1 ; n] ; n];
    let mut tabu = TabuColTenure::new(10, 0.6, n, sol.len());
    // random number generator
    // let rng:Rng = Rng::new();
    let mut current_sol = sol.to_vec();
    let mut best_sol = current_sol.clone();
    // perform iterations
    let mut nb_iter:i64 = 0;
    while !stopping_criterion.is_finished() {
        let time_init_build = Instant::now();
        // invariant: current_sol is feasible
        let mut new_color_id;
        loop {
            // find the best color merge (couple of colors that minimize the conflict penalties)
            println!(
                " {:<15.3} it: {:<15} colors: {:<15} conflicts: {:<15} penalties: {:<15}",
               time_init+time_ls_init.elapsed().as_secs_f32(), nb_iter, current_sol.len(), 0, 0
            );
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
        // colors_bitset[c]: color c vertices
        let mut colors_bitsets:Vec<BitSet> = vec![BitSet::new() ; nb_colors];
        for (i,color) in current_sol.iter().enumerate() {
            for v in color {
                colors[*v] = i;
                colors_bitsets[i].insert(*v);
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
        // total penalty in the current state
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
        println!("builded the search state in {:.3} seconds", time_init_build.elapsed().as_secs_f32());
        let time_start_search = Instant::now();
        let mut aspiration_criterion:i64 = i64::MAX;
        // 2. perform local search iterations (for each conflicting vertex, find the best penalty)
        while nb_conflicting_edges > 0 && !stopping_criterion.is_finished() {
            assert!(nb_conflicting_edges > 0);
            nb_iter += 1;
            if nb_iter % 10000 == 0 {
                // aspiration_criterion = i64::MAX; // reinitialize the aspiration criterion
                aspiration_criterion += 1;
                println!(
                    " {:<15.3} it: {:<15} colors: {:<15} conflicts: {:<15} penalties: {:<15}",
                    time_init+time_ls_init.elapsed().as_secs_f32(),
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
                            if nb_conflicting_edges < aspiration_criterion || !tabu.contains(&current_node, &current_node) {
                                best_node = current_node; 
                            }
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
            colors_bitsets[best_node.previous_color].remove(best_node.vertex);
            colors_bitsets[best_node.next_color].insert(best_node.vertex);
            total_penalty = best_node.total_penalties; // update total penalty
            // mark the move tabu
            tabu.insert(&best_node, best_node.clone()); // make the decision tabu
            tabu.increment_iter();
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
                }
            }
            aspiration_criterion = std::cmp::min(aspiration_criterion, nb_conflicting_edges);
        }
        println!("search took {:.3} seconds", time_start_search.elapsed().as_secs_f32());
        if nb_conflicting_edges == 0 {
            current_sol = vec![ vec![] ; nb_colors ];
            for (u,c) in colors.iter().enumerate() {
                current_sol[*c].push(u);
            }
            assert_eq!(checker(inst.clone(), &current_sol), CheckerResult::Ok(current_sol.len()));
            best_sol = current_sol.clone();
        }
    }
    best_sol
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
            // "./insts/cgshop22/reecn3382.instance.json"
            // "./insts/cgshop22/reecn9674.instance.json"
            // "./insts/cgshop22/reecn12588.instance.json"
            // "./insts/cgshop22/reecn31126.instance.json"
            // "./insts/cgshop22/reecn73116.instance.json"
            "./insts/cgshop22/rvispecn17968.instance.json"
            // "./insts/cgshop_22_examples/visp_5K.instance.json"
            // "./insts/cgshop_22_examples/sqrm_10K_5.instance.json"
        ));
        let greedy_sol = greedy_dsatur(inst.clone(), false);
        println!("initial solution: {}", greedy_sol.len());
        let stopping_criterion:TimeStoppingCriterion = TimeStoppingCriterion::new(3000.);
        row_weighting_local_search(inst, &greedy_sol, stopping_criterion, None, None, 0.);
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