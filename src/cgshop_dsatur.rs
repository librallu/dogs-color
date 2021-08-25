use std::cmp::max;

use ordered_float::OrderedFloat;
use bit_set::BitSet;

use crate::cgshop::CGSHOPInstance;

/** implements an adapted DSATUR algorithm for the large CGSHOP instances
    1. sorts segments by length (the idea is: the largest probably has the more intersections)
    2. add the segment to the first color available
    3. mark all its intersections (O(|Segments|)) seeing its color
    4. select the next segment seeing the most colors (break ties by segment length)
*/
pub fn cgshop_dsatur(inst:&CGSHOPInstance) {
    let m = inst.m(); // nb segments
    let mut colors:Vec<Option<usize>> = vec![None ; m]; // colors[s] -> color assigned to segment s
    let mut adj_colors:Vec<BitSet> = vec![BitSet::default() ; m]; // adj_colors[s] -> colors s sees
    let mut nb_adj_colors:Vec<usize> = vec![0 ; m]; // nb_adj_colors[m] -> number of colors s sees.
    let mut nb_colored:usize = 0;
    let mut nb_colors:usize = 0;
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
        nb_colors = max(nb_colors, color); // update nb colors
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
    println!("nb colors: {}", nb_colors);
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_instance() {
        let cg_inst = CGSHOPInstance::from_file(
            "./insts/CGSHOP_22_original/cgshop_2022_examples_01/example_instances_visp/visp_10K.instance.json"
        );
        cg_inst.display_statistics();
        cgshop_dsatur(&cg_inst);
    }

    #[test]
    fn test_read_instance_sqrm() {
        let cg_inst = CGSHOPInstance::from_file(
            "./insts/CGSHOP_22_original/cgshop_2022_examples_01/example-instances-sqrm/sqrm_5K_1.instance.json"
        );
        cg_inst.display_statistics();
        cgshop_dsatur(&cg_inst);
    }
}