use std::rc::Rc;

use bit_set::BitSet;
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



/**
Admissible Orientation Greedy algorithm for the CGSHOP challenge
Sorts the segments by orientation and apply a simple coloring algorithm.
Chooses to select the non-conflicting color-class that is the closest to the edge direction
*/
pub fn cgshop_aog_v2(inst:Rc<CGSHOPInstance>, show_completion:bool) -> Solution {
    let n = inst.nb_vertices();
    let mut sorted_segments:Vec<usize> = (0..n).collect();
    sorted_segments.sort_by_key(|i| OrderedFloat(inst.segment_orientation(*i)));
    let mut res:Vec<Vec<usize>> = Vec::new();
    let mut avg_orientation:Vec<f64> = Vec::new();
    let mut nb_colored = 0;
    for i in sorted_segments { // add segments one by one
        nb_colored += 1;
        if show_completion && nb_colored % 1000 == 0 { println!("colored {} / {}...", nb_colored, n); }
        let mut added = false;
        // iterate over the existing colors
        let mut existing_colors:Vec<usize> = (0..res.len()).collect();
        // existing_colors.sort_by_key(|c| -OrderedFloat((avg_orientation[*c]-inst.segment_orientation(i)).abs()) );
        existing_colors.sort_by_key(|c| -(res[*c].len() as i64));
        for current_color in existing_colors {
            let mut is_conflicting = false;
            for j in &res[current_color] {
                if inst.are_adjacent(i, *j) {
                    is_conflicting = true;
                    break;
                }
            }
            if !is_conflicting {
                res[current_color].push(i); // add color i to current color
                avg_orientation[current_color] += (inst.segment_orientation(i)-avg_orientation[current_color])
                    / res[current_color].len() as f64;
                added = true;
                break;
            }
        }
        // if not added, create a new color
        if !added {
            res.push(vec![i]);
            avg_orientation.push(inst.segment_orientation(i));
        }
    }
    res
}


/**
Admissible Orientation Greedy algorithm for the CGSHOP challenge
finds an objective orientation (weighted average of remaining segment orientations)
Then, generate a stable, choosing first segments closest to this orientation.
*/
pub fn cgshop_aog_v3(inst:Rc<CGSHOPInstance>, show_completion:bool) -> Solution {
    let mut res:Solution = Vec::new();
    let n = inst.nb_vertices();
    let mut colored = BitSet::with_capacity(n);
    let mut nb_colored:usize = 0;
    while nb_colored < n { // while not all vertices are colored
        let mut uncolored_segments:Vec<usize> = (0..n).filter(|i| !colored.contains(*i)).collect();
        // find average orientation
        let goal_orientation:f64 = uncolored_segments.iter()
            .map(|i| inst.segment_orientation(*i)).sum::<f64>() / uncolored_segments.len() as f64;
        // let goal_orientation:f64 = inst.segment_orientation(*uncolored_segments.iter()
        //     .max_by_key(|s| OrderedFloat(inst.squared_length(**s))).unwrap());
        // sort and add uncolored segments by proximity to goal orientation
        uncolored_segments.sort_by_key(|i| OrderedFloat(
            (inst.segment_orientation(*i) - goal_orientation).abs()
        ));
        let mut current_segments:Vec<usize> = Vec::new();
        for segment in uncolored_segments.iter() {
            let mut is_conflicting = false;
            for s in current_segments.iter() {
                if inst.are_adjacent(*s, *segment) {
                    is_conflicting = true;
                    break;
                }
            }
            if !is_conflicting { // add the segment to the current segment list (color)
                current_segments.push(*segment);
                nb_colored += 1;
                if show_completion && nb_colored % 1000 == 0 { println!("colored {} / {}...", nb_colored, n); }
                colored.insert(*segment);
            }
        }
        res.push(current_segments);
    }
    res
}


/** which segment orientation to choose */
#[derive(Debug)]
pub enum SegmentOrientationStrategy {
    /// sort by increasing orientation
    Increasing,
    /// sort by decreasing orientation
    Decreasing,
    /// alternate between each color
    Alternate
}

/**
Admissible Orientation Greedy algorithm for the CGSHOP challenge.
Adds color by color, sorting segments by orientation (decreasing if even, increasing if odd)
*/
pub fn cgshop_aog_v4(inst:Rc<CGSHOPInstance>, show_completion:bool, orientation_strategy:SegmentOrientationStrategy) -> Solution {
    let n = inst.nb_vertices();
    let mut res:Vec<Vec<usize>> = Vec::new();
    let mut colored:BitSet = BitSet::with_capacity(n);
    let mut nb_colored = 0;
    while nb_colored < n {
        let mut current_color = Vec::new();
        // sort remaining segments
        let mut sorted_segments:Vec<usize> = (0..n).filter(|i| !colored.contains(*i)).collect();
        sorted_segments.sort_by_key(|i| {
            match orientation_strategy {
                SegmentOrientationStrategy::Increasing => {
                    OrderedFloat(inst.segment_orientation(*i))
                },
                SegmentOrientationStrategy::Decreasing => {
                    -OrderedFloat(inst.segment_orientation(*i))
                },
                SegmentOrientationStrategy::Alternate => {
                    if res.len() % 2 == 0 {
                        OrderedFloat(inst.segment_orientation(*i))
                    } else {
                        -OrderedFloat(inst.segment_orientation(*i))
                    }
                }
            }
        });
        // iterate over segments, and add them if possible
        for i in &sorted_segments {
            let mut is_conflicting = false;
            for j in &current_color {
                if inst.are_adjacent(*i, *j) {
                    is_conflicting = true;
                    break;
                }
            }
            if !is_conflicting {
                current_color.push(*i);
                nb_colored += 1;
                colored.insert(*i);
                if show_completion && nb_colored % 1000 == 0 { println!("colored {} / {}...", nb_colored, n); }
            }
        }
        // add to the color pool and repeat
        res.push(current_color);
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
            "./insts/CGSHOP_22_original/cgshop_2022_examples_01/tiny.json"
        ));
        cg_inst.display_statistics();
        let solution = cgshop_aog(cg_inst, false);
        println!("nb colors: {}", solution.len());
    }

    #[test]
    fn test_read_instance_tiny_v3() {
        let cg_inst = Rc::new(CGSHOPInstance::from_file(
            "./insts/CGSHOP_22_original/cgshop_2022_examples_01/tiny.json"
        ));
        cg_inst.display_statistics();
        let solution = cgshop_aog_v3(cg_inst, false);
        println!("nb colors: {}", solution.len());
    }

    #[test]
    fn test_read_instance_visp() {
        let cg_inst = Rc::new(CGSHOPInstance::from_file(
            "./insts/CGSHOP_22_original/cgshop_2022_examples_01/example_instances_visp/visp_5K.instance.json"
        ));
        cg_inst.display_statistics();
        let solution = cgshop_aog(cg_inst, true);
        println!("nb colors: {}", solution.len());
        let nb_segments:Vec<usize> = solution.iter().map(|c| c.len()).collect();
        println!("{:?}", nb_segments);
    }

    #[test]
    fn test_read_instance_visp_50k() {
        let cg_inst = Rc::new(CGSHOPInstance::from_file(
            "./insts/CGSHOP_22_original/cgshop_2022_examples_01/example_instances_visp/visp_50K.instance.json"
        ));
        cg_inst.display_statistics();
        let solution = cgshop_aog_v3(cg_inst, true);
        println!("nb colors: {}", solution.len());
    }

    #[test]
    fn test_read_instance_sqrm() {
        let cg_inst = Rc::new(CGSHOPInstance::from_file(
            "./insts/CGSHOP_22_original/cgshop_2022_examples_01/example-instances-sqrm/sqrm_5K_1.instance.json"
        ));
        cg_inst.display_statistics();
        let solution = cgshop_aog(cg_inst, true);
        println!("nb colors: {}", solution.len());
        let nb_segments:Vec<usize> = solution.iter().map(|c| c.len()).collect();
        println!("{:?}", nb_segments);
    }

    #[test]
    fn test_read_instance_sqrm_v3() {
        let cg_inst = Rc::new(CGSHOPInstance::from_file(
            "./insts/CGSHOP_22_original/cgshop_2022_examples_01/example-instances-sqrm/sqrm_5K_1.instance.json"
        ));
        cg_inst.display_statistics();
        let solution = cgshop_aog_v3(cg_inst, true);
        println!("nb colors: {}", solution.len());
    }

    #[test]
    fn test_read_instance_sqrm_50k() {
        let cg_inst = Rc::new(CGSHOPInstance::from_file(
            "./insts/CGSHOP_22_original/cgshop_2022_examples_01/example-instances-sqrm/sqrm_50K_1.instance.json"
        ));
        cg_inst.display_statistics();
        let solution = cgshop_aog(cg_inst, true);
        println!("nb colors: {}", solution.len());
        let nb_segments:Vec<usize> = solution.iter().map(|c| c.len()).collect();
        println!("{:?}", nb_segments);
    }

    #[test]
    fn test_read_instance_sqrm_50k_v2() {
        let cg_inst = Rc::new(CGSHOPInstance::from_file(
            "./insts/CGSHOP_22_original/cgshop_2022_examples_01/example-instances-sqrm/sqrm_50K_1.instance.json"
        ));
        cg_inst.display_statistics();
        let solution = cgshop_aog_v2(cg_inst, true);
        println!("nb colors: {}", solution.len());
    }

    #[test]
    fn test_read_instance_sqrm_50k_v3() {
        let cg_inst = Rc::new(CGSHOPInstance::from_file(
            "./insts/CGSHOP_22_original/cgshop_2022_examples_01/example-instances-sqrm/sqrm_50K_1.instance.json"
        ));
        cg_inst.display_statistics();
        let solution = cgshop_aog_v3(cg_inst, true);
        println!("nb colors: {}", solution.len());
    }

    #[test]
    fn test_read_instance_sqrm_100k() {
        let cg_inst = Rc::new(CGSHOPInstance::from_file(
            "./insts/CGSHOP_22_original/cgshop_2022_examples_01/example-instances-sqrm/sqrm_100K_1.instance.json"
        ));
        cg_inst.display_statistics();
        let solution = cgshop_aog(cg_inst, true);
        println!("nb colors: {}", solution.len());
    }

    #[test]
    fn test_read_instance_sqrm_500k() {
        let cg_inst = Rc::new(CGSHOPInstance::from_file(
            "./insts/CGSHOP_22_original/cgshop_2022_examples_01/example-instances-sqrm/sqrm_500K_1.instance.json"
        ));
        cg_inst.display_statistics();
        let solution = cgshop_aog(cg_inst, true);
        println!("nb colors: {}", solution.len());
    }

    // TESTS AOG V4
    #[test]
    fn sqrm_5k_1_aog_v4() {
        let cg_inst = Rc::new(CGSHOPInstance::from_file(
            "./insts/CGSHOP_22_original/cgshop_2022_examples_01/example_instances_visp/visp_50K.instance.json"
        ));
        let solution_i = cgshop_aog_v4(
            cg_inst.clone(),
            false,
            SegmentOrientationStrategy::Increasing
        );
        println!("Increasing:\t{}", solution_i.len());
        let solution_d = cgshop_aog_v4(
            cg_inst.clone(),
            false,
            SegmentOrientationStrategy::Decreasing
        );
        println!("Decreasing:\t{}", solution_d.len());
        let solution_a = cgshop_aog_v4(
            cg_inst,
            false,
            SegmentOrientationStrategy::Alternate
        );
        println!("Alternate :\t{}", solution_a.len());
    }


}