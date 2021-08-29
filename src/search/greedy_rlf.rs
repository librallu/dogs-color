use std::rc::Rc;

use bit_set::BitSet;

use crate::color::{ColoringInstance, Solution};

/** implements a greedy RLF algorithm. That colors vertices one color at a time
    1. selects the vertex with the largest degree in the graph and mark it colored
    2. mark its neighbors unreachable
    3. select a reachable vertex that has the largest set of edges in the reachable vertices
    4. when there are no reachable vertices, start over with a new color
*/
pub fn greedy_rlf(inst:Rc<dyn ColoringInstance>, show_completion:bool) -> Solution {
    let n:usize = inst.nb_vertices();
    let mut colors:Vec<Option<usize>> = vec![None ; n];
    let mut colored:BitSet<u64> = BitSet::default();
    let mut reachable_degree:Vec<usize> = (0..n).map(|u| inst.degree(u)).collect();
    let mut nb_colored:usize = 0;
    let mut current_color:usize = 0;
    while nb_colored < n { // add a new color until everything is colored
        let mut unreachable:BitSet<u64> = BitSet::default();
        let mut reachable_degree_removal:Vec<usize> = vec![0 ; n];
        // find not colored and reachable vertex with maximum degree
        loop {
            match (0..n)
            .filter(|v| !colored.contains(*v) && !unreachable.contains(*v))
            // .max_by_key(|v| reachable_degree[*v] - reachable_degree_removal[*v] ) {
            .max_by(|a,b| {
                reachable_degree_removal[*a].cmp(&reachable_degree_removal[*b])
                    .then_with(|| (reachable_degree[*a] - reachable_degree_removal[*a]).cmp(
                        &(reachable_degree[*b] - reachable_degree_removal[*b])
                    ))
            }) {
                None => { break; } // no more reachable vector, stpo and add
                Some(current_vertex) => {
                    if show_completion && nb_colored % 1000 == 0 { println!("colored {} / {}...", nb_colored, n); }
                    nb_colored += 1;
                    colored.insert(current_vertex);
                    colors[current_vertex] = Some(current_color);
                    // mark its neighbors unreachable and decrease their reachability degree
                    for v in inst.neighbors(current_vertex) {
                        if !unreachable.contains(v) && !colored.contains(v) {
                            // every vertex that sees v sees a reachable vertex less
                            for w in inst.neighbors(v) {
                                reachable_degree_removal[w] += 1; // because v is now unreachable
                            }
                            unreachable.insert(v);
                            reachable_degree[v] -= 1; // because current_vertex is now colored
                        }
                    }
                }
            }
        }
        current_color += 1;
    }
    // finished. Solution completed build the solution
    let mut res = vec![vec![] ; current_color];
    for (i,c) in colors.iter().enumerate() {
        res[c.unwrap()].push(i);
    }
    res
}




#[cfg(test)]
mod tests {
    use super::*;

    use crate::cgshop::CGSHOPInstance;

    #[test]
    fn test_read_instance_tiny() {
        let cg_inst = Rc::new(CGSHOPInstance::from_file(
            "./insts/CGSHOP_22_original/cgshop_2022_examples_01/tiny.json",
            true
        ));
        cg_inst.display_statistics();
        let solution = greedy_rlf(cg_inst, false);
        println!("nb colors: {}", solution.len());
    }

    #[test]
    fn test_read_instance_visp() {
        let cg_inst = Rc::new(CGSHOPInstance::from_file(
            "./insts/CGSHOP_22_original/cgshop_2022_examples_01/example_instances_visp/visp_5K.instance.json",
            true
        ));
        cg_inst.display_statistics();
        let solution = greedy_rlf(cg_inst, true);
        println!("nb colors: {}", solution.len());
    }

    #[test]
    fn test_read_instance_sqrm() {
        let cg_inst = Rc::new(CGSHOPInstance::from_file(
            "./insts/CGSHOP_22_original/cgshop_2022_examples_01/example-instances-sqrm/sqrm_5K_1.instance.json",
            true
        ));
        cg_inst.display_statistics();
        let solution = greedy_rlf(cg_inst, true);
        println!("nb colors: {}", solution.len());
    }

}