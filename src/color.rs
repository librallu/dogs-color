use std::fs;

use nom::IResult;

/** Color Id */
pub type ColorId = usize;

/** Vertex Id */
pub type VertexId = usize;

/** Solution of a graph coloring problem
(represented as a partition).
*/
pub type Solution = Vec<Vec<VertexId>>;

/** models a Graph Coloring instance */
#[derive(Debug)]
pub struct Instance {
    /// nb vertices
    n: usize,
    /// nb edges
    m: usize,
    /// adj_list[i]: list of vertices adjacent to i
    adj_list: Vec<Vec<VertexId>>,
}


impl Instance {

    /// number of vertices
    pub fn n(&self) -> usize { self.n }

    /// number of edges
    pub fn m(&self) -> usize { self.m }

    /// list of vertices adjacent to vertex i
    pub fn adj(&self, i:VertexId) -> &Vec<VertexId> {
        &self.adj_list[i]
    }

    /// creates an instance from a DIMACS file
    pub fn from_file(filename:&str) -> Self {
        Self {
            n:0,
            m:0,
            adj_list:Vec::new(),
        }
    }

}