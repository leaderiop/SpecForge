# 035 — Agustín Borgna (ABorgna)

**Cluster:** C5 — Graph engine & algorithms
**Roster role:** petgraph maintainer
**SpecForge anchors:** crates/specforge-graph/src/graph.rs node/edge indexes (`nodes: HashMap<Sym, Node>`, `edges: Vec<Edge>`, `source_index`/`target_index`); workspace `petgraph = "0.8.3"` (root Cargo.toml); typed edge labels (`Edge { source, target, label: Sym }`) vs extension edge registries

## Why this engineer
Borgna is the current petgraph maintainer — verified as the crates.io publisher of the 0.8.x line and member of the petgraph GitHub org — and he maintains the graph stack daily from inside a real compiler project: Quantinuum's TKET2 quantum compiler (Cambridge, UK), where his portgraph crate adds port-typed edges to petgraph-style storage. That combination is exactly SpecForge's open decision: whether to adopt petgraph 0.8.3 (already in workspace deps) instead of the hand-rolled HashMap+Vec indexes in graph.rs, and how to type edges — portgraph's port semantics are the production precedent for SpecForge's `Edge { source, target, label }` with registry-checked labels. His ZX-calculus work (quizx) proves large-graph rewriting on this stack.

## References for SpecForge
**Key works**
- [petgraph](https://github.com/petgraph/petgraph) — canonical repo he maintains; current 0.8.x publisher on crates.io. The library staged in SpecForge's workspace for the graph engine.
- [portgraph](https://crates.io/crates/portgraph) — his ports-aware graph crate: typed connection points on nodes — the closest published analogue to SpecForge's labeled, extension-validated edges.
- [quizx](https://github.com/zxcalc/quizx) — ZX-calculus graph rewriting over petgraph/portgraph: evidence the stack holds up under heavy graph-to-graph transformation.
- [TKET2](https://github.com/CQCL/tket2) — the quantum compiler he works on at Quantinuum: a production consumer of this graph toolchain.

## Study first
1. petgraph 0.8 release notes: what the maintainer considers stable API surface
2. portgraph: how port-typed edges map onto SpecForge's label/edge-type registry
3. quizx rewrite architecture for future model-to-model transformations
