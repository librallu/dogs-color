use std::rc::Rc;

use clap::{App, load_yaml};
use dogs::search_algorithm::TimeStoppingCriterion;

use dogs_color::cgshop::CGSHOPInstance;
use dogs_color::search::cgshop_aog::cgshop_aog;
use dogs_color::search::tabucol::tabucol_with_solution;
use dogs_color::search::greedy_dsatur::greedy_dsatur;
use dogs_color::util::{read_params, export_results};
use serde_json::json;


/** solves a coloring instance using a DSATUR greedy */
pub fn main() {
    // parse arguments
    let yaml = load_yaml!("tabucol.yml");
    let main_args = App::from_yaml(yaml).get_matches();
    let (
        inst_filename,
        instance,
        t,
        sol_file,
        perf_file
    ) = read_params(main_args);
    // let instance = Rc::new(CGSHOPInstance::from_file(&inst_filename));

    // solve it
    let sol_greedy = greedy_dsatur(instance.clone(), true);
    // let sol_orientation_greedy = cgshop_aog(instance.clone(), true);
    // let sol_greedy = if sol_dsatur.len() < sol_orientation_greedy.len() {
    //     sol_dsatur
    // } else {
    //     sol_orientation_greedy
    // };
    println!("greedy found {} colors", sol_greedy.len());
    let solution = tabucol_with_solution(
        instance.clone(),
        &sol_greedy,
        TimeStoppingCriterion::new(t),
        None
    );
    let stats = json!({
        "primal_list": vec![solution.len()],
        "time_searched": t,
        "inst_name": inst_filename
    });

    // export results
    export_results(instance, &solution, &stats, perf_file, sol_file);
}