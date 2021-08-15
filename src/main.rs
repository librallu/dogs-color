//! DOGS implementation of the Graph Coloring problem


// #![warn(clippy::all, clippy::pedantic)]
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

#[macro_use]
extern crate clap;
use clap::App;
use dogs::search_space::SearchSpace;

use std::rc::Rc;
use std::cell::RefCell;

use dogs::metric_logger::MetricLogger;
use dogs::search_algorithm::{SearchAlgorithm, TimeStoppingCriterion};
// use dogs::search_space::{SearchSpace, ToSolution};
use dogs::tree_search::decorators::stats::StatTsDecorator;
use dogs::tree_search::decorators::pruning::PruningDecorator;
// use dogs::tree_search::algo::beam_search::BeamSearch;
use dogs::tree_search::algo::beam_search::create_iterative_beam_search;


// register modules
mod dimacs;
mod color;

mod dsatur;
mod tabucol;

use color::Instance;
use crate::dsatur::dsatur_greedy;


/**
reads an instance, takes the time limit as a parameter, and solves the problem.

# Panics
 - if the time cannot be parsed
*/
pub fn main() {
    // parse arguments
    let yaml = load_yaml!("main_args.yml");
    let main_args = App::from_yaml(yaml).get_matches();
    // read instance file, time, and optionnal parameters
    println!("=========================================================");
    let inst_filename = main_args.value_of("instance").unwrap();
    let t:f32 = main_args.value_of("time").unwrap().parse::<f32>()
        .expect("unable to parse the time given");
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
    inst.print_stats();
    // println!("{:?}", inst);
    println!("time limit: {}", t);
    // create logger and stopping criterion
    let logger = Rc::new(MetricLogger::default());
    let stopping_criterion = TimeStoppingCriterion::new(t);
    if main_args.subcommand_matches("dsatur").is_some() {
        // // create search space
        // let space = Rc::new(RefCell::new(
        //     StatTsDecorator::new(
        //         PruningDecorator::new(
        //             DSATURSpace::new(inst)
        //         )
        //     ).bind_logger(Rc::downgrade(&logger)),
        // ));
        // // create the search algorithm
        // logger.display_headers();
        // let mut ts = create_iterative_beam_search(space.clone(), 1., 2.)
        //     .bind_logger(Rc::downgrade(&logger));
        // ts.run(stopping_criterion);
        // space.borrow_mut().display_statistics();
        let solution = dsatur_greedy(inst);
        println!("nb initial colors: {}", solution.len());
        println!("{:?}", solution);
    }
}