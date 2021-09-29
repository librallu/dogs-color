use std::fs;

use bit_set::BitSet;
use nom::{IResult, error::Error};
use nom::bytes::complete::{take, tag, take_until};
use nom::branch::alt;

use crate::color::{ColoringInstance, VertexId};


/** models a Graph Coloring instance.  */
#[derive(Debug)]
pub struct DimacsInstance {
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

impl ColoringInstance for DimacsInstance {
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


impl DimacsInstance {

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
        let mut res = Self { n,m, edges, adj_list, adj_matrix:None };
        res.populate_adj_matrix();
        res
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


/// reads an instance from file, returns (n,m,adj_list)
pub fn read_from_file(filename:&str) -> (usize, usize, Vec<Vec<usize>>) {
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
    (n, m, adj_list)
}

/// skips a single comment
fn skip_comment(s:&str) -> IResult<&str, &str> {
    match tag("c")(s) {
        Ok((remaining,_)) => { // if a comment: read until a '\n'
            match take_until("\n")(remaining) {
                Ok((remaining2, _)) => {
                    let n:usize = 1;
                    take(n)(remaining2)
                },
                Err(e) => Err(e),
            }
        },
        Err(e) => Err(e),
    }
}

/// skips all comments
pub fn skip_comments(s:&str) -> IResult<&str, Vec<&str>> {
    nom::multi::many0(skip_comment)(s)
}

/// reads two numbers separated by a space
fn read_two_integers(s:&str) -> IResult<&str, (usize,usize)> {
    match nom::character::complete::digit1(s) {
        Ok((remaining1,s1)) => {
            let n1 = s1.parse::<usize>().unwrap();
            let usize_1:usize = 1;
            match take(usize_1)(remaining1) {
                Ok((remaining2,_)) => {
                    match nom::character::complete::digit1(remaining2) {
                        Ok((remaining3, s2)) => {
                            let n2 = s2.parse::<usize>().unwrap();
                            if nom::character::is_newline(*remaining3.as_bytes().get(0).unwrap()) {
                                match take::<usize, &str, Error<&str>>(usize_1)(remaining3) {
                                    Ok((remaining4,_)) => Ok((remaining4,(n1,n2))),
                                    Err(_) => Ok((remaining3,(n1,n2))),
                                }
                            } else {
                                Ok((remaining3,(n1,n2)))
                            }
                        },
                        Err(e) => Err(e),
                    }
                },
                Err(e) => Err(e)
            }
        },
        Err(e) => Err(e)
    }
}

/// reads header containing (n,m)
pub fn read_header(s:&str) -> IResult<&str, (usize,usize)> {
    match alt((tag("p edge "), tag("p col ")))(s) {
        Ok((remaining,_)) => { // if ok, read the two numbers
            read_two_integers(remaining)
        }
        Err(e) => Err(e)
    }
}

/// reads edge line (WARNING: indices start at 1 in the DIMACS format)
pub fn read_edge(s:&str) -> IResult<&str, (usize,usize)> {
    match nom::bytes::complete::tag("e ")(s) {
        Ok((remaining,_)) => { // if ok, read the two numbers
            read_two_integers(remaining)
        }
        Err(e) => Err(e)
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_instance() {
        let inst = DimacsInstance::from_file("insts/grid-instances/grid2x2");
        // println!("{:?}", inst);
        assert_eq!(inst.nb_vertices(), 4);
        assert_eq!(inst.nb_edges(), 4);
    }

    #[test]
    fn test_read_comment1() {
        let s = "c this is a test comment\np edge 2 1\ne 1 2";
        let res:Result<(&str, &str), nom::Err<()>> = nom::bytes::complete::is_a("c")(s);
        assert_eq!(res, Ok((" this is a test comment\np edge 2 1\ne 1 2", "c")));
        assert_eq!(
            skip_comments(s),
            Ok((
                "p edge 2 1\ne 1 2",
                vec!["\n"]
            ))
        );
    }

    #[test]
    fn test_read_header() {
        let s = "p edge 2 1\ne 1 2";
        assert_eq!(read_header(s).unwrap().0, "e 1 2");
        assert_eq!(read_header(s).unwrap().1, (2,1));
    }

    #[test]
    fn test_read_header_col() {
        let s = "p col 2 1\ne 1 2";
        assert_eq!(read_header(s).unwrap().0, "e 1 2");
        assert_eq!(read_header(s).unwrap().1, (2,1));
    }

    #[test]
    fn test_read_edges_on_one_line() {
        let (n,m,e) = read_from_file("insts/other-instances/peterson.col");
        println!("n:{}, m:{}", n, m);
        println!("e:{:?}", e);
    }

    #[test]
    fn test_read_edge() {
        let s = "e 1 2\n";
        assert_eq!(read_edge(s).unwrap().1, (1,2));
        assert_eq!(read_edge(s).unwrap().0, "");
    }
}