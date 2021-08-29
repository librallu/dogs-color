use std::time::Instant;

use clap::{App, load_yaml};
use serde_json::json;
use dogs::search_algorithm::TimeStoppingCriterion;

use dogs_color::search::tabucol::tabucol;
use dogs_color::search::greedy_dsatur::greedy_dsatur;
use dogs_color::util::{read_params, export_results};


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

    // solve it
    let sol_greedy = greedy_dsatur(instance.clone(), true);
    println!("greedy found {} colors", sol_greedy.len());
    tabucol(
        instance.clone(),
        sol_greedy.len(),
        TimeStoppingCriterion::new(t),
        None
    );
    // let stats = json!({
    //     "primal_list": vec![],
    //     "time_searched": t,
    //     "inst_name": inst_filename
    // });

    // export results
    // export_results(instance, &solution, &stats, perf_file, sol_file);
}