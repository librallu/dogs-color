use std::cmp::{Ordering, max, Ord};
use std::rc::Rc;

use priority_queue::PriorityQueue;
use bit_set::BitSet;

use crate::color::{ColoringInstance, Solution, VertexId};

#[derive(PartialEq, Eq)]
struct DSatInfo {
    dsat: usize,
    degree: usize
}

impl Ord for DSatInfo {
    fn cmp(&self, other: &Self) -> Ordering {
        self.dsat.cmp(&other.dsat)
            .then_with(|| self.degree.cmp(&other.degree))
    }
}

// `PartialOrd` needs to be implemented as well.
impl PartialOrd for DSatInfo {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

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
pub fn greedy_dsatur(inst:Rc<dyn ColoringInstance>, show_completion:bool) -> Solution {
    let n:usize = inst.nb_vertices();
    let mut remaining_vertices:PriorityQueue<VertexId, DSatInfo> = PriorityQueue::new();
    for i in 0..n {
        remaining_vertices.push(i, DSatInfo { dsat:0, degree:inst.degree(i)});
    }
    let mut colors:Vec<Option<usize>> = vec![None ; n]; // colors[v] -> color assigned to vertex v
    let mut adj_colors:Vec<BitSet> = vec![BitSet::default() ; n]; // adj_colors[n] -> colors n sees
    let mut nb_colored:usize = 0;
    let mut last_color:usize = 0;
    loop {
        if show_completion && nb_colored % 1000 == 0 { println!("colored {} / {}...", nb_colored, n); }
        // get current vertex
        let current_vertex = match remaining_vertices.pop() {
            None => break,
            Some(v) => v.0
        };
        // assign it a color
        let mut color:usize = 0;
        while adj_colors[current_vertex].contains(color) { color += 1; }
        colors[current_vertex] = Some(color);
        nb_colored += 1;
        last_color = max(last_color, color); // update nb colors
        // update saturation degree information
        for conflict_vertex in inst.neighbors(current_vertex).iter()
        .filter(|conflict_vertex| colors[**conflict_vertex] == None) {
            if !adj_colors[*conflict_vertex].contains(color) { // update adj_colors & nb_adj_colors
                // nb_adj_colors[*conflict_vertex] += 1;
                adj_colors[*conflict_vertex].insert(color);
                remaining_vertices.change_priority_by(conflict_vertex, |p| {p.dsat += 1;} )
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

    use crate::cgshop::CGSHOPInstance;

    #[test]
    fn test_read_instance_tiny() {
        let cg_inst = Rc::new(CGSHOPInstance::from_file(
            "./insts/CGSHOP_22_original/cgshop_2022_examples_01/tiny.json",
            true
        ));
        cg_inst.display_statistics();
        let solution = greedy_dsatur(cg_inst, false);
        println!("nb colors: {}", solution.len());
    }

    #[test]
    fn test_read_instance_visp() {
        let cg_inst = Rc::new(CGSHOPInstance::from_file(
            "./insts/CGSHOP_22_original/cgshop_2022_examples_01/example_instances_visp/visp_5K.instance.json",
            true
        ));
        cg_inst.display_statistics();
        let solution = greedy_dsatur(cg_inst, true);
        println!("nb colors: {}", solution.len());
    }

    #[test]
    fn test_read_instance_sqrm() {
        let cg_inst = Rc::new(CGSHOPInstance::from_file(
            "./insts/CGSHOP_22_original/cgshop_2022_examples_01/example-instances-sqrm/sqrm_5K_1.instance.json",
            true
        ));
        cg_inst.display_statistics();
        let solution = greedy_dsatur(cg_inst, true);
        println!("nb colors: {}", solution.len());
    }

    #[test]
    fn test_read_instance_sqrm_10k() {
        let cg_inst = Rc::new(CGSHOPInstance::from_file(
            "./insts/CGSHOP_22_original/cgshop_2022_examples_01/example-instances-sqrm/sqrm_10K_1.instance.json",
            true
        ));
        cg_inst.display_statistics();
        let solution = greedy_dsatur(cg_inst, true);
        println!("nb colors: {}", solution.len());
    }

    #[test]
    fn test_read_instance_sqrm_50k() {
        let cg_inst = Rc::new(CGSHOPInstance::from_file(
            "./insts/CGSHOP_22_original/cgshop_2022_examples_01/example-instances-sqrm/sqrm_50K_2.instance.json",
            true
        ));
        cg_inst.display_statistics();
        let solution = greedy_dsatur(cg_inst, true);
        println!("nb colors: {}", solution.len());
    }

    #[test]
    fn test_read_instance_visp_50k() {
        let cg_inst = Rc::new(CGSHOPInstance::from_file(
            "./insts/CGSHOP_22_original/cgshop_2022_examples_01/example_instances_visp/visp_50K.instance.json",
            true
        ));
        cg_inst.display_statistics();
        let solution = greedy_dsatur(cg_inst, true);
        println!("nb colors: {}", solution.len());
    }

    #[test]
    fn test_read_instance_sqrm_100k() {
        let cg_inst = Rc::new(CGSHOPInstance::from_file(
            "./insts/CGSHOP_22_original/cgshop_2022_examples_01/example-instances-sqrm/sqrm_100K_1.instance.json",
            true
        ));
        cg_inst.display_statistics();
        let solution = greedy_dsatur(cg_inst, true);
        println!("nb colors: {}", solution.len());
    }
}