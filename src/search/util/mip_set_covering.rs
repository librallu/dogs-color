use good_lp::{Expression, ProblemVariables, SolverModel, default_solver, variable, Solution, ResolutionError};
use bit_set::BitSet;

/**
    solves the set cover problem
*/
pub fn mip_cbc_set_covering(nb_elts:usize, subsets:&[Vec<usize>]) -> Option<Vec<usize>> {
    println!("Solving using MIP (CBC) ...");
    let sub_bitsets:Vec<BitSet> = subsets.iter().map(|s| {
        let mut res = BitSet::with_capacity(nb_elts);
        for e in s { res.insert(*e); }
        res
    }).collect();
    let nb_subs = subsets.len();
    let mut model = ProblemVariables::new();
    // x_{s}: subset s is selected
    let mut x = Vec::with_capacity(nb_subs);
    for _ in 0..nb_subs {
        x.push(model.add(variable().binary()));
    }
    // ∑_{s∈S} (s covers e) >= 1      ∀ e ∈ E
    let mut csts = Vec::with_capacity(nb_elts);
    for e in 0..nb_elts {
        let covers:Vec<usize> = (0..nb_subs)
            .filter(|s| sub_bitsets[*s].contains(e)).collect(); // covers[e]: subsets covering e
        let mut cst = Expression::with_capacity(covers.len());
        for s in covers { cst.add_mul(1., x[s]); }
        csts.push(cst.geq(1));
    }
    // add objective
    let mut obj_expr = Expression::with_capacity(nb_subs);
    for xs in x.iter() { obj_expr.add_mul(1., *xs); }
    // set solver parameters
    // solve
    let mut problem = model.minimise(obj_expr).using(default_solver); // CBC
    while !csts.is_empty() {
        problem.add_constraint(csts.pop().unwrap());
    }
    let sol = problem.solve();
    // extract solutions
    match sol {
        Ok(sol_cbc) => {
            let mut sol = Vec::new();
            for (i,xi) in x.iter().enumerate() {
                if sol_cbc.value(*xi) >= 0.5 {
                    sol.push(i);
                }
            }
            Some(sol)
        },
        Err(ResolutionError::Infeasible) => None,
        Err(e) => {
            println!("{:?}", e);
            panic!("error while solving");
        }
    }
}