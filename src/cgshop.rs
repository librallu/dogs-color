/*
Implements:
 - procedures to read and write CGSHOP instance and solution formats
 - procedures to produce an instance from a CGSHOP instance and vice-versa
*/
use std::fs;

use serde::{Serialize, Deserialize};
use geo::{Coordinate, Line, intersects::Intersects};

use crate::color::Instance;

/** data structure to represent a CGSHOP instance */
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CGSHOPInstance {
    /// number of points
    n: usize,
    /// number of edges
    m: usize,
    /// x coordinates for points
    x: Vec<f32>,
    /// y coordinates for points
    y: Vec<f32>,
    /// edge_i[i]: first endpoint of the ith edge
    edge_i: Vec<usize>,
    /// edge_j[i]: second endpoint of the ith edge
    edge_j: Vec<usize>,
}


impl CGSHOPInstance {
    /** reads a CGSHOP instance from a file. */
    pub fn from_file(filename:&str) -> Self {
        let str = fs::read_to_string(filename)
            .expect("Error while reading the file...");
        serde_json::from_str(&str)
            .expect("Error while deserializing the json file")
    }

    /** converts to a graph coloring instance. */
    pub fn to_graph_coloring_instance(&self) -> Instance {
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
        Instance::new(adj_list)
    }

    /// number of vertices
    pub fn n(&self) -> usize { self.n }

    /// number of edges
    pub fn m(&self) -> usize { self.m }

    /// edge coordinates (x1,y1,x2,y2)
    pub fn edge_coordinates(&self) -> Vec<(f32,f32,f32,f32)> {
        (0..self.edge_i.len()).map(|i| {
            let x1 = self.x[self.edge_i[i]];
            let y1 = self.y[self.edge_i[i]];
            let x2 = self.x[self.edge_j[i]];
            let y2 = self.y[self.edge_j[i]];
            (x1, y1, x2, y2)
        }).collect()
    }

    /** displays some statistics of the instance */
    pub fn display_statistics(&self) {
        println!("\t{:>25}{:>10}", "nb vertices:", self.n());
        println!("\t{:>25}{:>10}", "nb edges:",    self.m());
        // for (x1,y1,x2,y2) in self.edge_coordinates() {
        //     println!("{:.2}\t{:.2} | {:.2}\t{:.2}", x1,y1, x2,y2);
        // }
    }
}

/** true iff the lines defined by (x1,y1), (x2,y2) intersect */
pub fn is_intersection(a:&(f32,f32,f32,f32), b:&(f32,f32,f32,f32)) -> bool {
    let l1 = Line::new(
        Coordinate {x:a.0, y:a.1},
        Coordinate {x:a.2, y:a.3}
    );
    let l2 = Line::new(
        Coordinate {x:b.0, y:b.1},
        Coordinate {x:b.2, y:b.3}
    );
    l1.intersects(&l2)
}


/** data structure to represent a CGSHOP instance */
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


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_instance() {
        let cg_inst = CGSHOPInstance::from_file("./insts/CGSHOP_22_original/cgshop_2022_examples_01/example_instances_visp/visp_5K.instance.json");
        cg_inst.display_statistics();
        let inst = cg_inst.to_graph_coloring_instance();
        inst.display_statistics();
    }
}