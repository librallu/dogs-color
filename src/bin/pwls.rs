use std::rc::Rc;
use std::time::Instant;

use clap::{App, load_yaml};
use dogs::search_algorithm::TimeStoppingCriterion;

use dogs_color::cgshop::CGSHOPInstance;
use dogs_color::color::{CheckerResult, checker};
use dogs_color::search::cgshop_aog::cgshop_aog;
use dogs_color::search::coloring_partial_weighting::{coloring_partial_weighting};
use dogs_color::search::greedy_dsatur::greedy_dsatur;
use dogs_color::util::read_params;


/** solves a coloring instance using a DSATUR greedy */
pub fn main() {
    // parse arguments
    let yaml = load_yaml!("pwls.yml");
    let main_args = App::from_yaml(yaml).get_matches();
    let (
        inst_filename,
        instance,
        t,
        instance_type,
        sol_file,
        perf_file
    ) = read_params(main_args);
    let time_init = Instant::now();
    // solve it
    let initial_solution = match instance.coloring() {
        None => {
            let sol_greedy = match instance_type.as_str() {
                "dimacs" => { greedy_dsatur(instance.clone(), false) }
                "cgshop" => {
                    let sol_dsatur = greedy_dsatur(instance.clone(), false);
                    let instance = Rc::new(CGSHOPInstance::from_file(&inst_filename));
                    let sol_orientation_greedy = cgshop_aog(instance, true);
                    if sol_dsatur.len() < sol_orientation_greedy.len() {
                        sol_dsatur
                    } else {
                        sol_orientation_greedy
                    }
                },
                _ => { panic!("unrecognized instance type {} (valid: 'dimacs', 'cgshop')", instance_type.as_str())}
            };
            println!("greedy found {} colors in {:.3} seconds", sol_greedy.len(), time_init.elapsed().as_secs_f32());
            sol_greedy
        }
        Some(sol) => { sol }
    };
    assert_eq!(
        checker(instance.clone(), &initial_solution),
        CheckerResult::Ok(initial_solution.len())
    );
    println!("initial coloring: {}", initial_solution.len());
    match instance.clique() {
        None => {},
        Some(c) => {
            println!("clique: {}", c.len());
            if c.len() == initial_solution.len() {
                println!("optimal solution is already found!");
                return;
            }
        }
    }
    coloring_partial_weighting(
        instance,
        &initial_solution,
        perf_file,
        sol_file,
        TimeStoppingCriterion::new(t)
    );
}