use std::{rc::Rc, time::Instant};

use bit_set::BitSet;
use fastrand::Rng;

use dogs::{
    combinators::helper::tabu_tenure::TabuTenure,
    data_structures::sparse_set::SparseSet,
    search_algorithm::StoppingCriterion
};

use crate::color::{ColoringInstance, VertexId};

type Weight = u16;

/// models a decision within the local search.
#[derive(Debug,Clone,Eq,PartialEq,Hash)]
struct Node {
    pub vertex:VertexId, // vertex to change
    pub previous_color:usize, // previous color of vertex v
    pub next_color:usize, // next color of vertex v
    pub total_penalties:Weight, // total Weight associated with the decision
    pub nb_conflicts:i64, // number of conflicts
}


/** implements a specific tabu tenure for the graph coloring
Is parametrized by:
 - L: minimum size of the tabu tenure (example value: 10). We use a random number between 0 and L.
 - λ: (example value: 0.6)
keeps all moves that are between L+λ.F(c) where F(c) is the number of conflicts.
Maintains the current iteration number and the last iteration in which
the decision have been taken. When checking if a conflict exists,
checks that its last access is greater than current_iter - L+λ.F(c)
*/
#[derive(Debug)]
pub struct TabuColTenure {
    /// tabu fixed size
    l:usize,
    /// tabu dynamic size
    lambda: f64,
    /// number of iterations since the beginning of the search
    nb_iter: i64,
    /// decisions[v][c]: last iteration in which the decision have been taken
    decisions: Vec<Vec<i64>>,
    /// random number generator
    rng: Rng,
    /// threshold value for a given iteration
    threshold: i64,
}

impl TabuTenure<Node, Node> for TabuColTenure {
    fn insert(&mut self, n:&Node, d:Node) {
        self.decisions[d.vertex][d.previous_color] = self.nb_iter;
        self.threshold = self.rng.i64(0..self.l as i64) + (self.lambda * (n.nb_conflicts as f64)) as i64;
    }

    fn contains(&mut self, _n:&Node, d:&Node) -> bool {
        self.decisions[d.vertex][d.next_color] >= self.nb_iter - self.threshold
    }
}

impl TabuColTenure {
    /** creates a tabucol tenure given:
     - l: fixed tabu size
     - λ: variable tabu size
     - n: the number of vertices in the graph
     - c: the maximum number of colors
    */
    pub fn new(l:usize, lambda: f64, n:usize, c:usize) -> Self {
        Self {
            l, lambda,
            nb_iter: 0,
            decisions: vec![vec![i64::MIN ; c] ; n],
            rng: Rng::new(),
            threshold: 0, // will be changed later
        }
    }

    /// increases the number of iterations of the tabu tenure
    pub fn increment_iter(&mut self) { self.nb_iter += 1; }
}

/** implements a conflict weighting local search */
#[derive(Debug)]
struct ConflictWeightingLocalSearch {
    /// instance object
    inst:Rc<dyn ColoringInstance>,
    /// weights[u][v]: weight learned for the edge (u,v)
    weights:Vec<Vec<Weight>>,
    /// current best feasible solution 
    current_sol:Vec<Vec<VertexId>>,
    /// colors[v]: color of vertex v
    colors:Vec<usize>,
    /// colors_bitset[c]: color c vertices
    colors_bitsets:Vec<BitSet>,
    /// colors_vertex_number[c]: number of vertices coloried with c
    colors_vertex_number:Vec<usize>,
    /// weights_neigh_colors[v][c]: weights of neighbors of v that are assigned color c
    weights_neigh_colors:Vec<Vec<Weight>>,
    /// conflicting_vertices: list of vertices that have some conflict
    conflicting_vertices:SparseSet,
    /// vertex_nb_conflicts[v]: number of conflicts for the vertex v
    vertex_nb_conflicts:Vec<i64>,
    /// number of conflicting edges
    nb_conflicting_edges:i64,
    /// total weight in the current state
    total_weight:Weight,
    /// tabu list
    tabu:TabuColTenure,
    /// threshold on the number of conflicts to disable the tabu tenure
    aspiration_criterion:i64,
    /// number of iterations
    nb_iter:i64,
    /// number of colors at the beginning of the search
    nb_colors:usize,
}

impl ConflictWeightingLocalSearch {

    /// initializes the data-structure from an initial solution 
    fn initialize(inst:Rc<dyn ColoringInstance>, sol:&[Vec<VertexId>]) -> Self {
        // build colors & colors_bitsets
        let n = inst.nb_vertices();
        let nb_colors = sol.len();
        let mut colors = vec![0 ; n];
        let mut colors_bitsets = vec![BitSet::with_capacity(n); nb_colors];
        let mut colors_vertex_number = vec![0 ; nb_colors];
        for (i,c) in sol.iter().enumerate() {
            for v in c {
                colors[*v] = i;
                colors_bitsets[i].insert(*v);
                colors_vertex_number[i] += 1;
            }
        }
        // build weights_neigh_colors
        let mut weights_neigh_colors = vec![vec![ 0 ; nb_colors] ; n];
        for u in inst.vertices() {
            for v in inst.neighbors(u) {
                weights_neigh_colors[u][colors[v]] += 1;
            }
        }
        Self {
            inst,
            weights: (0..n).map(|i| vec![1 ; i]).collect(),
            current_sol: sol.to_vec(),
            colors,
            colors_bitsets,
            colors_vertex_number,
            weights_neigh_colors,
            conflicting_vertices: SparseSet::new(n),
            vertex_nb_conflicts: vec![0 ; n],
            nb_conflicting_edges: 0,
            total_weight: 0,
            tabu: TabuColTenure::new(10, 0.6, n, nb_colors),
            aspiration_criterion: i64::MAX,
            nb_iter: 0,
            nb_colors
        }
    }

    /// merges 2 colors
    fn merge_colors(&mut self) {
        loop { // invariant: self.current_sol is feasible
            // find the best color merge (couple of colors that minimize the conflict penalties)
            let mut best_weight:Weight = Weight::MAX;
            let mut current_best_color_merge:Option<(VertexId,VertexId)> = None;
            for (i1, color1) in self.current_sol.iter().enumerate() {
                for (i2, color2) in self.current_sol.iter().enumerate() {
                    if i2 < i1 && !color1.is_empty() && !color2.is_empty() {
                        let current_weight_large:u32 = color1.iter().map(|u| self.weights_neigh_colors[*u][i2] as u32).sum();
                        let current_weight = if current_weight_large > u16::MAX as u32 { u16::MAX } else { current_weight_large as Weight };
                        if current_weight < best_weight {
                            best_weight = current_weight;
                            current_best_color_merge = Some((i2, i1));
                        }
                    }
                }
            }
            // perform the merge
            let (c1,c2) = current_best_color_merge.unwrap();
            let c_min; let c_max;
            if self.current_sol[c1] < self.current_sol[c2] {
                c_min = c1; c_max = c2;
            } else {
                c_min = c2; c_max = c1;
            }
            // for every vertex in the minimum color, change it to the maximum color
            for v in self.current_sol[c_min].clone() {
                self.change_vertex_color(v, c_max);
            }
            // quit if there are some conflicts, otherwise, repeat
            if self.is_goal() { // if goal, update the solution
                println!("merge found an improving solution: (new number of colors: {})", self.nb_colors());
                self.update_current_solution();
            } else { break; }
        }
    }

    /// find best move
    fn find_best_move(&mut self) -> Node {
        let mut best_node:Node = Node {
            vertex:0, previous_color:0, next_color:0, total_penalties:Weight::MAX, nb_conflicts:0
        };
        let mut i = 0;
        while i < self.conflicting_vertices.len() { // iterate over conflicting vertices
            let u = self.conflicting_vertices.nth(i);
            if self.vertex_nb_conflicts[u] > 0 { // u has indeed some conflicts
                // for each vertex, try changing its color by an existing other color
                for c in 0..self.nb_colors {
                    if c != self.colors[u] && self.colors_vertex_number[c] > 0 {
                        let current_penalties:Weight = self.total_weight +
                            self.weights_neigh_colors[u][c] - self.weights_neigh_colors[u][self.colors[u]];
                        if current_penalties < best_node.total_penalties { // find the best node
                            let current_node = Node {
                                vertex:u,
                                previous_color:self.colors[u],
                                next_color:c,
                                total_penalties:current_penalties,
                                nb_conflicts: self.nb_conflicting_edges
                            };
                            let is_tabu = self.tabu.contains(&current_node, &current_node);
                            if !is_tabu || self.nb_conflicting_edges < self.aspiration_criterion {
                                best_node = current_node; 
                            }
                        }
                    }
                }
                i += 1;
            } else {
                self.conflicting_vertices.remove(u); // update conflicting_vertices if it has no conflict
            }
        }
        best_node
    }

    /// applies a move (coloring a vertex with a color)
    fn commit(&mut self, node:Node) {
        // mark the move tabu
        self.tabu.insert(&node, node.clone()); // make the decision tabu
        self.tabu.increment_iter();
        self.nb_iter += 1;
        self.change_vertex_color(node.vertex, node.next_color);
    }


    /// change the color of vertex v, to color c
    fn change_vertex_color(&mut self, v:VertexId, next_color:usize) {
        let previous_color = self.colors[v];
        self.colors[v] = next_color; // change colors
        self.colors_bitsets[previous_color].remove(v);
        self.colors_bitsets[next_color].insert(v);
        self.colors_vertex_number[previous_color] -= 1;
        self.colors_vertex_number[next_color] += 1;
        self.total_weight = self.total_weight +
            self.weights_neigh_colors[v][next_color] - self.weights_neigh_colors[v][previous_color];
        // update weights & vertex_nb_conflicts
        for neigh in self.inst.neighbors(v) {
            self.weights_neigh_colors[neigh][previous_color] -= self.get_weight(neigh, v);
            self.weights_neigh_colors[neigh][next_color] += self.get_weight(neigh, v);
            if self.colors[neigh] == previous_color { // remove conflict
                self.vertex_nb_conflicts[neigh] -= 1;
                self.vertex_nb_conflicts[v] -= 1;
                self.nb_conflicting_edges -= 1;
            }
            if self.colors[neigh] == next_color { // add conflict
                self.vertex_nb_conflicts[neigh] += 1;
                self.vertex_nb_conflicts[v] += 1;
                self.nb_conflicting_edges += 1;
                // update weights when conflicts enter
                self.increase_weight(neigh, v);
                self.weights_neigh_colors[neigh][next_color] += 1;
                self.weights_neigh_colors[v][next_color] += 1;
                self.conflicting_vertices.insert(neigh);
                self.conflicting_vertices.insert(v);
                self.total_weight += 1;
            }
        }
        self.aspiration_criterion = std::cmp::min(self.aspiration_criterion, self.nb_conflicting_edges);
        if self.is_goal() { // if goal, update the solution
            self.update_current_solution();
        }
    }

    /// update the current solution
    fn update_current_solution(&mut self) {
        assert!(self.is_goal());
        let mut new_solution = vec![vec![] ; self.nb_colors];
        for (v,c) in self.colors.iter().enumerate() {
            new_solution[*c].push(v);
        }
        self.current_sol = new_solution;
    }

    /// get the learned weight of an edge
    fn get_weight(&self, u:VertexId, v:VertexId) -> Weight {
        if u < v { self.weights[v][u] }
        else { self.weights[u][v] }
    }

    /// increase the learned weight of an edge
    fn increase_weight(&mut self, u:VertexId, v:VertexId) {
        if u < v { self.weights[v][u] += 1 }
        else { self.weights[u][v] += 1 }
    }

    /// gets the current number of colors
    fn nb_colors(&self) -> usize { self.current_sol.iter().filter(|e| !e.is_empty()).count() }

    /// true iff state is feasible
    fn is_goal(&self) -> bool { self.total_weight == 0 }

    /// displays short statistics about the search
    fn display_log_line(&self, time:f32) {
        println!(" {:<15.3} it: {:<15} colors: {:<15} conflicts: {:<15} weight: {:<15}",
            time, self.nb_iter, self.nb_colors(), self.nb_conflicting_edges, self.total_weight
        );
    }

    /// get current solution
    fn get_solution(&self) -> Vec<Vec<VertexId>> {
        assert!(self.is_goal());
        self.current_sol.iter().filter(|e| !e.is_empty()).cloned().collect()
    }

}

/** performs a conflict weighting local search. */
pub fn conflict_weighting_local_search<Stopping:StoppingCriterion>(inst:Rc<dyn ColoringInstance>, sol:&[Vec<VertexId>], stop:Stopping, time_initial_solution:f32) -> Vec<Vec<VertexId>> {
    let time_init = Instant::now();
    let mut best_sol = sol.to_vec();
    let mut state = ConflictWeightingLocalSearch::initialize(inst, sol);
    println!("CWLS init time: {:.3}", time_init.elapsed().as_secs_f32());
    while !stop.is_finished() {
        // remove some colors
        // let time_merge = Instant::now();
        state.merge_colors();
        // println!("CWLS merge time: {:.3}", time_merge.elapsed().as_secs_f32());
        // repair
        // let time_repair = Instant::now();
        while !state.is_goal() && !stop.is_finished() {
            let node = state.find_best_move();
            state.commit(node);
            if state.nb_iter % 10000 == 0 {
                state.display_log_line(time_initial_solution+time_init.elapsed().as_secs_f32());
            }
        }
        if state.is_goal() { // new feasible solution
            // println!("{} colors", state.nb_colors());
            state.display_log_line(time_initial_solution+time_init.elapsed().as_secs_f32());
            best_sol = state.get_solution();
        }
        // println!("CWLS repair time: {:.3}", time_repair.elapsed().as_secs_f32());
    }
    best_sol
}


#[cfg(test)]
mod tests {
    use super::*;

    use dogs::search_algorithm::TimeStoppingCriterion;
    
    use crate::{cgshop::CGSHOPInstance, dimacs::DimacsInstance, search::greedy_dsatur::greedy_dsatur};

    #[test]
    fn test_cwls() {
        let time_init = Instant::now();
        let inst = Rc::new(CGSHOPInstance::from_file(
            // "./insts/cgshop22/reecn3382.instance.json"
            // "./insts/cgshop22/reecn9674.instance.json"
            // "./insts/cgshop22/reecn12588.instance.json"
            // "./insts/cgshop22/reecn31126.instance.json"
            // "./insts/cgshop22/reecn73116.instance.json"
            "./insts/cgshop22/rvispecn17968.instance.json"
            // "./insts/cgshop_22_examples/visp_5K.instance.json"
            // "./insts/cgshop_22_examples/sqrm_10K_5.instance.json"
        ));
        let greedy_sol = greedy_dsatur(inst.clone(), false);
        println!("initial solution: {}", greedy_sol.len());
        let stopping_criterion:TimeStoppingCriterion = TimeStoppingCriterion::new(3000.);
        conflict_weighting_local_search(inst, &greedy_sol, stopping_criterion, time_init.elapsed().as_secs_f32());
    }

    #[test]
    fn test_cwls2() {
        let time_init = Instant::now();
        let inst = Rc::new(CGSHOPInstance::from_file(
            // "./insts/cgshop22/reecn3382.instance.json"
            // "./insts/cgshop22/reecn9674.instance.json"
            // "./insts/cgshop22/reecn12588.instance.json"
            // "./insts/cgshop22/reecn31126.instance.json"
            // "./insts/cgshop22/reecn73116.instance.json"
            // "./insts/cgshop22/rvispecn17968.instance.json"
            "./insts/cgshop22/sqrp63650.instance.json"
            // "./insts/cgshop_22_examples/visp_5K.instance.json"
            // "./insts/cgshop_22_examples/sqrm_10K_5.instance.json"
        ));
        let greedy_sol = greedy_dsatur(inst.clone(), false);
        println!("initial solution: {}", greedy_sol.len());
        let stopping_criterion:TimeStoppingCriterion = TimeStoppingCriterion::new(3000.);
        conflict_weighting_local_search(inst, &greedy_sol, stopping_criterion, time_init.elapsed().as_secs_f32());
    }


    #[test]
    fn test_cwls3() {
        let time_init = Instant::now();
        let inst = Rc::new(CGSHOPInstance::from_file(
            // "./insts/cgshop22/reecn3382.instance.json"
            // "./insts/cgshop22/reecn9674.instance.json"
            // "./insts/cgshop22/reecn12588.instance.json"
            // "./insts/cgshop22/reecn31126.instance.json"
            // "./insts/cgshop22/reecn73116.instance.json"
            // "./insts/cgshop22/rvispecn17968.instance.json"
            // "./insts/cgshop22/sqrpecn71571.instance.json"
            "./insts/cgshop22/sqrp63650.instance.json"
            // "./insts/cgshop22/rvispecn13421.instance.json"
            // "./insts/cgshop_22_examples/visp_5K.instance.json"
            // "./insts/cgshop_22_examples/sqrm_10K_5.instance.json"
        ));
        let greedy_sol = greedy_dsatur(inst.clone(), false);
        println!("initial solution: {}", greedy_sol.len());
        let stopping_criterion:TimeStoppingCriterion = TimeStoppingCriterion::new(3000.);
        conflict_weighting_local_search(inst, &greedy_sol, stopping_criterion, time_init.elapsed().as_secs_f32());
    }


    #[test]
    fn test_cwls4() {
        let time_init = Instant::now();
        let inst = Rc::new(CGSHOPInstance::from_file(
            // "./insts/cgshop22/reecn3382.instance.json"
            // "./insts/cgshop22/vispecn2518.instance.json"
            // "./insts/cgshop22/reecn9674.instance.json"
            // "./insts/cgshop22/reecn12588.instance.json"
            // "./insts/cgshop22/reecn31126.instance.json"
            // "./insts/cgshop22/reecn73116.instance.json"
            // "./insts/cgshop22/rvispecn17968.instance.json"
            // "./insts/cgshop22/vispecn74166.instance.json"
            // "./insts/cgshop22/sqrp73525.instance.json"
            // "./insts/cgshop22/sqrpecn71571.instance.json"
            "./insts/cgshop22/rvispecn13421.instance.json"
            // "./insts/cgshop_22_examples/visp_5K.instance.json"
            // "./insts/cgshop_22_examples/sqrm_10K_5.instance.json"
        ));
        let greedy_sol = greedy_dsatur(inst.clone(), false);
        println!("initial solution: {}", greedy_sol.len());
        let stopping_criterion:TimeStoppingCriterion = TimeStoppingCriterion::new(30.);
        conflict_weighting_local_search(inst, &greedy_sol, stopping_criterion, time_init.elapsed().as_secs_f32());
    }


    #[test]
    fn test_cwls5() {
        let time_init = Instant::now();
        let inst = Rc::new(DimacsInstance::from_file(
            "insts/instances-dimacs1/DSJC1000.9.col"
        ));
        let greedy_sol = greedy_dsatur(inst.clone(), false);
        println!("initial solution: {}", greedy_sol.len());
        let stopping_criterion:TimeStoppingCriterion = TimeStoppingCriterion::new(30.);
        conflict_weighting_local_search(inst, &greedy_sol, stopping_criterion, time_init.elapsed().as_secs_f32());
    }

}



/*
$ i=rvispecn17968 && ./bazel-bin/minimumplanepartitionsolver/main -v -i "./data/cgshop2022/instances/${i}.instance.json" --reduce -c "${i}.solution.json" -a localsearch_rowweighting
Instance:            rvispecn17968
Number of vertices:  27058
Number of edges:     17968
Maximum degree:      6
Number of dominated edges: 156
*** localsearch_rowweighting ***
T (s)       UB          LB          GAP         GAP (%)     
127.063     17969       0           17969       inf         
127.961     269         0           269         inf         greedy orientation
134.956     243         0           243         inf         greedy dsatur
139.776     242         0           242         inf         
139.89      241         0           241         inf         
139.975     240         0           240         inf         
140.083     239         0           239         inf         
140.168     238         0           238         inf         
140.258     237         0           237         inf         
140.371     236         0           236         inf         
140.463     235         0           235         inf         
140.554     234         0           234         inf         
140.67      233         0           233         inf         
140.768     232         0           232         inf         
140.853     231         0           231         inf         
140.968     230         0           230         inf         
141.057     229         0           229         inf         
141.153     228         0           228         inf         
141.262     227         0           227         inf         
141.374     226         0           226         inf         
141.476     225         0           225         inf         
141.636     224         0           224         inf         
141.798     223         0           223         inf         
141.937     222         0           222         inf         
142.419     221         0           221         inf         
370.894     220         0           220         inf         
374.453     219         0           219         inf         
374.783     218         0           218         inf         
386.475     217         0           217         inf         
386.981     216         0           216         inf         
388.408     215         0           215         inf         
389.525     214         0           214         inf         
390.75      213         0           213         inf         
452.31      212         0           212         inf         
455.062     211         0           211         inf         
456.066     210         0           210         inf         
461.661     209         0           209         inf         
466.111     208         0           208         inf         
467.358     207         0           207         inf         
476.46      206         0           206         inf         
478.112     205         0           205         inf         
481.611     204         0           204         inf         
485.795     203         0           203         inf         
493.377     202         0           202         inf         
503.736     201         0           201         inf         
552.916     200         0           200         inf         
559.894     199         0           199         inf         
577.864     198         0           198         inf         
582.352     197         0           197         inf         
593.217     196         0           196         inf         
721.128     195         0           195         inf         
779.644     194         0           194         inf         
834.019     193         0           193         inf         
846.482     192         0           192         inf         
873.633     191         0           191         inf         
999.835     190         0           190         inf         
1138.08     189         0           189         inf         
1156.72     188         0           188         inf         
1199.65     187         0           187         inf         
1523.91     186         0           186         inf         
1576.8      185         0           185         inf         
1640.89     184         0           184         inf         
2621.6      183         0           183         inf         
3127.13     182         0           182         inf         
3581.15     181         0           181         inf         
7210.1      180         0           180         inf         
14495.4     179         0           179         inf  
*/

/*
$ i=rvispecn13421 && ./bazel-bin/minimumplanepartitionsolver/main -v -i "./data/cgshop2022/instances/${i}.instance.json" --reduce -c "${i}.solution.json" -a localsearch_rowweighting
Instance:            rvispecn13421
Number of vertices:  20102
Number of edges:     13421
Maximum degree:      6
Number of dominated edges: 146
*** localsearch_rowweighting ***
T (s)       UB          LB          GAP         GAP (%)     
100.163     13422       0           13422       inf         
101.031     249         0           249         inf         greedy orientation
108.513     227         0           227         inf         greedy dsatur
115.013     226         0           226         inf         
115.091     225         0           225         inf         
115.216     224         0           224         inf         
115.381     223         0           223         inf         
115.628     222         0           222         inf         
115.868     221         0           221         inf         
116.034     220         0           220         inf         
116.269     219         0           219         inf         
116.515     218         0           218         inf         
116.699     217         0           217         inf         
116.931     216         0           216         inf         
117.191     215         0           215         inf         
117.37      214         0           214         inf         
117.632     213         0           213         inf         
117.858     212         0           212         inf         
118.134     211         0           211         inf         
118.443     210         0           210         inf         
118.778     209         0           209         inf         
119.116     208         0           208         inf         
119.512     207         0           207         inf         
119.971     206         0           206         inf         
120.545     205         0           205         inf         
121.405     204         0           204         inf         
121.77      203         0           203         inf         
122.04      202         0           202         inf         
604.621     201         0           201         inf         
605.423     200         0           200         inf         
606.777     199         0           199         inf         
609.211     198         0           198         inf         
610.469     197         0           197         inf         
613.23      196         0           196         inf         
614.602     195         0           195         inf         
616.379     194         0           194         inf         
618.733     193         0           193         inf         
621.425     192         0           192         inf         
641.171     191         0           191         inf         
791.344     190         0           190         inf         
796.652     189         0           189         inf         
797.495     188         0           188         inf         
801.274     187         0           187         inf         
806.64      186         0           186         inf         
808.79      185         0           185         inf         
811.157     184         0           184         inf         
828.807     183         0           183         inf         
835.656     182         0           182         inf         
840.805     181         0           181         inf         
849.538     180         0           180         inf         
863.741     179         0           179         inf         
880.247     178         0           178         inf         
883.37      177         0           177         inf         
1303.66     176         0           176         inf         
1338.04     175         0           175         inf         
1375.71     174         0           174         inf         
1469.92     173         0           173         inf         
1508.48     172         0           172         inf         
1532.34     171         0           171         inf         
1587.05     170         0           170         inf         
1799.16     169         0           169         inf         
1978.01     168         0           168         inf         
2254.8      167         0           167         inf         
2369.95     166         0           166         inf         
2422.98     165         0           165         inf         
3342.01     164         0           164         inf         
3520.31     163         0           163         inf         
3639.35     162         0           162         inf     
 */