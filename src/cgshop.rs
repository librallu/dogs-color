/*
Implements:
 - procedures to read and write CGSHOP instance and solution formats
 - procedures to produce an instance from a CGSHOP instance and vice-versa
*/
use std::fs;
use std::fs::File;
use std::io::{BufReader, Write};

use serde::{Serialize, Deserialize};
use geo::{Coordinate, Line};
use geo::algorithm::line_intersection::line_intersection;

use crate::color::{VertexId, ColoringInstance};
use crate::compact_instance::CompactInstance;

/** data structure to represent a CGSHOP instance */
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CGSHOPInstance {
    /// number of points
    n: usize,
    /// number of edges
    m: usize,
    /// x coordinates for points
    x: Vec<f64>,
    /// y coordinates for points
    y: Vec<f64>,
    /// edge_i[i]: first endpoint of the ith edge
    edge_i: Vec<usize>,
    /// edge_j[i]: second endpoint of the ith edge
    edge_j: Vec<usize>,
    /// identifier of the instance
    id: String,
    /// degrees if they are computed of each segment
    #[serde(skip)]
    degrees: Vec<usize>,
}


impl ColoringInstance for CGSHOPInstance {
    fn nb_vertices(&self) -> usize { self.m }

    fn degree(&self, u:VertexId) -> usize { self.degrees[u] }

    fn neighbors(&self, u:VertexId) -> Vec<VertexId> {
        (0..self.m()).filter(move |v| *v != u)
            .filter(|v| self.are_adjacent(u, *v)).collect()
    }

    fn are_adjacent(&self, u:VertexId, v:VertexId) -> bool {
        is_intersection(&self.coordinates(u), &self.coordinates(v))
    }
}


impl CGSHOPInstance {
    /** reads a CGSHOP instance from a file. */
    pub fn from_file(filename:&str) -> Self {
        let str = fs::read_to_string(filename)
            .expect("Error while reading the file...");
        let mut res:Self = serde_json::from_str(&str)
            .expect("Error while deserializing the json file");
        res.compute_degrees();
        res
    }

    /** converts to a graph coloring instance. */
    pub fn to_graph_coloring_instance(&self) -> CompactInstance {
        let nb_vertices = self.m();
        let edge_coordinates = self.edge_coordinates();
        let mut adj_list:Vec<Vec<usize>> = vec![vec![] ; nb_vertices];
        for i in 0..nb_vertices {
            for j in 0..i {
                if is_intersection(&edge_coordinates[i], &edge_coordinates[j]){
                    adj_list[i].push(j);
                    adj_list[j].push(i);
                }
            }
        }
        CompactInstance::new(adj_list)
    }

    /// number of vertices
    pub fn n(&self) -> usize { self.n }

    /// number of edges
    pub fn m(&self) -> usize { self.m }

    /// instance id
    pub fn id(&self) -> String { self.id.clone() }

    /// squared length of a segment
    pub fn squared_length(&self, i:usize) -> f64 {
        let dx = self.x[self.edge_j[i]] - self.x[self.edge_i[i]];
        let dy = self.y[self.edge_j[i]] - self.y[self.edge_i[i]];
        dx*dx + dy*dy
    }

    /// edge coordinates (x1,y1,x2,y2)
    pub fn edge_coordinates(&self) -> Vec<(f64,f64,f64,f64)> {
        (0..self.edge_i.len()).map(|i| {
            self.coordinates(i)
        }).collect()
    }

    /// edge coordinate for segment i (x1,y1,x2,y2)
    pub fn coordinates(&self, i:usize) -> (f64,f64,f64,f64) {
        (
            self.x[self.edge_i[i]],
            self.y[self.edge_i[i]],
            self.x[self.edge_j[i]],
            self.y[self.edge_j[i]],
        )
    }

    /** displays some statistics of the instance */
    pub fn display_statistics(&self) {
        println!("\t{:>25}{:>10}", "nb vertices:", self.n());
        println!("\t{:>25}{:>10}", "nb edges:",    self.m());
    }

    /// computes the degrees for each edge
    fn compute_degrees(&mut self) {
        let mut degrees:Vec<usize> = vec![0 ; self.nb_vertices()];
        for i in 0..self.nb_vertices() {
            if i % 1000 == 0 { println!("computed degrees for {} / {}...", i, self.nb_vertices()); }
            for j in 0..i {
                if self.are_adjacent(i, j) {
                    degrees[i] += 1;
                    degrees[j] += 1;
                }
            }
        }
        self.degrees = degrees;
        println!("CGSHOP Instance: finished computing degrees.")
    }
}

/**
    true iff the lines defined by (x1,y1), (x2,y2) intersect
    There exists an intersection if and only if:
        - Collinear, and proper intersection (not at end points)
*/
pub fn is_intersection(a:&(f64,f64,f64,f64), b:&(f64,f64,f64,f64)) -> bool {
    let l1 = Line::new(
        Coordinate {x:a.0, y:a.1},
        Coordinate {x:a.2, y:a.3}
    );
    let l2 = Line::new(
        Coordinate {x:b.0, y:b.1},
        Coordinate {x:b.2, y:b.3}
    );
    match line_intersection(l1,l2) {
        Some(intersection) => match intersection {
            geo::line_intersection::LineIntersection::SinglePoint
                {intersection:_, is_proper } => { is_proper },
            geo::line_intersection::LineIntersection::Collinear { intersection:_ } => true,
        },
        None => false,
    }
}


/** data structure to represent a CGSHOP solution */
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CGSHOPSolution {
    /// solution type (should be "Solution_CGSHOP2022")
    #[serde(rename="type")]
    sol_type: String,
    /// instance name
    instance: String,
    /// number of colors
    num_colors: usize,
    /// color list (color[e]: color of edge e)
    colors: Vec<usize>,
}

impl CGSHOPSolution {
    /// creates a solution from a file, given a number of colors and the assignemnt
    pub fn new(instance:String, num_colors: usize, colors: Vec<usize>) -> Self {
        Self {
            sol_type: "Solution_CGSHOP2022".to_string(),
            instance, num_colors, colors,
        }
    }

    /// creates a solution from a solution
    pub fn from_solution(instance:String, solution:&[Vec<VertexId>]) -> Self {
        let nb_colors  = solution.len();
        let n = solution.iter().map(|c| c.len()).sum();
        let mut colors:Vec<usize> = vec![0 ; n];
        for (i,c) in solution.iter().enumerate() {
            for v in c {
                colors[*v] = i;
            }
        }
        Self::new(instance, nb_colors, colors)
    }

    /// reads a solution from a file
    pub fn from_file(filename:&str) -> Self {
        let file = File::open(filename)
            .expect("CGHSOPSolution.from_file: unable to open the file");
        let reader = BufReader::new(file);
        serde_json::from_reader(reader)
            .expect("CGHSOPSolution.from_file: unable to serialize")
    }

    /// writes the solution to a file
    pub fn to_file(&self, filename_prefix:&str) {
        let res_str = serde_json::to_string(self).unwrap();
        let mut file = File::create(
            format!("{}{}.sol.json", filename_prefix, self.instance.as_str())
        ).expect("CGHSOPSolution.to_file: unable to open the file");
        file.write_all(res_str.as_bytes())
            .expect("CGHSOPSolution.to_file: unable to write in the file");
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_conflict() {
        let l1 = (42146., 64522., 63387., 19658.);
        let l2 = (66944., 32411., 42137., 48996.);
        assert!(is_intersection(&l1, &l2));
    }

    #[test]
    fn test_read_tiny() {
        let cg_inst = CGSHOPInstance::from_file(
            "./insts/CGSHOP_22_original/cgshop_2022_examples_01/tiny.json"
        );
        cg_inst.display_statistics();
        assert_eq!(cg_inst.coordinates(0), (60941.,77185.,  42146.,64522.));
    }

    #[test]
    fn test_read_instance() {
        let cg_inst = CGSHOPInstance::from_file(
            "./insts/CGSHOP_22_original/cgshop_2022_examples_01/example_instances_visp/visp_5K.instance.json"
        );
        cg_inst.display_statistics();
        let inst = cg_inst.to_graph_coloring_instance();
        inst.display_statistics();
    }

    #[test]
    fn test_read_instance_sqrm() {
        let cg_inst = CGSHOPInstance::from_file(
            "./insts/CGSHOP_22_original/cgshop_2022_examples_01/example-instances-sqrm/sqrm_5K_1.instance.json"
        );
        cg_inst.display_statistics();
        let inst = cg_inst.to_graph_coloring_instance();
        inst.display_statistics();
    }
}
