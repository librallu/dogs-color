use std::rc::Rc;

use clap::ArgMatches;
use serde_json::Value;

use crate::{
    cgshop::CGSHOPInstance,
    color::{ColoringInstance, VertexId},
    compact_instance::CompactInstance
};

/** reads command line input and returns the instance name, time, solution_filename, stats_filename */
pub fn read_params(main_args:ArgMatches) -> (String, Rc<dyn ColoringInstance>, f32, Option<String>, Option<String>) {
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
            Rc::new(CompactInstance::from_file(inst_filename))
        },
        "cgshop" => { // read CGSHOP instance
            Rc::new(CGSHOPInstance::from_file(inst_filename, true))
        },
        _ => panic!("instance type unknown {}", instance_type)
    };
    instance.display_statistics();
    println!("=======================");
    (inst_filename.to_string(), instance, t, sol_file, perf_file)
}

/// exports search results to files
pub fn export_results(
    instance:Rc<dyn ColoringInstance>,
    solution:&[Vec<VertexId>],
    stats:&Value,
    perf_file:Option<String>,
    sol_file:Option<String>
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
        Some(filename) => { instance.write_solution(filename.as_str(), solution); }
    }
}