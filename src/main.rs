// useful additional warnings if docs are missing, or crates imported but unused, etc.
#![warn(missing_debug_implementations)]
#![warn(missing_docs)]
#![warn(trivial_casts, trivial_numeric_casts)]
#![warn(unsafe_code)]
#![warn(unused_extern_crates)]
#![warn(variant_size_differences)]

// not sure if already by default in clippy
#![warn(clippy::similar_names)]
#![warn(clippy::shadow_unrelated)]
#![warn(clippy::shadow_same)]
#![warn(clippy::shadow_reuse)]

// checks integer arithmetic in the project
// #![warn(clippy::integer_arithmetic)]

// these flags can be useful, but will indicate normal behavior
// #![warn(clippy::print_stdout)]
// #![warn(clippy::use_debug)]
// #![warn(clippy::cast_possible_truncation)]
// #![warn(clippy::cast_possible_wrap)]
// #![warn(clippy::cast_precision_loss)]
// #![warn(clippy::cast_sign_loss)]

#[macro_use]
extern crate clap;
use clap::App;

use std::rc::Rc;
use std::cell::RefCell;

use dogs::metric_logger::MetricLogger;
use dogs::search_algorithm::{SearchAlgorithm, TimeStoppingCriterion};
use dogs::search_space::{SearchSpace, ToSolution};
use dogs::tree_search::decorators::stats::StatTsDecorator;
use dogs::tree_search::algo::beam_search::BeamSearch;

// register modules
mod color;
use color::Instance;

// mod dsatur;

/**
reads an instance, takes the time limit as a parameter,
and solves the problem
*/
pub fn main() {
    // parse arguments
    let yaml = load_yaml!("main_args.yml");
    let main_args = App::from_yaml(yaml).get_matches();
    // read instance file, time, and optionnal parameters
    println!("=========================================================");
    let inst_filename = main_args.value_of("instance").unwrap();
    let t:f32 = main_args.value_of("time").unwrap().parse::<f32>().unwrap();
    // read value of the solution filename
    let _sol_file: Option<String> = match main_args.value_of("solution") {
        None => None,
        Some(e) => {
            println!("\t printing solutions in: {}", e);
            Some(e.to_string())
        }
    };
    // read value of the performance logs filename
    let _perf_file: Option<String> = match main_args.value_of("perf") {
        None => None,
        Some(e) => {
            println!("\t printing perfs in: {}\n", e);
            Some(e.to_string())
        }
    };
    println!("reading instance: {}...", inst_filename);
    let inst = Rc::new(Instance::from_file(inst_filename));
    println!("time limit: {}", t);
    // create logger and stopping criterion
    let logger = Rc::new(MetricLogger::default());
    let stopping_criterion = TimeStoppingCriterion::new(t);
    // if main_args.subcommand_matches("localsearch").is_some() {
    //     println!("generating initial solution...");
    //     // create search space
    //     let space = Rc::new(RefCell::new(
    //         TreeSearchSpace::new(inst)
    //     ));
    //     // create the search algorithm
    //     let mut ts = BeamSearch::new(space.clone(), 1);
    //     ts.run(stopping_criterion);
    //     // get the greedy solution
    //     let mut greedy_node = ts.get_manager().best().clone()
    //         .expect("fail: no solution found by the greedy");
    //     let greedy_solution = space.borrow_mut()
    //         .solution(&mut greedy_node);
    //     println!("greedy solution ({}): {:?}", greedy_node.cost(), greedy_solution);
    //     println!("starting local search...");
    // }
}