use std::time::Instant;

use bit_set::BitSet;
use clap::{App, load_yaml};
use serde_json::json;

use dogs_color::util::{read_params, export_results};
use dogs_color::search::greedy_clique::greedy_clique;
use dogs_color::search::clique_swap::clique_swaps;


/** solves a CLIQUE problem using a swap-based tabu search. */
pub fn main() {
    // parse arguments
    let yaml = load_yaml!("clique_tabu_swap.yml");
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
    let sol_greedy = greedy_clique(instance.clone());
    let solution = clique_swaps(
        instance.clone(),
        sol_greedy,
        instance.nb_vertices(),
        true
    );
    let duration = t_start.elapsed().as_secs_f32();
    let nb_vertices = solution.len();
    println!("tabu search took {:.3} seconds. Nb vertices: {}", duration, nb_vertices);
    let stats = json!({
        "primal_list": vec![nb_vertices],
        "time_searched": duration,
        "inst_name": inst_filename
    });

    // export results
    let mut in_clique = BitSet::new();
    for e in &solution { in_clique.insert(*e); }
    let mut not_in_clique = Vec::new();
    for e in instance.vertices() {
        if !in_clique.contains(e) { not_in_clique.push(e); }
    }
    let exportable_sol = vec![solution, not_in_clique];
    export_results(instance, &exportable_sol, &stats, perf_file, sol_file);
}