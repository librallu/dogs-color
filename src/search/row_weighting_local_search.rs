use std::{ops::BitAnd, rc::Rc, thread::current};

use bit_set::BitSet;
use fastrand::Rng;

use dogs::{data_structures::sparse_set::SparseSet, search_algorithm::StoppingCriterion};

use crate::color::{ColoringInstance, VertexId};

type Penalty = i64;

pub fn row_weighting_local_search<Stopping:StoppingCriterion>(inst:Rc<dyn ColoringInstance>, sol:&[Vec<VertexId>], stopping_criterion:Stopping, solution_filename:Option<String>, perf_filename:Option<String>) -> Vec<Vec<VertexId>> {
    let n = inst.nb_vertices();
    let mut c = sol.len();
    // penalties learned for each edge. It indicates how difficult 
    let mut penalties:Vec<Vec<Penalty>> = vec![ vec![1 ; n] ; n];
    // nb_neigh_colors[v][c]: number of neighbors of v that are assigned color c
    let mut nb_neigh_colors:Vec<Vec<usize>> = vec![ vec![0 ; n] ; c ];
    // conflicting_vertices: list of vertices that have some conflict
    let mut conflicting_vertices:SparseSet = SparseSet::new(n);
    // for each vertex, the number of conflicts
    let mut vertex_nb_conflicts:Vec<i64> = vec![0 ; n];
    // number of conflicting edges
    let mut nb_conflicting_edges:i64 = 0;
    // random number generator
    let rng:Rng = Rng::new();
    let mut current_sol = sol.to_vec();
    // perform iterations
    while !stopping_criterion.is_finished() {
        // merge 2 colors, minimizing the weight of the conflicting edges until there are some conflicts
        let mut best_penalties:Penalty;
        let mut current_best:Option<(VertexId,VertexId)>;
        loop {
            best_penalties = Penalty::MAX;
            current_best = None;
            for (i1, color1) in current_sol.iter().enumerate() {
                let mut color1_bitset = BitSet::new();
                for v in color1 { color1_bitset.insert(*v); }
                for (i2, color2) in current_sol.iter().enumerate() {
                    if i2 < i1 {
                        let mut current_penalties:Penalty = 0;
                        for u in color2 {
                            for v in inst.neighbors(*u) {
                                if color1_bitset.contains(v) {
                                    current_penalties += penalties[*u][v];
                                }
                            }
                        }
                        if current_penalties < best_penalties {
                            best_penalties = current_penalties;
                            current_best = Some((i1,i2));
                        }

                    }
                }
            }
            if best_penalties > 0 {
                break;
            } else {
                println!("merging with 0 penalties!");
            }
        }
        println!("merging {:?}, with {} penalties", current_best, best_penalties);
        // perform the local search
    }

    // report statistics & solutions

    // let max_degree = (0..n).map(|e| inst.degree(e)).max().unwrap();
    // // local search data-structures
    // let vertices = vec![0 ; n];
    // let mut penalties = vec![max_degree ; n];
    // let mut nb_conflicts = 0;
    // let mut current_sol = sol.to_vec();
    // // iterate until the time limit is reached
    // while !stopping_criterion.is_finished() {
    //     // while the solution is valid, merge 2 colors
    //     while nb_conflicts == 0 {
    //         println!("(val:{}) \t merging 2 colors...", current_sol.len());
    //         // compute colors
    //         let mut colors:Vec<usize> = vec![0 ; n];
    //         for (i,color) in current_sol.iter().enumerate() {
    //             for v in color { colors[*v] = i; }
    //         }
    //         // compute positions // TODO
    //         // initialize penalty structure
    //         let penalties:Vec<Vec<Penalty>> = vec![ vec![0 ; current_sol.len()-1] ; current_sol.len()];
    //         // compute penalties
    //         for u in inst.vertices() {
    //             for v in inst.neighbors(u) {
    //                 if u < v { // (u,v) represent edges
    //                     let cu = colors[u];
    //                     let cv = colors[v];
    //                     let cmin = std::cmp::min(cu,cv);
    //                     let cmax = std::cmp::max(cu,cv);
    //                     // penalties[cmin][cmax] += todo!();
    //                 }
    //             }
    //         }
    //         // find best color combination
    //         // apply color merge

    //         nb_conflicts += 1; // TODO remove
    //     }

    //     // choose randomly a conflicting edge
    //     // find the best swap
    //     // update penalties
    // } 

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
        ));
        let greedy_sol = greedy_dsatur(inst.clone(), false);
        println!("initial solution: {}", greedy_sol.len());
        let stopping_criterion:TimeStoppingCriterion = TimeStoppingCriterion::new(3000.);
        row_weighting_local_search(inst, &greedy_sol, stopping_criterion, None, None);
    }
}