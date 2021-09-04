use std::rc::Rc;

use ordered_float::OrderedFloat;

use crate::{cgshop::CGSHOPInstance, color::{ColoringInstance, Solution}};



/**
Admissible Orientation Greedy algorithm for the CGSHOP challenge
Sorts the segments by orientation and apply a simple coloring algorithm.
*/
pub fn cgshop_aog(inst:Rc<CGSHOPInstance>, show_completion:bool) -> Solution {
    let n = inst.nb_vertices();
    let mut sorted_segments:Vec<usize> = (0..n).collect();
    sorted_segments.sort_by_key(|i| OrderedFloat(inst.segment_orientation(*i)));
    let mut res:Vec<Vec<usize>> = Vec::new();
    let mut nb_colored = 0;
    for i in sorted_segments { // add segments one by one
        nb_colored += 1;
        if show_completion && nb_colored % 1000 == 0 { println!("colored {} / {}...", nb_colored, n); }
        let mut current_color = 0;
        let mut added = false;
        while current_color < res.len() {
            let mut is_conflicting = false;
            for j in &res[current_color] {
                if inst.are_adjacent(i, *j) {
                    is_conflicting = true;
                    break;
                }
            }
            if !is_conflicting {
                res[current_color].push(i); // add color i to current color
                added = true;
                break;
            } else {
                current_color += 1; // try next color
            }
        }
        // if not added, create a new color
        if !added {
            res.push(vec![i]);
        }
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
        let solution = cgshop_aog(cg_inst, false);
        println!("nb colors: {}", solution.len());
    }

    #[test]
    fn test_read_instance_visp() {
        let cg_inst = Rc::new(CGSHOPInstance::from_file(
            "./insts/CGSHOP_22_original/cgshop_2022_examples_01/example_instances_visp/visp_5K.instance.json",
            true
        ));
        cg_inst.display_statistics();
        let solution = cgshop_aog(cg_inst, true);
        println!("nb colors: {}", solution.len());
    }

    #[test]
    fn test_read_instance_sqrm() {
        let cg_inst = Rc::new(CGSHOPInstance::from_file(
            "./insts/CGSHOP_22_original/cgshop_2022_examples_01/example-instances-sqrm/sqrm_5K_1.instance.json",
            true
        ));
        cg_inst.display_statistics();
        let solution = cgshop_aog(cg_inst, true);
        println!("nb colors: {}", solution.len());
    }

    #[test]
    fn test_read_instance_sqrm_10k() {
        let cg_inst = Rc::new(CGSHOPInstance::from_file(
            "./insts/CGSHOP_22_original/cgshop_2022_examples_01/example-instances-sqrm/sqrm_10K_1.instance.json",
            true
        ));
        cg_inst.display_statistics();
        let solution = cgshop_aog(cg_inst, true);
        println!("nb colors: {}", solution.len());
    }

    #[test]
    fn test_read_instance_sqrm_100k() {
        let cg_inst = Rc::new(CGSHOPInstance::from_file(
            "./insts/CGSHOP_22_original/cgshop_2022_examples_01/example-instances-sqrm/sqrm_100K_1.instance.json",
            true
        ));
        cg_inst.display_statistics();
        let solution = cgshop_aog(cg_inst, true);
        println!("nb colors: {}", solution.len());
    }

    #[test]
    fn test_read_instance_sqrm_500k() {
        let cg_inst = Rc::new(CGSHOPInstance::from_file(
            "./insts/CGSHOP_22_original/cgshop_2022_examples_01/example-instances-sqrm/sqrm_500K_1.instance.json",
            true
        ));
        cg_inst.display_statistics();
        let solution = cgshop_aog(cg_inst, true);
        println!("nb colors: {}", solution.len());
    }


}