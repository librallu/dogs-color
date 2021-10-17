use std::rc::Rc;

use bit_set::BitSet;

use dogs::{search_algorithm::StoppingCriterion, search_space::{ToSolution}};

use crate::color::{ColoringInstance, Solution, VertexId};


// impl Ord for VertexOrderingInfo {
//     fn cmp(&self, other: &Self) -> Ordering {
//         self.dsat.cmp(&other.dsat)
//             .then_with(|| self.d.cmp(&other.d))
//             .then_with(|| self.v.cmp(&other.v))
//     }
// }

// // `PartialOrd` needs to be implemented as well.
// impl PartialOrd for VertexOrderingInfo {
//     fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
//         Some(self.cmp(other))
//     }
// }



/** represents a node structure */
#[derive(Debug, Clone)]
pub struct Node {
    /// number of colored nodes
    nb_colored: usize,
}

/** represents a decision (assigning color c to vertex v) */
#[derive(Debug, Clone)]
struct Decision {
    v: VertexId,
    c: usize,
}

/// either a decision, either a commit
#[derive(Debug)]
enum BacktrackEvent {
    Commit(Decision), // decision to commit
    Restore(Decision) // decision taken
}


/**
Implements a backtracking search space for DSATUR.
*/
#[derive(Debug)]
pub struct BacktrackingDsaturSpace {
    /// instance
    inst: Rc<dyn ColoringInstance>,
    /// colored: set of colored vertices
    uncolored: BitSet,
    /// dsat[v]: saturation degree of vertex v
    dsat: Vec<usize>,
    /// colors[i]: color assigned to vertex i
    colors: Vec<Option<usize>>,
    /// number of colors in the search state
    nb_colors: usize,
    /// nb_adj_colored[v][c]: number of vertices adjacent to v colored with c
    nb_adj_colored: Vec<Vec<usize>>,
    /// color_nb_vertices[c]: number of vertices using color c
    color_nb_vertices: Vec<usize>,
    /// upper bound on the number of colors
    upper_bound: usize,
    /// decision history
    decisions: Vec<BacktrackEvent>,
    /// number of colored vertices
    nb_vertices_colored: usize,
    /// best so far coloring
    best_so_far_coloring: Option<Vec<usize>>,
}


impl BacktrackingDsaturSpace {
    /** creates a new backtracking Dsatur search space */
    pub fn new(inst:Rc<dyn ColoringInstance>, initial_clique:&[VertexId], upper_bound:usize) -> Self {
        let n = inst.nb_vertices();
        let mut uncolored = BitSet::default();
        let mut dsat = vec![0 ; n];
        let mut colors = vec![None ; n];
        let mut nb_colors = 0;
        let mut color_nb_vertices = vec![0 ; upper_bound];
        let mut clique_vertices:BitSet = BitSet::default();
        for (i,v) in initial_clique.iter().enumerate() { // color clique vertices
            colors[*v] = Some(i);
            nb_colors += 1;
            color_nb_vertices[i] += 1;
            clique_vertices.insert(*v);
        }
        for i in inst.vertices() {
            if !clique_vertices.contains(i) { uncolored.insert(i); }
        }
        let mut nb_adj_colored = vec![vec![0 ; upper_bound-1] ; n];
        for v in inst.vertices() {
            for w in initial_clique {
                if inst.are_adjacent(v, *w) {
                    dsat[v] += 1;
                    nb_adj_colored[*w][colors[*w].unwrap()] += 1;
                }
            }
        }
        // build the search space
        Self {
            inst,
            uncolored,
            dsat,
            colors,
            nb_colors,
            nb_adj_colored,
            upper_bound,
            decisions: Vec::with_capacity(n),
            color_nb_vertices,
            nb_vertices_colored: initial_clique.len(),
            best_so_far_coloring: None,
        }
    }

    /// finds the next vertex to color (maximum saturation degree, break ties by degree)
    fn next_vertex(&self) -> Option<VertexId> {
        self.uncolored.iter().max_by(|a,b| {
            self.dsat[*a].cmp(&self.dsat[*b])
                .then_with(|| self.inst.degree(*a).cmp(&self.inst.degree(*b)))
                .then_with(|| a.cmp(b))
        })
    }

    /// returns the next vertex and possible colorings for it.
    /// removes the corresponding vertexId.
    fn next_decisions(&self) -> Option<(VertexId, Vec<usize>)> {
        if self.nb_colors == self.inst.nb_vertices() { return None; }
        // find the next best vertex
        let v = self.next_vertex().unwrap();
        // compute candidate colors (non-adjacent colors)
        let mut candidate_colors:Vec<usize> = (0..self.nb_colors)
            .filter(|c| self.nb_adj_colored[v][*c] == 0).collect();
        // if possible, color the vertex with a new color)
        if self.nb_colors < self.upper_bound-1 {
            candidate_colors.push(self.nb_colors);
        }
        Some((v, candidate_colors))
    }

    /// applies a decision to the search space
    fn commit(&mut self, decision:Decision) {
        // println!("commit: {:?}", decision);
        self.color_nb_vertices[decision.c] += 1;
        self.nb_vertices_colored += 1;
        if decision.c == self.nb_colors {
            self.nb_colors += 1;
        }
        debug_assert!(self.colors[decision.v].is_none());
        self.colors[decision.v] = Some(decision.c);
        for u in self.inst.neighbors(decision.v) {
            self.nb_adj_colored[u][decision.c] += 1;
            // update dsat value of u
            if self.nb_adj_colored[u][decision.c] == 1 {
                self.dsat[u] += 1;
            }
        }
        self.uncolored.remove(decision.v);
    }

    /// restores the search space from a decision (does not pop self.decisions)
    fn restore(&mut self, decision:Decision) {
        // println!("restore: {:?}", decision);
        self.color_nb_vertices[decision.c] -= 1;
        self.nb_vertices_colored -= 1;
        // change nb_colors if the last color is removed (c should be the last color)
        if self.color_nb_vertices[decision.c] == 0 {
            debug_assert_eq!(decision.c, self.nb_colors-1);
            self.nb_colors -= 1;
        }
        debug_assert!(!self.colors[decision.v].is_none());
        self.colors[decision.v] = None;
        for u in self.inst.neighbors(decision.v) {
            self.nb_adj_colored[u][decision.c] -= 1;
            if self.nb_adj_colored[u][decision.c] == 0 {
                self.dsat[u] -= 1;
            }
        }
        self.uncolored.insert(decision.v);
    }

    fn push_next_decisions(&mut self) {
        let (v, colors) = self.next_decisions().unwrap();
        for c in colors.iter().rev() {
            let decision = Decision { v, c:*c };
            self.decisions.push(BacktrackEvent::Restore(decision.clone())); // prepare to backtrack
            self.decisions.push(BacktrackEvent::Commit(decision)); // decision to apply
        }
    }

    /// backtracking search
    /// 
    /// Stores the decisions to be taken in a queue.
    /// pops each decision, try to apply it
    /// when all vertices are assigned, report a new solution
    pub fn dfs_search<Stop:StoppingCriterion>(&mut self, stopping_criterion:Stop) {
        // populate decisions with the root node children
        self.push_next_decisions();
        let mut nb_expanded:usize = 1;
        // while stopping criterion is not met, and the decisions stack is not empty
        while !stopping_criterion.is_finished() && !self.decisions.is_empty() {
            // println!("{:?}", self.decisions);
            match self.decisions.pop().unwrap() {
                BacktrackEvent::Restore(decision) => { // restore the state
                    self.restore(decision);
                },
                BacktrackEvent::Commit(decision) => { // apply the decision and generate children
                    self.commit(decision.clone());
                    if self.nb_colors < self.upper_bound { // check bound
                        // println!("{} / {} \t ({}/{})", 
                        //     self.nb_colors, self.upper_bound,
                        //     self.nb_vertices_colored, self.inst.nb_vertices()
                        // );
                        // check if feasible solution, otherwise, push next decisions
                        if self.nb_vertices_colored == self.inst.nb_vertices() {
                            println!("feasible: {} colors", self.nb_colors);
                            self.upper_bound = std::cmp::min(self.upper_bound, self.nb_colors);
                            let mut new_sol = vec![0 ; self.inst.nb_vertices()];
                            for (v,c) in self.colors.iter().enumerate() {
                                new_sol[v] = c.unwrap();
                            }
                            self.best_so_far_coloring = Some(new_sol);
                            // println!("{:?}", self.best_so_far_coloring);
                        } else if !self.decisions.is_empty() { // stop if no more events on the stack
                            self.push_next_decisions();
                            nb_expanded += 1;
                        }
                    }
                }
            }
        }
        println!("nb expanded: {}", nb_expanded);
    }

}

impl ToSolution<Node, Solution> for BacktrackingDsaturSpace {
    fn solution(&mut self, _node: &mut Node) -> Solution {
        // debug_assert!(self.goal(node));
        // build the solution (res[i]: vertices assigned color i)
        let mut res = vec![vec![]; self.nb_colors];
        for (i,color) in self.colors.iter().enumerate() {
            res[color.unwrap()].push(i);
        }
        res
    }
}

#[cfg(test)]
mod tests {
    use dogs::search_algorithm::TimeStoppingCriterion;

    use super::*;
    
    use crate::cgshop::CGSHOPInstance;
    use crate::dimacs::DimacsInstance;
    use crate::search::clique_partial_weighting::clique_partial_weighting;
    use crate::search::greedy_clique::greedy_clique;
    
    #[test]
    fn test_simple() {
        let inst = Rc::new(DimacsInstance::from_file(
            // "insts/other-instances/peterson.col"
            // "insts/other-instances/7partite2.col"
            // "insts/instances-dimacs1/DSJC125.1.col"
            // "insts/instances-dimacs1/DSJR500.1.col"
            // "insts/instances-dimacs1/le450_25a.col"
            // "insts/instances-dimacs2/david.col"
            "insts/instances-dimacs2/zeroin.i.1.col"
        ));
        let n = inst.nb_vertices();
        let mut space = BacktrackingDsaturSpace::new(inst, &[], n);
        let stop = TimeStoppingCriterion::new(30.);
        space.dfs_search(stop);
    }

    #[test]
    fn test_simple_cgshop() {
        let inst = Rc::new(CGSHOPInstance::from_file(
            // "./insts/cgshop22/vispecn2518.instance.json"
            // "./insts/cgshop22/reecn3382.instance.json"
            "./insts/cgshop22/rvisp3499.instance.json"
            // "./insts/cgshop22/rvisp14562.instance.json"
            // "./insts/cgshop22/reecn9674.instance.json"
            // "./insts/cgshop22/reecn12588.instance.json"
            // "./insts/cgshop22/reecn25913.instance.json"
            // "./insts/cgshop22/reecn31126.instance.json"
            // "./insts/cgshop22/reecn73116.instance.json"
            // "./insts/cgshop22/rvispecn17968.instance.json"
            // "./insts/cgshop22/vispecn17665.instance.json"
            // "./insts/cgshop22/vispecn13806.instance.json"
            // "./insts/cgshop22/vispecn37349.instance.json"
            // "./insts/cgshop22/visp26405.instance.json"
            // "./insts/cgshop_22_examples/visp_5K.instance.json"
            // "./insts/cgshop_22_examples/sqrm_10K_5.instance.json"
        ));
        // find a clique
        let greedy_clique_sol = greedy_clique(inst.clone());
        println!("initial clique: {}", greedy_clique_sol.len());
        let stopping_criterion:TimeStoppingCriterion = TimeStoppingCriterion::new(5.);
        let clique = clique_partial_weighting(
            inst.clone(), &greedy_clique_sol, None, None , stopping_criterion
        );
        // find a coloring
        let mut space = BacktrackingDsaturSpace::new(inst, &clique[0], 50);
        let stop = TimeStoppingCriterion::new(3000.);
        space.dfs_search(stop);
    }
}