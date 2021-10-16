use std::rc::Rc;

use bit_set::BitSet;
use clap::ArgMatches;
use serde_json::Value;

use crate::{
    cgshop::CGSHOPInstance,
    dimacs::DimacsInstance,
    color::{ColoringInstance, VertexId, checker},
};

/** reads command line input and returns the instance name, time, solution_filename, stats_filename */
pub fn read_params(main_args:ArgMatches) -> (String, Rc<dyn ColoringInstance>, f32, String, Option<String>, Option<String>) {
    let inst_filename = main_args.value_of("instance").unwrap();
    let instance_type = main_args.value_of("type").unwrap();
    let t:f32 = main_args.value_of("time").unwrap().parse::<f32>()
        .expect("unable to parse the time given");
    // read value of the solution filename
    let sol_file: Option<String> = match main_args.value_of("solution") {
        None => None,
        Some(e) => {
            println!("printing solutions in: {}", e);
            Some(e.to_string())
        }
    };
    // read value of the performance logs filename
    let perf_file: Option<String> = match main_args.value_of("perf") {
        None => None,
        Some(e) => {
            println!("printing perfs in: {}\n", e);
            Some(e.to_string())
        }
    };
    // read instance file
    let instance:Rc<dyn ColoringInstance> = match instance_type {
        "dimacs" => { // read DIMACS instance
            Rc::new(DimacsInstance::from_file(inst_filename))
        },
        "cgshop" => { // read CGSHOP instance
            Rc::new(CGSHOPInstance::from_file(inst_filename))
        },
        _ => panic!("instance type unknown {}", instance_type)
    };
    instance.display_statistics();
    println!("=======================");
    (inst_filename.to_string(), instance, t, instance_type.to_string(), sol_file, perf_file)
}

/// exports search results to files
pub fn export_results(
    instance:Rc<dyn ColoringInstance>,
    solution:&[Vec<VertexId>],
    stats:&Value,
    perf_file:Option<String>,
    sol_file:Option<String>,
    check_result:bool,
) {
    // export statistics and solution
    match perf_file {
        None => {},
        Some(filename) => {
            let mut file = match std::fs::File::create(filename.as_str()) {
                Err(why) => panic!("couldn't create {}: {}", filename, why),
                Ok(file) => file
            };
            if let Err(why) = std::io::Write::write(
                &mut file, serde_json::to_string(stats).unwrap().as_bytes()
            ) { panic!("couldn't write: {}",why) };
        }
    }
    // export solution
    match sol_file {
        None => {},
        Some(filename) => {
            if check_result {
                let checker_result = checker(instance.clone(), solution);
                match checker_result {
                    crate::color::CheckerResult::Ok(_) => {},
                    // _ => { panic!("invalid solution (reason: {:?})", checker_result)}
                    _ => { println!("invalid solution (reason: {:?})", checker_result)}
                };
            }
            instance.write_solution(filename.as_str(), solution);
        }
    }
}

/// transforms a clique defined by a vector, to a clique defined by a vector of vector
pub fn clique_vec_to_vecvec(sol:&[VertexId], n:usize) -> Vec<Vec<VertexId>> {
    let mut res = vec![sol.to_vec()];
    let mut inside_res:BitSet = BitSet::default();
    for i in &res[0] { inside_res.insert(*i); }
    let mut non_clique = Vec::new();
    for i in 0..n {
        if ! inside_res.contains(i) { non_clique.push(i); }
    }
    res.push(non_clique);
    res
}