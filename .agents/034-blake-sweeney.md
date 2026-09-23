# 034 — bluss (Ulrik Sverdrup)

**Cluster:** C5 — Graph engine & algorithms
**Roster role:** petgraph author; index-based graph data structures in Rust
**SpecForge anchors:** crates/specforge-graph/src/graph.rs node/edge indexes (`nodes: HashMap<Sym, Node>`, `edges: Vec<Edge>`, `source_index`/`target_index` adjacency); workspace `petgraph = "0.8.3"` (root Cargo.toml, staged for adoption); scope subgraph extraction (crates/specforge-emitter/src/scope.rs)

## Why this engineer
Roster note: assigned as "Blake Sweeney"; bluss's own GitHub README identifies the person behind the handle as **Ulrik Sverdrup**, original author of petgraph. SpecForge's graph engine hand-rolls petgraph's core design — slot storage addressed by interned `Sym` node/edge indexes, adjacency rebuilt via source/target index maps — and already declares petgraph 0.8.3 in the workspace without consuming it. Sverdrup's crates show how the real thing does it: typed `Ix` indexes with size-based generics, stable free-slot reuse after removal (the semantics specforge-watch's invalidation needs), and order-preserving maps (indexmap) that keep emitted output deterministic. His M.Sc.-physics engineer discipline — small, correct, benchmarked primitives — is the standard for graph.rs.

## References for SpecForge
**Key works**
- [petgraph](https://github.com/petgraph/petgraph) — canonical repo he created. `Graph`/`StableGraph` with typed node/edge indexes: the reference design for graph.rs and the candidate to replace it.
- [indexmap](https://github.com/indexmap-rs/indexmap) — his order-preserving hash map. The pattern for deterministic, insertion-ordered node tables behind stable emit output.
- [ndarray](https://github.com/rust-ndarray/ndarray) — his array-view design: zero-copy views over owned buffers, the philosophy behind cheap subgraph/scope extraction.

## Study first
1. petgraph `Graph`/`StableGraph`: typed `Ix` indexes and free-slot reuse on removal
2. petgraph's cycle/SCC and toposort modules vs graph.rs's hand-rolled DFS
3. indexmap internals: why order preservation buys deterministic diagnostics
