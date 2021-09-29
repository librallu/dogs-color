use std::rc::Rc;
use std::time::Instant;

use clap::{App, load_yaml};
use dogs::search_algorithm::TimeStoppingCriterion;

use dogs_color::cgshop::CGSHOPInstance;
use dogs_color::search::cgshop_aog::cgshop_aog;
use dogs_color::search::row_weighting_local_search::row_weighting_local_search;
use dogs_color::search::greedy_dsatur::greedy_dsatur;
use dogs_color::util::{read_params, export_results};
use serde_json::json;


/** solves a coloring instance using a DSATUR greedy */
pub fn main() {
    // parse arguments
    let yaml = load_yaml!("cwls.yml");
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
    println!("greedy found {} colors", sol_greedy.len());
    row_weighting_local_search(
        instance,
        &sol_greedy,
        TimeStoppingCriterion::new(t),
        sol_file,
        perf_file,
        time_init.elapsed().as_secs_f32()
    );
    // let solution = tabucol_with_solution(
    //     instance.clone(),
    //     &sol_greedy,
    //     TimeStoppingCriterion::new(t),
    //     None
    // );
    // let stats = json!({
    //     "primal_list": vec![solution.len()],
    //     "time_searched": t,
    //     "inst_name": inst_filename
    // });

    // // export results
    // export_results(instance, &solution, &stats, perf_file, sol_file);
}