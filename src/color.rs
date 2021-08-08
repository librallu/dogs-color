use std::fs;
use bit_set::BitSet;

use crate::dimacs::{skip_comments, read_edge, read_header};

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
        let s1 = fs::read_to_string(filename)
            .expect("Instance: unable to read file").replace("\r","");
        let s2 = skip_comments(s1.as_str()).unwrap().0;
        let (mut s3,(n,m)) = read_header(s2).unwrap();
        let mut adj_list = vec![Vec::new();n];
        let mut check_nb_edges = 0;
        while match read_edge(s3) {
            Ok((tmp,(a,b))) => {
                s3 = tmp;
                adj_list[a-1].push(b-1);
                adj_list[b-1].push(a-1);
                check_nb_edges += 1;
                true
            }
            Err(_) => false
        } {}
        assert!(
            check_nb_edges == m || 2*check_nb_edges == m,
            "check: {}\t m: {}", check_nb_edges, m
        );
        Self { n, m, adj_list }
    }

    /// print statistics of the instance
    pub fn print_stats(&self) {
        println!("\t{} \t vertices", self.n());
        println!("\t{} \t edges", self.m());
        let degrees:Vec<usize> = (0..self.n()).map(|i| {
            self.adj(i).len()
        }).collect();
        println!("\t{} \t min degree", degrees.iter().min().unwrap());
        println!("\t{} \t min degree", degrees.iter().max().unwrap());
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
            let mut adj_v1 = BitSet::new();
            for neigh in inst.adj(*v1) {
                adj_v1.insert(*neigh);
            }
            for v2 in c {
                if adj_v1.contains(*v2) {
                    return None;
                }
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