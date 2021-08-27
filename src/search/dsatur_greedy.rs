use std::cmp::max;
use std::rc::Rc;

use bit_set::BitSet;

use crate::color::{ColoringInstance, Solution};

/** implements a greedy DSATUR algorithm. This algorithm should be able to handle large scale
instances (up to 1M nodes and 1T edges).
    1. choose an uncolored node that sees the most colors (break ties by the largest degree)
    2. add the segment to the first color available
    3. mark all its neighbors seeing this color
    4. repeat until a proper coloring is found

parameters:
 - inst: reference to an instance
 - show_completion: if true, print progress towards the coloring (useful to indicate the progression)
*/
pub fn dsatur_greedy(inst:Rc<dyn ColoringInstance>, show_completion:bool) -> Solution {
    let n:usize = inst.nb_vertices();
    let mut colors:Vec<Option<usize>> = vec![None ; n]; // colors[v] -> color assigned to vertex v
    let mut adj_colors:Vec<BitSet> = vec![BitSet::default() ; n]; // adj_colors[n] -> colors n sees
    let mut nb_adj_colors:Vec<usize> = vec![0 ; n]; // nb_adj_colors[n] -> number of colors n sees.
    let mut nb_colored:usize = 0;
    let mut last_color:usize = 0;
    while nb_colored < n {
        if show_completion && nb_colored % 1000 == 0 { println!("colored {} / {}...", nb_colored, n); }
        // find uncolored segment with largest saturation degree, breaking ties by length
        let current_vertex = (0..n)
            .filter(|v| colors[*v] == None)
            .max_by(|a, b| {
                nb_adj_colors[*a].cmp(&nb_adj_colors[*b]).then_with(|| inst.degree(*a).cmp(&inst.degree(*b)))
            }).unwrap();
        // assign it its color
        let mut color:usize = 0;
        while adj_colors[current_vertex].contains(color) { color += 1; }
        colors[current_vertex] = Some(color);
        nb_colored += 1;
        last_color = max(last_color, color); // update nb colors
        // check all its unasigned conflicting segments, and update their saturation information
        for conflict_vertex in inst.neighbors(current_vertex).iter()
        .filter(|conflict_vertex| colors[**conflict_vertex] == None) {
            if !adj_colors[*conflict_vertex].contains(color) { // update adj_colors & nb_adj_colors
                nb_adj_colors[*conflict_vertex] += 1;
                adj_colors[*conflict_vertex].insert(color);
            }
        }
    }
    // finished. Solution completed build the solution
    let mut res = vec![vec![] ; last_color+1];
    for (i,c) in colors.iter().enumerate() {
        res[c.unwrap()].push(i);
    }
    res
}




#[cfg(test)]
mod tests {
    use super::*;

    use crate::cgshop::{CGSHOPInstance,CGSHOPSolution};

    #[test]
    fn test_read_instance_tiny() {
        let cg_inst = Rc::new(CGSHOPInstance::from_file(
            "./insts/CGSHOP_22_original/cgshop_2022_examples_01/tiny.json"
        ));
        cg_inst.display_statistics();
        let solution = dsatur_greedy(cg_inst, false);
        println!("nb colors: {}", solution.len());
    }

    #[test]
    fn test_read_instance_visp() {
        let cg_inst = Rc::new(CGSHOPInstance::from_file(
            "./insts/CGSHOP_22_original/cgshop_2022_examples_01/example_instances_visp/visp_5K.instance.json"
        ));
        cg_inst.display_statistics();
        let solution = dsatur_greedy(cg_inst.clone(), true);
        println!("nb colors: {}", solution.len());
        let cg_sol = CGSHOPSolution::from_solution(cg_inst.id(), &solution);
        cg_sol.to_file("insts/CGSHOP_22_original/");
    }

    #[test]
    fn test_read_instance_sqrm() {
        let cg_inst = Rc::new(CGSHOPInstance::from_file(
            "./insts/CGSHOP_22_original/cgshop_2022_examples_01/example-instances-sqrm/sqrm_5K_1.instance.json"
        ));
        cg_inst.display_statistics();
        let solution = dsatur_greedy(cg_inst, true);
        println!("nb colors: {}", solution.len());
    }

    #[test]
    fn test_read_instance_sqrm_50k() {
        let cg_inst = Rc::new(CGSHOPInstance::from_file(
            "./insts/CGSHOP_22_original/cgshop_2022_examples_01/example-instances-sqrm/sqrm_50K_2.instance.json"
        ));
        cg_inst.display_statistics();
        let solution = dsatur_greedy(cg_inst, true);
        println!("nb colors: {}", solution.len());
    }

    #[test]
    fn test_read_instance_visp_50k() {
        let cg_inst = Rc::new(CGSHOPInstance::from_file(
            "./insts/CGSHOP_22_original/cgshop_2022_examples_01/example_instances_visp/visp_50K.instance.json"
        ));
        cg_inst.display_statistics();
        let solution = dsatur_greedy(cg_inst, true);
        println!("nb colors: {}", solution.len());
    }

    #[test]
    fn test_read_instance_sqrm_100k() {
        let cg_inst = Rc::new(CGSHOPInstance::from_file(
            "./insts/CGSHOP_22_original/cgshop_2022_examples_01/example-instances-sqrm/sqrm_100K_1.instance.json"
        ));
        cg_inst.display_statistics();
        let solution = dsatur_greedy(cg_inst, true);
        println!("nb colors: {}", solution.len());
    }
}