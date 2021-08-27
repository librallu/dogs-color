use bit_set::BitSet;
use std::rc::Rc;

/** Vertex Id */
pub type VertexId = usize;

/** Solution of a graph coloring problem
(represented as a partition). */
pub type Solution = Vec<Vec<VertexId>>;


/** Represents an instance of graph coloring.
This trait allows to represent various coloring instances using an implicit graph. */
pub trait ColoringInstance {
    /// returns the number of vertices in the graph
    fn nb_vertices(&self) -> usize;

    /// number of neighbors of vertex v
    fn degree(&self, u:VertexId) -> usize;

    /// returns the neighbors of vertex u
    fn neighbors(&self, u:VertexId) -> Vec<VertexId>;

    /// returns true iff u and v are adjacent
    fn are_adjacent(&self, u:VertexId, v:VertexId) -> bool;
}


/** checker result.
Returns the solution value if correct,
otherwise, provide an explanation on why the solution is incorrect.
*/
#[derive(Clone,Debug,Eq,PartialEq)]
pub enum CheckerResult {
    /// solution is correct and provide its cost
    Ok(usize),
    /// a vertex is added twice in the solution
    VertexAddedTwice(usize),
    /// some vertex is not colored
    VertexNotColored(usize),
    /// conflicting edge
    ConflictingEdge(usize, usize),
}

/**
returns None if the solution is infeasible
returns the objective if the solution is feasible
*/
pub fn checker(inst:Rc<dyn ColoringInstance>, sol:&[Vec<VertexId>]) -> CheckerResult {
    // check that all vertices are added
    let mut visited = BitSet::new();
    for c in sol {
        for v in c {
            if visited.contains(*v) {
                return CheckerResult::VertexAddedTwice(*v);
            }
            visited.insert(*v);
        }
    }
    if visited.len() != inst.nb_vertices() {
        for v in 0..inst.nb_vertices() {
            if !visited.contains(v) {
                return CheckerResult::VertexNotColored(v);
            }
        }
        panic!("checker: internal error");
    }
    // check conflicts
    for c in sol {
        for v1 in c {
            for v2 in c {
                if inst.are_adjacent(*v1, *v2) {
                    return CheckerResult::ConflictingEdge(*v1,*v2);
                }
            }
        }
    }
    // if ok: return the number of colors
    CheckerResult::Ok(sol.len())
}