use std::rc::Rc;
use ordered_float::OrderedFloat;
use bit_set::BitSet;

use dogs::search_space::{SearchSpace, TotalNeighborGeneration, GuidedSpace, ToSolution};

use crate::color::{Instance, Solution, VertexId, checker};

/**
Implements a DSATUR tree search space.
root: vertex with maximum degree
Overall, select a node with maximum degree neighbors (break ties by degree)
*/
#[derive(Debug)]
pub struct DSATURSpace {
    inst: Rc<Instance>,
    /// vertex_ranks[v]: rank of vertex v
    vertex_ranks: Vec<usize>,
    /// ranked_vertices[i]: vertex ranked at position i
    ranked_vertices: Vec<VertexId>
}

/**
represents a node structure
*/
#[derive(Debug, Clone)]
pub struct Node {
    /// number of colored nodes
    nb_colored: usize,
    /// bitset of colored vertices
    colored: BitSet,
    /// colors[i] vertices colored by color "i"
    colors: Vec<BitSet>,
}

impl DSATURSpace {
    pub fn new(inst:Rc<Instance>) -> Self {
        let mut ranked_vertices:Vec<VertexId> = (0..inst.n()).collect();
        ranked_vertices.sort_by_key(|v| -(inst.adj(*v).len() as i64));
        let mut vertex_ranks = vec![0;inst.n()];
        for (i,v) in ranked_vertices.iter().enumerate() {
            vertex_ranks[*v] = i;
        }
        Self { inst, vertex_ranks, ranked_vertices }
    }

    pub fn add_coloring(&self, node:&Node, v:VertexId, c:usize) -> Node {
        let mut res = node.clone();
        res.colored.insert(v);
        if c < res.colors.len() {
            res.colors[c].insert(v);
        } else {
            let mut tmp = BitSet::new();
            tmp.insert(v);
            res.colors.push(tmp);
        }
        res.nb_colored += 1;
        res
    }
}

impl GuidedSpace<Node, OrderedFloat<f64>> for DSATURSpace {
    fn guide(&mut self, node: &Node) -> OrderedFloat<f64> {
        OrderedFloat(node.colors.len() as f64)
    }
}

impl ToSolution<Node, Solution> for DSATURSpace {
    fn solution(&mut self, node: &mut Node) -> Solution {
        debug_assert!(self.goal(node));
        let mut res = vec![vec![]; node.colors.len()];
        for (i,color) in node.colors.iter().enumerate() {
            for v in color {
                res[i].push(v);
            }
        }
        res
    }
}

impl SearchSpace<Node, i64> for DSATURSpace {

    fn initial(&mut self) -> Node {
        let mut colored = BitSet::default();
        let v = self.ranked_vertices[0];
        colored.insert(v);
        let colors:Vec<BitSet> = vec![colored.clone()];
        Node {
            nb_colored: 1,
            colored,
            colors,
        }
    }

    fn g_cost(&mut self, node: &Node) -> i64 { node.colors.len() as i64 }

    fn bound(&mut self, node: &Node) -> i64 { node.colors.len() as i64 }

    fn goal(&mut self, node: &Node) -> bool { node.nb_colored == self.inst.n() }

    fn handle_new_best(&mut self, mut node: Node) -> Node {
        // checks that the solution is valid (call checker)
        let sol = self.solution(&mut node);
        match checker(&self.inst, &sol) {
            Some(v) => assert_eq!(v, node.colors.len()),
            None => panic!("invalid solution.")
        }
        node
    }
}

impl TotalNeighborGeneration<Node> for DSATURSpace {
    fn neighbors(&mut self, node: &mut Node) -> Vec<Node> {
        // println!("{:?}", node);
        // find next node to color
        let mut vertices:Vec<VertexId> = (0..self.inst.n())
            .filter(|v| !node.colored.contains(*v))
            .collect();
        vertices.sort_by_key(|v| {
            let mut res:OrderedFloat<f64> = OrderedFloat(self.vertex_ranks[*v] as f64);
            res /= self.inst.n() as f64;
            // count the number of colors v is adjacent to
            for color in &node.colors {
                for neigh in self.inst.adj(*v) {
                    if color.contains(*neigh) {
                        res -= 1.;
                        break;
                    }
                }
            }
            res
        });
        // try all possible colors for the node
        let next_vertex = vertices[0];
        let mut forbidden_colors = vec![false ; node.colors.len()];
        for neigh in self.inst.adj(next_vertex) {
            for (i,color) in node.colors.iter().enumerate() {
                if color.contains(*neigh) {
                    forbidden_colors[i] = true;
                    break;
                }
            }
        }
        let mut res = Vec::new();
        for (i,b) in forbidden_colors.iter().enumerate() {
            if !b {
                res.push(self.add_coloring(node, next_vertex, i));
            }
        }
        // add new color
        res.push(self.add_coloring(node, next_vertex, node.colors.len()));
        res
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_dsatur_constructor() {
        let inst = Instance::from_file("insts/test1");
        let space = DSATURSpace::new(Rc::new(inst));
        println!("{:?}", space);
        assert_eq!(space.vertex_ranks[0], 0);
    }
}