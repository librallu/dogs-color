use std::rc::Rc;
use std::time::Instant;

use clap::{App, load_yaml};
use dogs::search_algorithm::TimeStoppingCriterion;

use dogs_color::cgshop::CGSHOPInstance;
use dogs_color::search::cgshop_aog::cgshop_aog;
use dogs_color::search::conflict_weighting_local_search::{conflict_weighting_local_search};
use dogs_color::search::greedy_dsatur::greedy_dsatur;
use dogs_color::util::read_params;


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
    println!("greedy found {} colors in {:.3} seconds", sol_greedy.len(), time_init.elapsed().as_secs_f32());
    conflict_weighting_local_search(
        instance,
        &sol_greedy,
        perf_file,
        sol_file,
        TimeStoppingCriterion::new(t)
    );
}