/*
Implements:
 - procedures to read and write CGSHOP instance and solution formats
 - procedures to produce an instance from a CGSHOP instance and vice-versa
*/
use std::fs;
use std::fs::File;
use std::io::{BufReader, Write};
use std::cmp::{max, min};

use serde::{Serialize, Deserialize};
use serde_json::json;

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
    /// preprocessed coordinates
    #[serde(skip)]
    coordinates: Vec<((i64,i64),(i64,i64))>,

}


impl ColoringInstance for CGSHOPInstance {
    fn nb_vertices(&self) -> usize { self.m }

    fn degree(&self, u:VertexId) -> usize { self.degrees[u] }

    fn neighbors(&self, u:VertexId) -> Vec<VertexId> {
        (0..self.m()).filter(move |v| *v != u)
            .filter(|v| self.are_adjacent(u, *v)).collect()
    }

    fn are_adjacent(&self, u:VertexId, v:VertexId) -> bool {
        are_intersecting(&self.coordinates[u], &self.coordinates[v])
    }

    fn display_statistics(&self) {
        println!("{:<10} vertices", self.n());
        println!("{:<10} segments", self.m());
        // println!("\tdegrees: {:?}", self.degrees);
    }

    fn write_solution(&self, filename:&str, solution:&[Vec<usize>]) {
        CGSHOPSolution::from_solution(self.id(), solution).to_file(filename);
    }
}


impl CGSHOPInstance {
    /** reads a CGSHOP instance from a file. */
    pub fn from_file(filename:&str, should_compute_degrees:bool) -> Self {
        let str = fs::read_to_string(filename)
            .expect("Error while reading the file...");
        let mut res:Self = serde_json::from_str(&str)
            .expect("Error while deserializing the json file");
        // computing coordinates cache
        res.coordinates = (0..res.m()).map(|s| res.build_coordinates(s)).collect();
        if should_compute_degrees {
            res.compute_degrees();
        }
        res
    }

    /** converts to a graph coloring instance. */
    pub fn to_graph_coloring_instance(&self) -> CompactInstance {
        let nb_vertices = self.m();
        let mut adj_list:Vec<Vec<usize>> = vec![vec![] ; nb_vertices];
        for i in 0..nb_vertices {
            for j in 0..i {
                if self.are_adjacent(i, j) {
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

    /// edge coordinate for segment i (x1,y1,x2,y2)
    pub fn build_coordinates(&self, i:usize) -> ((i64,i64),(i64,i64)) {
        (
            (self.x[self.edge_i[i]] as i64, self.y[self.edge_i[i]] as i64),
            (self.x[self.edge_j[i]] as i64, self.y[self.edge_j[i]] as i64),
        )
    }

    /// computes the degrees for each edge
    fn compute_degrees(&mut self) {
        let cache_filename = format!("tmp/{}.degree.cache.json", self.id());
        if let Ok(str) = fs::read_to_string(&cache_filename) {
            self.degrees = serde_json::from_str(&str)
                .expect("Error while deserializing the json file");
            println!("reusing the cached degrees.");
            return;
        }
        println!("CGSHOP Instance: computing degrees...");
        let n = self.nb_vertices();
        let mut degrees:Vec<usize> = vec![0 ; n];
        for i in 0..n {
            let mut current_neighbors = Vec::new();
            if i % 1000 == 0 { println!("computed degrees for {} / {}...", i, n); }
            for j in 0..i {
                if self.are_adjacent(i, j) {
                    current_neighbors.push(j);
                    degrees[i] += 1;
                    degrees[j] += 1;
                }
            }
        }
        self.degrees = degrees;
        // write cache 
        let mut new_cache_file = File::create(&cache_filename)
            .expect("CGHSOP Instance cache: unable to open the file");
        let degree_cache_value = json!(self.degrees);
        new_cache_file.write_all(serde_json::to_string(&degree_cache_value).unwrap().as_bytes())
            .expect("CGHSOPSolution.to_file: unable to write in the file");
    }
}


/** 3 point orientation (either collinear, clockwise or counterclockwise) */
#[derive(Debug,Eq,PartialEq)]
enum Orientation {
    Collinear,
    Clockwise,
    CounterClockwise,
}

/** returns:
 - 0 if p,q,r are colinear
 - 1 if clockwise orientation
 - 2 if counterclockwise orientation
*/
fn orientation(p:&(i64,i64), q:&(i64,i64), r:&(i64,i64)) -> Orientation {
    let val:i64 = (q.1 - p.1) * (r.0 - q.0) - (q.0 - p.0) * (r.1 - q.1);
    if val == 0 { return Orientation::Collinear; }
    match val > 0 {
        true => Orientation::Clockwise,
        false => Orientation::CounterClockwise
    }
}

fn on_segment(p:&(i64,i64), q:&(i64,i64), r:&(i64,i64)) -> bool {
    if q.0 <= max(p.0, r.0) && q.0 >= min(p.0, r.0) && 
        q.1 <= max(p.1, r.1) && q.1 >= min(p.1, r.1) {
            return true;
    }
    false
}

/** returns true iff segments (p1,q1) and (p2,q2) intersect */
fn are_intersecting((p1,q1):&((i64,i64),(i64,i64)), (p2,q2):&((i64,i64),(i64,i64))) -> bool {
    if p1 == p2 || p1 == q2 || q1 == p2 || q1 == q2 { return false; } // accept end points that are the same
    let o1 = orientation(p1,q1,p2);
    let o2 = orientation(p1,q1,q2);
    let o3 = orientation(p2,q2,p1);
    let o4 = orientation(p2,q2,q1);
    if o1 != o2 && o3 != o4 { return true; }
    if o1 == Orientation::Collinear && on_segment(p1,p2,q1) { return true; }
    if o2 == Orientation::Collinear && on_segment(p1,q2,q1) { return true; }
    if o3 == Orientation::Collinear && on_segment(p2,p1,q2) { return true; }
    if o4 == Orientation::Collinear && on_segment(p2,q1,q2) { return true; }
    false
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
    pub fn to_file(&self, filename:&str) {
        let res_str = serde_json::to_string(self).unwrap();
        let mut file = File::create(filename)
            .expect("CGHSOPSolution.to_file: unable to open the file");
        file.write_all(res_str.as_bytes())
            .expect("CGHSOPSolution.to_file: unable to write in the file");
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_are_intersecting_1() {
        let a = ((1,1),(10,1));
        let b = ((1,2),(10,2));
        assert!(!are_intersecting(&a, &b));
    }

    #[test]
    fn test_are_intersecting_2() {
        let a = ((10,0),(0,10));
        let b = ((0,0),(10,10));
        assert!(are_intersecting(&a, &b));
    }

    #[test]
    fn test_are_intersecting_3() {
        let a = ((-5,-4),(0,0));
        let b = ((1,1),(10,10));
        assert!(!are_intersecting(&a, &b));
    }


    #[test]
    fn test_are_intersecting_4() {
        let a = ((0,0),(0,5));
        let b = ((0,0),(5,0));
        assert!(!are_intersecting(&a, &b));
    }

    #[test]
    fn test_read_tiny() {
        let cg_inst = CGSHOPInstance::from_file(
            "./insts/CGSHOP_22_original/cgshop_2022_examples_01/tiny.json",
            true
        );
        cg_inst.display_statistics();
    }

    #[test]
    fn test_read_instance() {
        let cg_inst = CGSHOPInstance::from_file(
            "./insts/CGSHOP_22_original/cgshop_2022_examples_01/example_instances_visp/visp_5K.instance.json",
            true
        );
        cg_inst.display_statistics();
        let inst = cg_inst.to_graph_coloring_instance();
        inst.display_statistics();
    }

    #[test]
    fn test_read_instance_sqrm() {
        let cg_inst = CGSHOPInstance::from_file(
            "./insts/CGSHOP_22_original/cgshop_2022_examples_01/example-instances-sqrm/sqrm_5K_1.instance.json",
            true
        );
        cg_inst.display_statistics();
        let inst = cg_inst.to_graph_coloring_instance();
        inst.display_statistics();
    }
}
