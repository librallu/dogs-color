use bit_set::BitSet;

use crate::dimacs::{read_from_file};

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
    /// edges of the graph
    edges: Vec<(VertexId,VertexId)>,
    /// adj_list[i]: list of vertices adjacent to i
    adj_list: Vec<Vec<VertexId>>,
    /// if exists: adj_matrix[i] represents a bitset of its neighbors
    adj_matrix: Option<Vec<BitSet>>,
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

    /// edge list
    pub fn edges(&self) -> &[(VertexId, VertexId)] {
        &self.edges
    }

    /// builds the edge list
    pub fn build_edges(adj_list:&[Vec<VertexId>]) -> Vec<(VertexId,VertexId)> {
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

    /// print statistics of the instance
    pub fn display_statistics(&self) {
        println!("\t{} \t vertices", self.n());
        println!("\t{} \t edges", self.m());
        let degrees:Vec<usize> = (0..self.n()).map(|i| {
            self.adj(i).len()
        }).collect();
        println!("\t{} \t min degree", degrees.iter().min().unwrap());
        println!("\t{} \t max degree", degrees.iter().max().unwrap());
        match self.adj_matrix {
            None => {},
            Some(_) => println!("\tadj matrix computed")
        }
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

    /** returns if a and b are adjacent
    if the adjacency matrix is defined: O(1)
    otherwise: O(Δ(G))
    */
    pub fn are_adjacent(&self, a:VertexId, b:VertexId) -> bool {
        match &self.adj_matrix {
            None => {
                self.adj(a).iter().any(|c| &b==c)
            },
            Some(matrix) => { matrix[a].contains(b) }
        }
    }
}

/**
returns None if the solution is infeasible
returns the objective if the solution is feasible
*/
pub fn checker(inst:&Instance, sol:&[Vec<VertexId>]) -> Option<usize> {
    // check that all vertices are added
    let mut visited = BitSet::new();
    for c in sol {
        for v in c {
            if visited.contains(*v) {
                return None;  // already added
            }
            visited.insert(*v);
        }
    }
    if visited.len() != inst.n() {
        return None;
    }
    // check conflicts
    for c in sol {
        for v1 in c {
            for v2 in c {
                if inst.are_adjacent(*v1, *v2) { return None }
            }
        }
    }
    // if ok: return the number of colors
    Some(sol.len())
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_instance() {
        let inst = Instance::from_file("insts/grid-instances/grid2x2");
        // println!("{:?}", inst);
        assert_eq!(inst.n(), 4);
        assert_eq!(inst.m(), 4);
        assert_eq!(inst.adj(0), &[1,2]);
    }

}