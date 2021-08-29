use std::time::Instant;

use clap::{App, load_yaml};
use serde_json::json;

use dogs_color::search::greedy_rlf::greedy_rlf;
use dogs_color::util::{read_params, export_results};


/** solves a coloring instance using a RLF greedy */
pub fn main() {
    // parse arguments
    let yaml = load_yaml!("greedy_rlf.yml");
    let main_args = App::from_yaml(yaml).get_matches();
    let (
        inst_filename,
        instance,
        _,
        sol_file,
        perf_file
    ) = read_params(main_args);

    // solve it
    let t_start = Instant::now();
    let solution = greedy_rlf(instance.clone(), true);
    let duration = t_start.elapsed().as_secs_f32();
    let nb_colors = solution.len();
    println!("RLF took {:.3} seconds. Nb colors: {}", duration, nb_colors);
    let stats = json!({
        "primal_list": vec![nb_colors],
        "time_searched": duration,
        "inst_name": inst_filename
    });

    // export results
    export_results(instance, &solution, &stats, perf_file, sol_file);
}