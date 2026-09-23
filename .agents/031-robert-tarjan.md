# 031 — Robert Tarjan

**Cluster:** C5 — Graph engine & algorithms
**Roster role:** SCC/cycle algorithms, graph data structures
**SpecForge anchors:** crates/specforge-graph/src/graph.rs `detect_cycles`/`has_cycles` (three-color DFS, W061 reference cycles); crates/specforge-resolver/src/resolve.rs import-graph cycle DFS (W003) + Kahn topological sort; extension-declared cycle rules E007/E015/E016 (crates/specforge-emitter/src/builtins/product.rs)

## Why this engineer
SpecForge enforces acyclicity in three independent places — file imports (W003), entity references (W061), and module/milestone/deliverable dependency cycles (E007/E015/E016) — each with an ad-hoc DFS that reports one path at a time. Tarjan invented the linear-time SCC algorithm that upgrades all three to principled machinery: collapse the graph to its condensation DAG, report full cycle membership, and reuse the topological order for emission and for specforge-watch's incremental invalidation. His integer-index adjacency style is the pattern specforge-graph's `edges: Vec<Edge>` + `source_index` storage already imitates.

## References for SpecForge
**Key works**
- **Depth-First Search and Linear Graph Algorithms** — SIAM Journal on Computing 1(2):146–160, 1972. The original linear-time SCC algorithm via lowlinks on a DFS stack — the direct upgrade path for `detect_cycles`.
- **Data Structures and Network Algorithms** — SIAM CBMS-NSF Regional Conference Series in Applied Mathematics 44, 1983. Canonical book treatment of SCC condensation and topological ordering; the theory behind emit-order and DAG invalidation.
- **Algorithm 447: Efficient Algorithms for Graph Manipulation** (with John Hopcroft) — Communications of the ACM 16(6), 1973. Linear-time graph manipulation from a single DFS pass; template for one-pass diagnostics.
- **Amortized Computational Complexity** — SIAM Journal on Algebraic and Discrete Methods 6(2), 1985. The amortized-analysis framing for cheap incremental re-checks in specforge-watch.

## Study first
1. The 1972 DFS paper: SCC via lowlink + stack, linear time
2. Condensation DAG: SCC collapse → topological order shared by emit and watch
3. Data Structures and Network Algorithms chapter on strong components
