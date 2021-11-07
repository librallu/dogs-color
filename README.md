# Dogs-color
Discrete Optimization Global Search for the Graph Coloring problem


# Features

- [X] DIMACS format reading
- [X] SOCG22 competition instance & solution format reading & writing
- [X] custom DSATUR algorithm (less memory requirement)

# Scripts

- [ ] Update the best color given a solution file

# Preprocessing

- [X] Dominated vertices: $u$ dominates $v$ if $N(v) \subseteq N(u)$. In this case, $v$ can be safely removed and
      colored using the same color as $u$.
- [ ] Consider a lower bound $B$ on the number of colors (for instance obtained using a CLIQUE algorithm).
      We can safely remove all the vertices where $\delta(v) < B-1$ as it can be colored using colors 1 to $B$.
- [ ] Instance abstraction to use preprocessing 
- [ ] Store coloring / cliques to reuse them later if needed

# Algorithms

- [X] DSATUR greedy
- [X] RLF greedy (see https://www.gerad.ca/~alainh/RLFPaper.pdf)
- [X] simple CLIQUE greedy algorithm
- [X] TabuCol algorithm (https://link.springer.com/content/pdf/10.1007/BF02239976.pdf)
- [X] PARTIALCOL (see https://doi.org/10.1016/j.cor.2006.05.014)
- [X] Backtracking DSATUR (see https://webdocs.cs.ualberta.ca/~joe/Coloring/)

# What's next (Coloring instances)?

- [ ] Compute more instance metrics (see https://doi.org/10.1016/j.cor.2013.11.015)

# What's next (CGSHOP)?

- [ ] RLF like (but inverting orientation iteration at each color)

# What's next (algorithms)?

- [ ] MACOL (see: http://azadproject.ir/wp-content/uploads/2013/12/2010-O6-A-memetic-algorithm-for-graph-coloring.pdf)
- [ ] other local search moves (hill climbing)
- [ ] EXTRACOL (see: 10.1016/j.cor.2011.04.002)
- [ ] PLSCOL probability learning algorithms (https://doi.org/10.1016/j.asoc.2018.01.027)
- [ ] multi-level algorithm (see: https://doi.org/10.1016/j.asoc.2021.107174)
