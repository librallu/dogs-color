# Dogs-color
Discrete Optimization Global Search for the Graph Coloring problem

# What's next?

- [X] read latest DIMACS format (p edge)
- [ ] read previous DIMACS format (p col)
- [ ] more instance metrics (see https://www.sciencedirect.com/science/article/pii/S0305054813003389?via%3Dihub)
- [ ] RLF implementation (see https://www.gerad.ca/~alainh/RLFPaper.pdf)
- [X] DSATUR branch-and-bound base implementation
- [ ] Backtracking DSATUR (see https://webdocs.cs.ualberta.ca/~joe/Coloring/)
- [ ] TabuCol
    - [X] branching
    - [X] DOGS greedy call
    - [X] tabu list
    - [X] aspiration criterion (improves the bks)
    - [ ] efficient L+λ tabu tenure:
          keep the tabu moves from the L+λ.F(c) last iterations,
          where F(c) is the current number of conflicts)
    - [ ] efficient move data structures
- [ ] Genetic Local Search (example: https://github.com/Adamovskiy/genetic-vertex-coloring/blob/master/GeneticVertexColoring.c)
- [ ] other local search moves (hillclimbing, partialcol)
- [ ] probability learning algorithms