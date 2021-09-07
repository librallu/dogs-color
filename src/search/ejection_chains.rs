use std::rc::Rc;

use bit_set::BitSet;

use crate::color::{ColoringInstance, Solution, checker};


/** Implements an ejection chain procedure.
 1. identifies the color $c_1$ with minimum size.
 2. choose another color $c_2$ with a minimum number of conflicts
 3. insert vertices of $c_1$, eject vertices of $c_2$
 4. repeat, trying to insert non-inserted vertices (or until max_iter is reached)

Each time an insertion is performed, mark the color tabu (cannot be inserted again)
*/
pub fn ejection_chain_iteration(inst:Rc<dyn ColoringInstance>, mut solution:Solution) -> Solution {
    let mut tabu = BitSet::new();
    // identify $c_1$ the minimum size color
    let solution_copy = solution.clone(); // store the solution in case we fail improving it
    let (c1,_) = solution.iter().enumerate()
        .min_by_key(|(_,c)| c.len()).unwrap();
    tabu.insert(c1);
    println!("ejection chains: minimum color: {} (size:{})", c1, solution[c1].len());
    let mut non_affected_vertices:Vec<usize> = solution[c1].clone();
    let mut failed_improving = false;
    loop {
        // compute conflicts for each other color
        let nb_conflicts:Vec<usize> = solution.iter().enumerate().map(|(i,c)| {
            let mut res:usize = 0;
            if i != c1 {
                for a in c {
                    let mut is_conflicting = false;
                    for b in &non_affected_vertices {
                        if inst.are_adjacent(*a, *b) {
                            is_conflicting = true;
                            break;
                        }
                    }
                    if is_conflicting { res += 1; }
                }
            }
            res
        }).collect();
        let (c2, nb_conflicts) = match nb_conflicts.iter().enumerate()
            .filter(|(c2,_)| !tabu.contains(*c2))
            .min_by_key(|(_,v)| **v) {
                None => { 
                    failed_improving = true;
                    break;
                },
                Some(res) => res
            };
        println!("inserting into {} ({} conflicts)", c2, nb_conflicts);
        // insert non affected edges and eject conflicting edges
        let mut conflicting_vertices:Vec<usize> = Vec::with_capacity(*nb_conflicts);
        let mut new_c2:Vec<usize> = Vec::with_capacity(solution[c2].len()-nb_conflicts+non_affected_vertices.len());
        for a in &solution[c2] {
            let mut is_conflicting = false;
            for b in &non_affected_vertices {
                if inst.are_adjacent(*a, *b) { // unassign a
                    is_conflicting = true;
                    break;
                }
            }
            if is_conflicting { // unassign a
                conflicting_vertices.push(*a);
            } else { // keep a
                new_c2.push(*a);
            }
        }
        new_c2.append(&mut non_affected_vertices.clone());
        solution[c2] = new_c2;
        tabu.insert(c2); // c2 is now tabu
        if conflicting_vertices.is_empty() { // solution with less colors found or max_iter reached
            break;
        }
        non_affected_vertices = conflicting_vertices;
    }
    // return best solution found
    if failed_improving {
        solution_copy // failed improving, get backup
    } else {
        solution.iter().enumerate().filter(|(i,_)| *i!=c1).map(|(_,c)| c).cloned().collect()
    }
}


/**
Repeats ejection chain procedure until no solution can be found.
*/
pub fn ejection_chains(inst:Rc<dyn ColoringInstance>, solution:Solution) -> Solution {
    let mut current_solution = solution;
    let mut nb_colors = current_solution.len();
    loop {
        current_solution = ejection_chain_iteration(inst.clone(), current_solution);
        let checker_result = checker(inst.clone(), &current_solution);
        match checker_result {
            crate::color::CheckerResult::Ok(_) => { println!("checker ok!"); },
            _ => panic!("checker detected an error: {:?}", checker_result)
        };
        if current_solution.len() < nb_colors {
            println!("ejection chains improves ({} -> {})", nb_colors, current_solution.len());
            nb_colors = current_solution.len();
        } else {
            break;
        }
    }
    current_solution
}



#[cfg(test)]
mod tests {
    use super::*;

    use crate::{cgshop::CGSHOPInstance, search::cgshop_aog::cgshop_aog};


    #[test]
    fn test_read_instance_visp() {
        let cg_inst = Rc::new(CGSHOPInstance::from_file(
            "./insts/CGSHOP_22_original/cgshop_2022_examples_01/example_instances_visp/visp_5K.instance.json",
            true
        ));
        let solution = cgshop_aog(cg_inst.clone(), false);
        println!("nb initial colors: {}", solution.len());
        let _ = ejection_chains(cg_inst, solution);
        // let solution2 = ejection_chain_iteration(cg_inst.clone(), solution);
        // let solution3 = ejection_chain_iteration(cg_inst, solution2);
    }

}