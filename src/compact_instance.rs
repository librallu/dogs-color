use bit_set::BitSet;
use std::fs;

use crate::dimacs::read_from_file;
use crate::color::{ColoringInstance, VertexId};

/** models a Graph Coloring instance.  */
#[derive(Debug)]
pub struct CompactInstance {
    /// nb vertices
    n: usize,
    /// nb edges
    m: usize,
    /// edges of the graph
    edges: Vec<(VertexId,VertexId)>,
    /// adj_list[i]: list of vertices adjacent to i
    adj_list: Vec<Vec<VertexId>>,
    /// if exists: adj_matrix[i] represents a bitset of its neighbors
    adj_matrix: Option<Vec<BitSet>>,
}

impl ColoringInstance for CompactInstance {
    fn nb_vertices(&self) -> usize { self.n }

    fn neighbors(&self, u:VertexId) -> Vec<VertexId> { self.adj_list[u].clone() }
    
    fn degree(&self, u:VertexId) -> usize { self.adj_list[u].len() }

    fn are_adjacent(&self, u:VertexId, v:VertexId) -> bool {
        match &self.adj_matrix { // if the matrix representation does not exist, iterate over
            None => { self.adj_list[u].iter().any(|c| &v==c) },
            Some(matrix) => { matrix[u].contains(v) } // otherwise, use it
        }
    }

    fn edges(&self) -> &[(VertexId, VertexId)] { &self.edges }

    fn display_statistics(&self) {
        println!("\t{} \t vertices", self.nb_vertices());
        println!("\t{} \t edges", self.nb_edges());
        let degrees:Vec<usize> = (0..self.nb_vertices()).map(|i|{ self.degree(i) }).collect();
        println!("\t{} \t min degree", degrees.iter().min().unwrap());
        println!("\t{} \t max degree", degrees.iter().max().unwrap());
        match self.adj_matrix {
            None => {},
            Some(_) => println!("\tadj matrix computed")
        }
    }

    /** writes a solution into a file. each line corresponds to a color. */
    fn write_solution(&self, filename:&str, solution:&[Vec<usize>]) {
        fs::write(filename, self.solution_to_string(solution))
            .unwrap_or_else(|_|
                panic!("write_solution: unable to write the solution in {}", filename)
            );
    }
}


impl CompactInstance {

    /// returns the number of edges in the graph
    pub fn nb_edges(&self) -> usize { self.m }

    /// builds the edge list
    fn build_edges(adj_list:&[Vec<VertexId>]) -> Vec<(VertexId,VertexId)> {
        let mut res = Vec::new();
        for (i,l) in adj_list.iter().enumerate() {
            for j in l {
                if i < *j {
                    res.push((i,*j));
                }
            }
        }
        res
    }

    /** constructor using an adjacency list */
    pub fn new(adj_list:Vec<Vec<usize>>) -> Self {
        let n = adj_list.len();
        // compute nb edges
        let mut m = 0;
        for e in &adj_list { // at the end: m = ∑ d(v)
            m += e.len();
        }
        m /= 2; // m = (∑ d(v)) / 2
        let edges = Self::build_edges(&adj_list);
        Self { n,m, edges, adj_list, adj_matrix:None }
    }

    /// creates an instance from a DIMACS file
    pub fn from_file(filename:&str) -> Self {
        let (_,_,adj_list) = read_from_file(filename);
        Self::new(adj_list)
    }

    /// if called, populate the adj_matrix
    pub fn populate_adj_matrix(&mut self) {
        let mut res = vec![BitSet::default(); self.n];
        for (a,resa) in res.iter_mut().enumerate() {
            for b in &self.adj_list[a] {
                resa.insert(*b);
            }
        }
        self.adj_matrix = Some(res);
    }

    /** writes a string encoding the solution (use this to export the solution) */
    pub fn solution_to_string(&self, solution:&[Vec<usize>]) -> String {
        let mut res = String::default();
        for e in solution {
            for v in e {
                res += format!("{} ", v).as_str();
            }
            res += "\n";
        } 
        res
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_instance() {
        let inst = CompactInstance::from_file("insts/grid-instances/grid2x2");
        // println!("{:?}", inst);
        assert_eq!(inst.nb_vertices(), 4);
        assert_eq!(inst.nb_edges(), 4);
    }

}