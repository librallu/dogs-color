use std::f64::consts::PI;
/*
Implements:
 - procedures to read and write CGSHOP instance and solution formats
 - procedures to produce an instance from a CGSHOP instance and vice-versa
*/
use std::fs;
use std::fs::File;
use std::io::{BufReader, Write};
use std::cmp::{max, min};
use bit_set::BitSet;
use serde::{Serialize, Deserialize};

use crate::color::{VertexId, ColoringInstance};


/// pre-processed info for the CGSHOP instance
#[derive(Clone, Debug, Serialize, Deserialize)]
struct PreprocessedData {
    /// degrees of each segment
    degrees: Vec<usize>,
    /// dominations (u dominates v)
    dominations: Vec<(VertexId,VertexId)>,
}


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
    /// meta-data
    meta: serde_json::Value,
    /// adjacency list
    #[serde(skip)]
    neighbors: Vec<BitSet>,
    /// integer coordinates
    #[serde(skip)]
    coordinates: Vec<((i64,i64),(i64,i64))>,
    /// bitset of dominated vertices
    #[serde(skip)]
    dominated:BitSet,
    /// pre-processed data
    preprocessed: Option<PreprocessedData>,
    /// best-so-far coloring
    coloring: Option<CGSHOPSolution>,
    /// best-so-far clique
    clique: Option<CGSHOPSolution>,
}


impl ColoringInstance for CGSHOPInstance {
    fn nb_vertices(&self) -> usize { self.m }

    fn degree(&self, u:VertexId) -> usize { self.preprocessed.as_ref().unwrap().degrees[u] }

    fn neighbors(&self, u:VertexId) -> Vec<VertexId> {
        self.neighbors[u].iter().collect()
    }

    fn are_adjacent(&self, u:VertexId, v:VertexId) -> bool {
        self.neighbors[u].contains(v)
    }

    fn display_statistics(&self) {
        println!("{:<10} vertices", self.n());
        println!("{:<10} segments", self.m());
        println!("{:<10} dominations", self.preprocessed.as_ref().unwrap().dominations.len());
    }

    fn write_solution(&self, filename:&str, solution:&[Vec<usize>]) {
        // TODO change solution to match preprossed segments
        CGSHOPSolution::from_solution(self.id(), solution).to_file(filename);
    }

    fn edges(&self) -> &[(VertexId, VertexId)] { todo!() }

    fn is_dominated(&self, v:VertexId) -> bool { self.dominated.contains(v) }

    fn coloring(&self) -> Option<Vec<Vec<VertexId>>> {
        match &self.coloring {
            None => None,
            Some(s) => {
                let mut res:Vec<Vec<VertexId>> = vec![vec![] ; s.num_colors];
                for (v,c) in s.colors.iter().enumerate() {
                    res[*c].push(v);
                }
                Some(res)
            }
        }
    }

    fn clique(&self) -> Option<Vec<VertexId>> {
        match &self.coloring {
            None => None,
            Some(s) => {
                let mut res:Vec<VertexId> = Vec::new();
                for (v,c) in s.colors.iter().enumerate() {
                    if *c == 0 { res.push(v); }
                }
                Some(res)
            }
        }
    }

}


impl CGSHOPInstance {
    /** reads a CGSHOP instance from a file. */
    pub fn from_file(filename:&str) -> Self {
        let str = fs::read_to_string(filename)
            .expect("Error while reading the file...");
        let mut res:Self = serde_json::from_str(&str)
            .expect("Error while deserializing the json file");
        // pre-process informations if needed
        println!("CGSHOP Instance: compute neighbors...");
        let n = res.nb_vertices();
        res.coordinates = (0..res.m()).map(|s| res.build_coordinates(s)).collect();
        res.neighbors = vec![BitSet::with_capacity(n) ; n];
        for i in 0..n {
            if i % 1000 == 0 { println!("computing neighbors ({} / {})...", i, n); }
            for j in 0..i {
                if are_intersecting(&res.coordinates[i], &res.coordinates[j]) {
                    res.neighbors[i].insert(j);
                    res.neighbors[j].insert(i);
                }
            }
        }
        // shrink bitsets
        for i in 0..n {
            res.neighbors[i].shrink_to_fit();
        }
        if res.preprocessed.is_none() {
            let degrees:Vec<usize> = (0..n).map(|i| res.neighbors[i].len()).collect();
            println!("\t{} conflicts", degrees.iter().map(|e| *e as i64).sum::<i64>());
            // preprocess degree & neighbors
            println!("CGSHOP Instance: computing degrees & neighbors...");
            // compute domiations
            let mut dominations:Vec<(VertexId,VertexId)> = Vec::new();
            let mut not_dominated = BitSet::with_capacity(n);
            for i in 0..n { not_dominated.insert(i); }
            for i in 0..n {
                if i % 1000 == 0 { println!("computing dominances ({} / {})...", i, n); }
                // list vertices dominated by i
                if not_dominated.contains(i) { // no need to check because domination is transitive
                    let mut dominating = not_dominated.clone();
                    for j in res.neighbors[i].iter() {
                        dominating.intersect_with(&res.neighbors[j]);
                        if dominating.is_empty() { break; } // stop if no more remaining vertex
                    }
                    match dominating.iter().find(|j| *j!=i) {
                        None => {},
                        Some(j) => { // if a vertex v dominates i
                            dominations.push((j,i));
                            not_dominated.remove(i);
                            res.dominated.insert(i);
                        }
                    };
                }
            }
            // update res
            res.preprocessed = Some(PreprocessedData {
                degrees, dominations
            });
            // write the new instance
            let res_str = serde_json::to_string(&res).unwrap();
            let mut file = std::fs::File::create(filename)
                .expect("unable to re-open instance file.");
            file.write_all(res_str.as_bytes())
                .expect("unable to write instance file.");
        }
        res
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

    /// coordinates of a segment ((ax,ay),(bx,by))
    pub fn coordinate(&self, i:usize) -> &((i64,i64),(i64,i64)) {
        &self.coordinates[i]
    }

    /// edge coordinate for segment i (x1,y1,x2,y2)
    pub fn build_coordinates(&self, i:usize) -> ((i64,i64),(i64,i64)) {
        (
            (self.x[self.edge_i[i]] as i64, self.y[self.edge_i[i]] as i64),
            (self.x[self.edge_j[i]] as i64, self.y[self.edge_j[i]] as i64),
        )
    }

    /// Orientation of the edge [0;π]
    pub fn segment_orientation(&self, i:usize) -> f64 {
        let ((ax,ay),(bx,by)) = self.coordinates[i];
        let dx = (bx - ax) as f64;
        let dy = (by - ay) as f64;
        (dy/dx).atan() * 180. / PI
    }

    /// writes the list of edges to a file
    pub fn write_adj_list_file(&self, filename:&str) {
        let m:u64 = self.neighbors.iter().map(|u| u.len() as u64).sum();
        let mut s = String::new();
        s += format!("{} {}\n", self.nb_vertices(), m).as_str();
        for i in self.vertices() {
            for j in self.neighbors(i) {
                s += format!("{} ", j+1).as_str();
            }
            s += "\n";
        }
        let mut file = File::create(filename)
            .expect("CGHSOPSolution.to_file: unable to open the file");
        file.write_all(s.as_bytes())
            .expect("unable to write file content");
    }

    /// writes the instance to the dimacs format
    pub fn write_dimacs(&self, filename:&str) {
        let n:usize = self.nb_vertices();
        let m:u64 = self.neighbors.iter().map(|u| u.len() as u64).sum();
        let mut s = String::new();
        s += format!("c original: {}\n", self.id).as_str();
        s += format!("p edge {} {}\n", n, m).as_str();
        for u in self.vertices() {
            for v in self.neighbors(u) {
                if u < v {
                    s += format!("e {} {}\n", u+1, v+1).as_str();
                }
            }
        }
        let mut file = File::create(filename)
            .expect("CGHSOPSolution.to_file: unable to open the file");
        file.write_all(s.as_bytes())
            .expect("unable to write file content");
    }

}


/** 3 point orientation (either collinear, clockwise or counterclockwise) */
#[derive(Debug,Eq,PartialEq)]
enum Orientation {
    Collinear,
    Clockwise,
    CounterClockwise,
}

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
    let o1 = orientation(p1,q1,p2);
    let o2 = orientation(p1,q1,q2);
    if p1 == p2 || p1 == q2 || q1 == p2 || q1 == q2 { // check if same points
        return (o1 == Orientation::Collinear && p1 != p2 && q1 != p2) ||
            (o2 == Orientation::Collinear && p1 != q2 && q1 != q2); // conflict only if collinear
    } // otherwise, accept end points that are the same
    // if no same points, "just" check if they are intersecting
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

    /// returns the corresponding graph coloring solution
    pub fn to_solution(&self) -> Vec<Vec<VertexId>> {
        let mut res = vec![vec![] ; self.num_colors];
        for (i,c) in self.colors.iter().enumerate() {
            res[*c].push(i);
        }
        res
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
    fn test_use_best_so_far() {
        let inst = CGSHOPInstance::from_file(
            "./insts/cgshop22/rvispecn17968.instance.json",
        );
        println!("clique len\t {}", inst.clique().unwrap().len());
        println!("coloring len\t {}", inst.coloring().unwrap().len());
    }

    #[test]
    fn colinear_with_same_point() {
        let a = ((0,0), (0,1));
        let b = ((0,0), (0,5));
        assert!(are_intersecting(&a, &b));
    }

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
    fn test_preprocess_merge_visp() {
        let _ = CGSHOPInstance::from_file(
            "./insts/cgshop_22_examples/visp_5K.instance.json",
        );
        // cg_inst.preprocess_merge();
    }

    #[test]
    fn test_preprocess_merge_visp10k() {
        let _ = CGSHOPInstance::from_file(
            "./insts/cgshop_22_examples/visp_10K.instance.json",
        );
        // cg_inst.preprocess_merge();
    }

    #[test]
    fn test_preprocess_merge_visp50k() {
        let _ = CGSHOPInstance::from_file(
            "./insts/cgshop_22_examples/visp_50K.instance.json",
        );
        // cg_inst.preprocess_merge();
    }

    #[test]
    fn test_read_reecn() {
        let _ = CGSHOPInstance::from_file(
            "./insts/cgshop22/reecn73116.instance.json",
        );
    }

    #[test]
    fn test_read_rsqrp() {
        let _ = CGSHOPInstance::from_file(
            "./insts/cgshop22/rsqrp24641.instance.json",
        );
    }

    #[test]
    fn test_read_sqrp() {
        let _ = CGSHOPInstance::from_file(
            "./insts/cgshop22/sqrp73525.instance.json",
        );
    }

    #[test]
    fn test_read_visp() {
        let _ = CGSHOPInstance::from_file(
            "./insts/cgshop22/visp73369.instance.json",
        );
    }

    #[test]
    fn test_read_vispecn() {
        let _ = CGSHOPInstance::from_file(
            "./insts/cgshop22/vispecn74166.instance.json",
        );
    }

    #[test]
    fn test_export() {
        let inst = CGSHOPInstance::from_file(
            "./insts/cgshop22/rvispecn17968.instance.json",
        );
        inst.write_adj_list_file("tmp/rvispecn17968.adjlist.txt")
    }

    #[test]
    fn test_export2() {
        let inst = CGSHOPInstance::from_file(
            "./insts/cgshop22/rvisp3499.instance.json",
        );
        inst.write_adj_list_file("tmp/rvisp3499.adjlist.txt")
    }

    #[test]
    fn test_export_dimacs() {
        let name = "vispecn2518";
        let inst = CGSHOPInstance::from_file(
            format!("./insts/cgshop22/{}.instance.json", name).as_str(),
        );
        inst.write_dimacs(format!("insts/dimacs_cgshop/{}.dimacs.txt", name).as_str());
    }
}


