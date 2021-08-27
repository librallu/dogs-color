use std::rc::Rc;
use std::cell::RefCell;

use rand::Rng;
use bit_set::BitSet;

use crate::color::{CompactInstance, Solution, VertexId, checker, CheckerResult};


/** PARTIALCOL algorithm (see: 10.1016/j.cor.2006.05.014)
 - Tabu search that explores a neighborhood of feasible solutions minimizinc the number of
   unassigned solutions. Selects an unassigned vertex and a color. Color this vertex with
   the color, and unassign adjacent vertices with the same color.
*/

/**
Decision of coloring the vertex v with c
*/
#[derive(Debug,Clone,Hash,Eq,PartialEq)]
pub struct Decision {
    pub v: VertexId,
    pub c: usize,
}


/**
Implements a search tree node.
Stores a decision and a number of conflicts
*/
#[derive(Debug,Clone)]
pub struct Node {
    /// decision taken by the node
    decision:Option<Decision>,
    /// number of uncolored vertices
    nb_uncolored: usize,
}


/** (see https://doi.org/10.1016/j.cor.2006.05.014)
Implements a local search procedure for the graph coloring (PARTIALCOL).
Starts with an initial solution using a DSATUR like algorithm (TODO: needs more details).
Then tries to assign vertices to colors, uncoloring conflicting vertices. 
*/
#[derive(Debug)]
pub struct SearchState {
    /// reference instance
    inst: Rc<CompactInstance>,
    /// colors[v]: color of the vertex v
    colors: Vec<Option<usize>>,
    /// number of colors used
    nb_colors: usize,
    /// nb_neigh_colors[v][c]: number of neighbors of v that are assigned color c
    nb_neigh_colors: Vec<Vec<usize>>,
}

impl SearchState {

    /** Creates a new search state by creating color class after color class.
    Time complexity: O(n.m.c)
    */
    pub fn initial_solution(inst:Rc<CompactInstance>, nb_colors:usize) -> Self {
        let mut c = 0; // current number of colors
        let mut nb_colored = 0; // number of colored vertices
        let mut colored:BitSet<u64> = BitSet::default(); // which vertex have been colored
        let mut colors:Vec<Option<usize>> = vec![None; inst.n()];
        while nb_colored < inst.n() && c <= nb_colors {
            let mut forbidden:BitSet<u64> = BitSet::default(); // set of forbidden vertices
            // create a new color
            for (i, color) in colors.iter_mut().enumerate() {  // iterate over all vertices, marking 
                if !forbidden.contains(i) && !colored.contains(i) { // if we can add vertex i
                    // color c
                    colored.insert(i);
                    *color = Some(c);
                    forbidden.insert(i);
                    nb_colored += 1;
                    for j in inst.adj(i) {  // forbid adjacent vertices to be set in the same color
                        forbidden.insert(*j);
                    }
                }
            }
            c += 1; // go to the next color
        }
        Self {
            inst,
            colors,
            nb_colors: c,
            nb_neigh_colors: Vec::new(),
        }
    }
}


#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_initial_sol() {
        let inst = Rc::new(
            CompactInstance::from_file("insts/instances-dimacs1/le450_15a.col")
        );
        let state = SearchState::initial_solution(inst, 20);
        // println!("{:?}", state);
        let nb_uncolored = state.colors.iter()
            .filter(|e| **e==None)
            .count();
        println!("{} uncolored", nb_uncolored);
    }

}