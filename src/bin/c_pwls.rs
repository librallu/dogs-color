use std::time::Instant;

use clap::{App, load_yaml};
use dogs::search_algorithm::TimeStoppingCriterion;

use dogs_color::search::clique_partial_weighting::{clique_partial_weighting};
use dogs_color::search::greedy_clique::greedy_clique;
use dogs_color::util::read_params;


/** solves a coloring instance using a DSATUR greedy */
pub fn main() {
    // parse arguments
    let yaml = load_yaml!("c_pwls.yml");
    let main_args = App::from_yaml(yaml).get_matches();
    let (
        _,
        instance,
        t,
        _,
        sol_file,
        perf_file
    ) = read_params(main_args);
    let time_init = Instant::now();
    // solve it
    let sol_greedy = greedy_clique(instance.clone());
    println!("greedy found {} vertices in {:.3} seconds", sol_greedy.len(), time_init.elapsed().as_secs_f32());
    clique_partial_weighting(
        instance,
        &sol_greedy,
        perf_file,
        sol_file,
        TimeStoppingCriterion::new(t)
    );
}