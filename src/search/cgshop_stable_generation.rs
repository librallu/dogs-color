use std::rc::Rc;

use ordered_float::OrderedFloat;

use crate::{cgshop::CGSHOPInstance, color::{ColoringInstance, Solution}};

use super::util::mip_set_covering::mip_cbc_set_covering;


/** builds a stable corresponding to some closely related segments. */
pub fn segment_to_stable(inst:Rc<CGSHOPInstance>, segment:usize) -> Vec<usize> {
    // sorts segments by proximity to *segemnt*
    let n = inst.nb_vertices();
    let mut sorted_segments:Vec<usize> = (0..n).collect();
    let segment_orientation = inst.segment_orientation(segment);
    sorted_segments.sort_by_key(|i|
        OrderedFloat((segment_orientation - inst.segment_orientation(*i)).abs())
    );
    // build stable
    let mut res = Vec::new();
    for s in sorted_segments {
        let mut is_conflicting = false;
        for existing_in_solution in res.iter() {
            if inst.are_adjacent(s, *existing_in_solution) {
                is_conflicting = true;
                break;
            }
        }
        if !is_conflicting {
            res.push(s);
        }
    }
    res
}


/** For each segment, generate a stable.
Then, solve a set covering problem to minimize the number of colors.
*/
pub fn cgshop_stables_by_orientation(inst:Rc<CGSHOPInstance>, show_completion:bool) -> Solution {
    let n = inst.nb_vertices();
    if show_completion { println!("generating stables..."); }
    let mut stables:Vec<Vec<usize>> = Vec::with_capacity(n);
    for i in 0..n {
        if show_completion && i % 1000 == 0 { println!("stable {} / {}...", i, n); }
        stables.push(segment_to_stable(inst.clone(), i));
    }
    if show_completion { println!("solving the set covering problem..."); }
    match mip_cbc_set_covering(n, &stables) {
        None => { panic!("set covering algorithm found no solution"); },
        Some(s) => {
            println!("{:?}", s);
            println!("{} colors", s.len());
        }
    }
    Vec::new()
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
        let solution = cgshop_stables_by_orientation(cg_inst, false);
        println!("nb colors: {}", solution.len());
    }

    #[test]
    fn test_read_instance_sqrm() {
        let cg_inst = Rc::new(CGSHOPInstance::from_file(
            "./insts/CGSHOP_22_original/cgshop_2022_examples_01/example-instances-sqrm/sqrm_5K_1.instance.json",
            true
        ));
        cg_inst.display_statistics();
        let solution = cgshop_stables_by_orientation(cg_inst, true);
        println!("nb colors: {}", solution.len());
    }

    #[test]
    fn test_read_instance_visp() {
        let cg_inst = Rc::new(CGSHOPInstance::from_file(
            "./insts/CGSHOP_22_original/cgshop_2022_examples_01/example_instances_visp/visp_5K.instance.json",
            true
        ));
        cg_inst.display_statistics();
        let solution = cgshop_stables_by_orientation(cg_inst, true);
        println!("nb colors: {}", solution.len());
    }

}