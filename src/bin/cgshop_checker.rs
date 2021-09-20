use std::rc::Rc;

use clap::{App, load_yaml};

use dogs_color::{
    cgshop::{CGSHOPInstance, CGSHOPSolution},
    color::{ColoringInstance, checker, CheckerResult}
};

/** solves a coloring instance using a DSATUR greedy */
pub fn main() {
    // parse arguments
    let yaml = load_yaml!("cgshop_checker.yml");
    let main_args = App::from_yaml(yaml).get_matches();
    let inst_filename = main_args.value_of("instance").unwrap();
    let sol_filename = main_args.value_of("solution").unwrap();
    // read files
    let instance:Rc<dyn ColoringInstance> = Rc::new(
        CGSHOPInstance::from_file(inst_filename)
    );
    let solution:CGSHOPSolution = CGSHOPSolution::from_file(sol_filename);
    // call checker
    let coloring_solution = solution.to_solution();
    let res_checker = checker(instance, &coloring_solution);
    match res_checker {
        CheckerResult::Ok(n) => {
            println!("{}", n);
        },
        CheckerResult::VertexAddedTwice(v) => {
            println!("ERROR: segment {} colored twice", v);
        },
        CheckerResult::VertexNotColored(v) => {
            println!("ERROR: segment {} not colored", v);
        },
        CheckerResult::ConflictingEdge(a, b) => {
            println!("ERROR: segments {} and {} are conflicting", a, b);
        },
    };
}
