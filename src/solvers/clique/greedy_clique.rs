use std::rc::Rc;

use bit_set::BitSet;

use crate::color::{ColoringInstance, VertexId};

/** implements a greedy algorithm that finds a "large" size clique.
The algorithm chooses the vertex with the largest degree. It marks as "candidates" its neighbors.
Then, while the set of candidates is not empty, choose the vertex with the largest degree.
*/
pub fn greedy_clique(inst:Rc<dyn ColoringInstance>) -> Vec<VertexId> {
    let n = inst.nb_vertices();
    let mut forbidden:BitSet<u64> = BitSet::default();
    let mut res = Vec::new();
    loop {
        match (0..n).filter(|v| !forbidden.contains(*v)).max_by_key(|v| inst.degree(*v)) {
            None => break,
            Some(current_vertex) => {
                // insert the current vertex as part of the clique solution
                res.push(current_vertex);
                // mark the non neighbors as forbidden
                let mut neighbors:BitSet<u64> = BitSet::default();
                for v in inst.neighbors(current_vertex) {
                    neighbors.insert(v);
                }
                for v in 0..n {
                    if !neighbors.contains(v) {
                        forbidden.insert(v);
                    }
                }
            }
        };
    }
    println!("greedy clique: {}", res.len());
    // try improving the clique
    let mut res_bitset = BitSet::new();
    for s in res.iter() { res_bitset.insert(*s); }
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
        let solution = greedy_clique(cg_inst);
        println!("clique size: {}", solution.len());
    }

    #[test]
    fn test_read_instance_visp() {
        let cg_inst = Rc::new(CGSHOPInstance::from_file(
            "./insts/CGSHOP_22_original/cgshop_2022_examples_01/example_instances_visp/visp_5K.instance.json"
        ));
        cg_inst.display_statistics();
        let solution = greedy_clique(cg_inst);
        println!("clique size: {}", solution.len());
    }

    #[test]
    fn test_read_instance_sqrm() {
        let cg_inst = Rc::new(CGSHOPInstance::from_file(
            "./insts/CGSHOP_22_original/cgshop_2022_examples_01/example-instances-sqrm/sqrm_5K_1.instance.json"
        ));
        cg_inst.display_statistics();
        let solution = greedy_clique(cg_inst);
        println!("clique size: {}", solution.len());
    }

    #[test]
    fn test_read_instance_sqrm_10k() {
        let cg_inst = Rc::new(CGSHOPInstance::from_file(
            "./insts/CGSHOP_22_original/cgshop_2022_examples_01/example-instances-sqrm/sqrm_10K_1.instance.json"
        ));
        cg_inst.display_statistics();
        let solution = greedy_clique(cg_inst);
        println!("clique size: {}", solution.len());
    }

    #[test]
    fn test_read_instance_sqrm_100k() {
        let cg_inst = Rc::new(CGSHOPInstance::from_file(
            "./insts/CGSHOP_22_original/cgshop_2022_examples_01/example-instances-sqrm/sqrm_100K_1.instance.json"
        ));
        cg_inst.display_statistics();
        let solution = greedy_clique(cg_inst);
        println!("clique size: {}", solution.len());
    }

    #[test]
    fn test_read_instance_visp_100k() {
        let cg_inst = Rc::new(CGSHOPInstance::from_file(
            "./insts/CGSHOP_22_original/cgshop_2022_examples_01/example_instances_visp/visp_100K.instance.json"
        ));
        cg_inst.display_statistics();
        let solution = greedy_clique(cg_inst);
        println!("clique size: {}", solution.len());
    }

}