use std::fs;

use nom::{IResult, error::Error};
use nom::bytes::complete::take;


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
    match nom::bytes::complete::tag("c")(s) {
        Ok((remaining,_)) => { // if a comment: read until a '\n'
            match nom::bytes::complete::take_until("\n")(remaining) {
                Ok((remaining2, _)) => {
                    let n:usize = 1;
                    nom::bytes::complete::take(n)(remaining2)
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
    match nom::bytes::complete::tag("p edge ")(s) {
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
    fn test_read_edge() {
        let s = "e 1 2\n";
        assert_eq!(read_edge(s).unwrap().1, (1,2));
        assert_eq!(read_edge(s).unwrap().0, "");
    }
}