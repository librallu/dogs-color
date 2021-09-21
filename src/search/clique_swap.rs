use std::rc::Rc;

use bit_set::BitSet;
use rand::{Rng, prelude::ThreadRng};

use dogs::combinators::helper::tabu_tenure::TabuTenure;

use crate::color::{ColoringInstance, VertexId};


/** simple tabu tenure that stores the insertions of colors */
#[derive(Debug)]
pub struct CliqueSwapTenure {
    /// tabu fixed size
    l:usize,
    /// tabu dynamic size
    lambda: f64,
    /// number of iterations since the beginning of the search
    nb_iter: usize,
    /// decisions[c]: last iteration in which vertex c was inserted
    decisions: Vec<Option<usize>>,
    /// random number generator
    rng: ThreadRng,
}

impl TabuTenure<usize, usize> for CliqueSwapTenure {
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

impl CliqueSwapTenure {
    /** creates a tabu tenure given:
     - l: fixed tabu size
     - λ: variable tabu size
     - n: nb vertices
    */
    pub fn new(l:usize, lambda: f64, n:usize) -> Self {
        Self {
            l, lambda,
            nb_iter: 0,
            decisions: vec![None ; n],
            rng: rand::thread_rng(),
        }
    }

    // pub fn reset(&mut self) {
    //     self.nb_iter += self.decisions.len()+1;
    // }
}



/** Implementation of a swap tabu search procedure.
Starts by an initial clique. Try to insert a vertex that is not in the clique. Possibly removes
some existing vertex. Break ties by choosing the vertex with the highest degree
*/
pub fn clique_swaps(inst:Rc<dyn ColoringInstance>, sol:Vec<VertexId>, nb_max_iter:usize, show_completion:bool) -> Vec<VertexId> {
    println!("starting with {}", sol.len());
    let mut current_clique = sol.clone();
    let mut best = current_clique.clone();
    let mut inside_clique = BitSet::new();
    for u in &current_clique { inside_clique.insert(*u); }
    // maintains clique degrees
    // let mut sum_clique_degrees:usize = res.iter().map(|u| inst.degree(*u)).sum();
    // for each vertex, maintain the number of vertices in the clique it sees
    let mut nb_clique_see:Vec<i64> = vec![0 ; inst.nb_vertices()];
    for u in sol {
        for v in inst.neighbors(u) {
            nb_clique_see[v] += 1;
        }
    }
    let mut nb_iter = 0;
    let mut tabu = CliqueSwapTenure::new(inst.nb_vertices()/5, 0.5, inst.nb_vertices());
    // println!("sum_clique:{}", sum_clique_degrees);
    // println!("nb_clique_see:{:?}", nb_clique_see);
    loop {
        nb_iter += 1;
        if nb_iter >= nb_max_iter { break; }
        if show_completion && nb_iter % 1000 == 0 { println!(" {} \t {}/{}", best.len(), nb_iter, nb_max_iter); }
        // search for the vertex that sees the maximum elements in the clique
        let u = match inst.vertices()
            // .filter(|u| !inside_clique.contains(*u) && !tabu.contains(&0, u))
            // only consider non added vertices and non-tabu (with a simple aspiration criterion)
            .filter(|u| !inside_clique.contains(*u) && (nb_clique_see[*u] as usize == current_clique.len() || !tabu.contains(&0, u)))
            .filter(|u| !inst.is_dominated(*u) && inst.degree(*u) > best.len()) // do not take dominated vertices
            .max_by(|u,v| nb_clique_see[*u].cmp(&nb_clique_see[*v])
                .then_with(|| inst.degree(*u).cmp(&inst.degree(*v)))
            ){
                None => { break; }
                Some(v) => { v }
        };
        // remove vertices in the clique that do not see u
        let to_remove:Vec<VertexId> = current_clique.iter().filter(|v| !inst.are_adjacent(u, **v))
            .cloned().collect();
        // println!("inserting {}\t removing {:?}", u, to_remove);
        // perform move (adding u)
        inside_clique.insert(u);
        current_clique.push(u);
        tabu.insert(&0, u);
        for v in inst.neighbors(u) {
            nb_clique_see[v] += 1;
        }
        // perform move (remove v ∈ to_remove)
        for v in &to_remove {
            inside_clique.remove(*v);
            for w in inst.neighbors(*v) {
                nb_clique_see[w] -= 1;
            }
        }
        current_clique = current_clique.iter().filter(|v| inside_clique.contains(**v)).cloned().collect();
        if to_remove.is_empty() && current_clique.len() > best.len() {
            best = current_clique.clone();
            println!("new best clique! ({})", best.len());
            // tabu.reset(); // reset the tabu list
            for a in best.iter() {
                for b in best.iter() {
                    if a < b {
                        assert!(inst.are_adjacent(*a, *b));
                    }
                }
            }
        }
        // break;
    }
    best
}


#[cfg(test)]
mod tests {

    use super::*;

    use crate::cgshop::CGSHOPInstance;
    // use crate::search::clique_bnb::adhoc_greedy_clique;
    use crate::search::greedy_clique::greedy_clique;

    #[test]
    fn test_run() {
        let inst = Rc::new(CGSHOPInstance::from_file(
            // "./insts/cgshop22/reecn3382.instance.json"
            // "./insts/cgshop22/reecn15355.instance.json"
            // "./insts/cgshop22/vispecn19370.instance.json"
            "./insts/cgshop22/rvispecn13421.instance.json"
            // "./insts/cgshop22/visp26405.instance.json"
            // "./insts/cgshop22/sqrp15532.instance.json"
            // "./insts/cgshop22/rvisp24116.instance.json"
            // "./insts/cgshop22/rsqrp24641.instance.json"
            // "./insts/cgshop22/rsqrp23406.instance.json"
            // "./insts/cgshop_22_examples/tiny10.instance.json"
        ));
        let sol = greedy_clique(inst.clone());
        clique_swaps(inst.clone(), sol, inst.nb_vertices(), true);
    }
}