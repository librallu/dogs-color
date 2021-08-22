/** PARTIALCOL algorithm (see: 10.1016/j.cor.2006.05.014)
 - Tabu search that explores a neighborhood of feasible solutions minimizinc the number of
   unassigned solutions. Selects an unassigned vertex and a color. Color this vertex with
   the color, and unassign adjacent vertices with the same color.
*/


/**
Implements a search tree node.
Stores a decision and a number of conflicts
*/
#[derive(Debug,Clone)]
pub struct Node {
    /// decision taken by the node
    decision:Option<Decision>,
    /// number of uncolored vertices
    nb_uncolored: usize,
}


/** (see https://doi.org/10.1016/j.cor.2006.05.014)
Implements a local search procedure for the graph coloring (PARTIALCOL).
Starts with an initial solution using a DSATUR like algorithm (TODO: needs more details).
Then tries to assign vertices to colors, uncoloring conflicting vertices. 
*/
#[derive(Debug)]
pub struct SearchState {
    /// reference instance
    inst: Rc<Instance>,
    /// colors[v]: color of the vertex v
    colors: Vec<usize>,
    /// number of colors used
    nb_colors: usize,
    /// nb_neigh_colors[v][c]: number of neighbors of v that are assigned color c
    nb_neigh_colors: Vec<Vec<usize>>,
}

impl SearchState {

    /** Creates a new search state using a modified DSATUR procedure possibly letting some
    vertices uncolored. */
    pub fn random_solution(inst:Rc<Instance>, nb_colors:usize) -> Self {
        todo!();
    }
}