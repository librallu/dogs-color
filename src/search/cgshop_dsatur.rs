use std::cmp::max;

use ordered_float::OrderedFloat;
use bit_set::BitSet;

use crate::cgshop::{CGSHOPInstance, CGSHOPSolution};

/** implements an adapted DSATUR algorithm for the large CGSHOP instances
    1. sorts segments by length (the idea is: the largest probably has the more intersections)
    2. add the segment to the first color available
    3. mark all its intersections (O(|Segments|)) seeing its color
    4. select the next segment seeing the most colors (break ties by segment length)
*/
pub fn cgshop_dsatur(inst:&CGSHOPInstance) -> usize {
    let m = inst.m(); // nb segments
    let mut colors:Vec<Option<usize>> = vec![None ; m]; // colors[s] -> color assigned to segment s
    let mut adj_colors:Vec<BitSet> = vec![BitSet::default() ; m]; // adj_colors[s] -> colors s sees
    let mut nb_adj_colors:Vec<usize> = vec![0 ; m]; // nb_adj_colors[m] -> number of colors s sees.
    let mut nb_colored:usize = 0;
    let mut last_color:usize = 0;
    while nb_colored < m {
        if nb_colored % 1000 == 0 {
            println!("{} / {}", nb_colored, m);
        }
        // find uncolored segment with largest saturation degree, breaking ties by length
        let current_segment = (0..m)
            .filter(|segment| colors[*segment] == None)
            .max_by(|a, b| {
                nb_adj_colors[*a].cmp(&nb_adj_colors[*b])
                    .then_with(|| {
                        OrderedFloat(inst.squared_length(*a)).cmp(&OrderedFloat(inst.squared_length(*b)))
                    })
            }).unwrap();
        // assign it its color
        let mut color:usize = 0;
        while adj_colors[current_segment].contains(color) { color += 1; }
        colors[current_segment] = Some(color);
        nb_colored += 1;
        last_color = max(last_color, color); // update nb colors
        // check all its unasigned conflicting segments, and update their saturation information
        for conflict_segment in (0..m)
        .filter(|conflict_segment| colors[*conflict_segment] == None)
        .filter(|conflict_segment| inst.conflict(current_segment, *conflict_segment)) {
            if !adj_colors[conflict_segment].contains(color) { // update adj_colors & nb_adj_colors
                nb_adj_colors[conflict_segment] += 1;
                adj_colors[conflict_segment].insert(color);
            }
        }
    }
    // finished. Solution completed
    let nb_colors = last_color+1;
    println!("nb colors: {}", nb_colors);
    let solution = CGSHOPSolution::new(
        inst.id().to_string(),
        nb_colors,
        colors.iter().map(|e| e.unwrap()).collect()
    );
    solution.to_file("tmp/");
    nb_colors

}


#[cfg(test)]
mod tests {
    use super::*;

    use std::rc::Rc;
    use crate::tabucol::tabucol;
    use dogs::search_algorithm::TimeStoppingCriterion;

    #[test]
    fn test_read_instance_visp() {
        let cg_inst = CGSHOPInstance::from_file(
            "./insts/CGSHOP_22_original/cgshop_2022_examples_01/example_instances_visp/visp_5K.instance.json"
        );
        cg_inst.display_statistics();
        let _nb_colors = cgshop_dsatur(&cg_inst);
        // let vcp_inst = Rc::new(cg_inst.to_graph_coloring_instance());
        // tabucol(vcp_inst, nb_colors, TimeStoppingCriterion::new(100.), None);
    }

    #[test]
    fn test_read_instance_sqrm() {
        let cg_inst = CGSHOPInstance::from_file(
            "./insts/CGSHOP_22_original/cgshop_2022_examples_01/example-instances-sqrm/sqrm_5K_1.instance.json"
        );
        cg_inst.display_statistics();
        cgshop_dsatur(&cg_inst);
    }

    #[test]
    fn test_read_instance_sqrm_100K() {
        let cg_inst = CGSHOPInstance::from_file(
            "./insts/CGSHOP_22_original/cgshop_2022_examples_01/example-instances-sqrm/sqrm_100K_1.instance.json"
        );
        cg_inst.display_statistics();
        cgshop_dsatur(&cg_inst);
    }

    #[test]
    fn test_read_instance_tiny() {
        let cg_inst = CGSHOPInstance::from_file(
            "./insts/CGSHOP_22_original/cgshop_2022_examples_01/tiny.json"
        );
        cg_inst.display_statistics();
        cgshop_dsatur(&cg_inst);
    }
}