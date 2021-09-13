use std::rc::Rc;
use rand::{Rng, prelude::ThreadRng};

use dogs::combinators::helper::tabu_tenure::TabuTenure;

use crate::color::{ColoringInstance, Solution, checker};


/** simple tabu tenure that stores the insertions of colors */
#[derive(Debug)]
pub struct EjectionTabuTenure {
    /// tabu fixed size
    l:usize,
    /// tabu dynamic size
    lambda: f64,
    /// number of iterations since the beginning of the search
    nb_iter: usize,
    /// decisions[c]: last iteration in which color c was used
    decisions: Vec<Option<usize>>,
    /// random number generator
    rng: ThreadRng,
}

impl TabuTenure<usize, usize> for EjectionTabuTenure {
    fn insert(&mut self, _n:&usize, d:usize) {
        self.decisions[d] = Some(self.nb_iter);
        self.nb_iter += 1;
    }

    fn contains(&mut self, n:&usize, d:&usize) -> bool {
        match self.decisions[*d] {
            None => false,
            Some(i) => {
                let rand_l = self.rng.gen_range(0..=self.l);
                let threshold = rand_l + (self.lambda * (*n as f64)) as usize;
                threshold > self.nb_iter || i >= self.nb_iter - threshold
            }
        }
    }
}

impl EjectionTabuTenure {
    /** creates a tabu tenure given:
     - l: fixed tabu size
     - λ: variable tabu size
     - c: the maximum number of colors
    */
    pub fn new(l:usize, lambda: f64, c:usize) -> Self {
        Self {
            l, lambda,
            nb_iter: 0,
            decisions: vec![None ; c],
            rng: rand::thread_rng(),
        }
    }
}



/** Implements an ejection chain procedure.
 1. identifies the color $c_1$ with minimum size.
 2. choose another color $c_2$ with a minimum number of conflicts
 3. insert vertices of $c_1$, eject vertices of $c_2$
 4. repeat, trying to insert non-inserted vertices (or until max_iter is reached)

Each time an insertion is performed, mark the color tabu (cannot be inserted again)
*/
pub fn ejection_chain_iteration(inst:Rc<dyn ColoringInstance>, mut solution:Solution) -> Solution {
    let mut tabu = EjectionTabuTenure::new(10,0.6,solution.len());
    // identify $c_1$ the minimum size color
    let solution_copy = solution.clone(); // store the solution in case we fail improving it
    let (c1,_) = solution.iter().enumerate()
        .min_by_key(|(_,c)| c.len()).unwrap();
    tabu.insert(&0,c1);
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
            .filter(|(c2,n)| *c2!=c1 && !tabu.contains(*n, c2))
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
        tabu.insert(&0, c2); // c2 is now tabu
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
    }

    #[test]
    fn test_read_instance_sqrm() {
        let cg_inst = Rc::new(CGSHOPInstance::from_file(
            "./insts/CGSHOP_22_original/cgshop_2022_examples_01/example-instances-sqrm/sqrm_10K_1.instance.json",
            true
        ));
        let solution = cgshop_aog(cg_inst.clone(), false);
        println!("nb initial colors: {}", solution.len());
        let _ = ejection_chains(cg_inst, solution);
    }

}