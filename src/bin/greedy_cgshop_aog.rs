use std::rc::Rc;
use std::time::Instant;

use clap::{App, load_yaml};
use serde_json::json;

use dogs_color::cgshop::CGSHOPInstance;
use dogs_color::search::cgshop_aog::cgshop_aog;
use dogs_color::util::{read_params, export_results};


/** solves a coloring instance using a DSATUR greedy */
pub fn main() {
    // parse arguments
    let yaml = load_yaml!("greedy_cgshop_aog.yml");
    let main_args = App::from_yaml(yaml).get_matches();
    let (
        inst_filename,
        _,
        _,
        sol_file,
        perf_file
    ) = read_params(main_args);
    let instance = Rc::new(CGSHOPInstance::from_file(&inst_filename, true));
    // solve it
    let t_start = Instant::now();
    let solution = cgshop_aog(instance.clone(), true);
    let duration = t_start.elapsed().as_secs_f32();
    let nb_colors = solution.len();
    println!("AOG took {:.3} seconds. Nb colors: {}", duration, nb_colors);
    let stats = json!({
        "primal_list": vec![nb_colors],
        "time_searched": duration,
        "inst_name": inst_filename
    });

    // export results
    export_results(instance, &solution, &stats, perf_file, sol_file);
}