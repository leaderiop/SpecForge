# 112 — Stephen North

**Cluster:** C13 — Diagrams & rendering
**Roster role:** Graphviz project lead; graph data model (cgraph) and large-graph applications
**SpecForge anchors:** `emit --format=dot` (crates/specforge-emitter/src/dot.rs), specforge-graph → dot rendering (crates/specforge-emitter/src/outline/dot.rs), petgraph graph model in crates/specforge-graph

## Why this engineer
North led Graphviz as a library-plus-tools platform, separating the `cgraph` graph data model from pluggable layout engines and renderers — the same split SpecForge makes between specforge-graph (nodes/edges, cycle detection, subgraphs) and the emitters that stringify views of it. GN99 frames DOT output as software-engineering visualization (call graphs, state machines, dependency graphs); SpecForge does precisely this for .spec corpora, so his system-level papers are the blueprint for what a *view of a model graph* should and should not promise.

## References for SpecForge
**Key works**
- [An open graph visualization system and its applications to software engineering](https://graphviz.org/documentation/GN99.pdf) (with Gansner) — Software: Practice and Experience, 1999. Architecture of format/layout/renderer separation and attribute language; the model behind SpecForge's emitter/graph split.
- [Graphviz and Dynagraph — Static and Dynamic Graph Drawing Tools](https://graphviz.org/documentation/EGKNW03.pdf) (with Ellson, Gansner, Koutsofios, Woodhull) — Graph Drawing Software (Springer), 2003. Batch tools vs incremental rendering — informs specforge-watch's invalidation story.
- **Applications of graph visualization** (Koutsofios, North) — Graphics Interface, 1991. Early taxonomy of SE uses for drawn graphs.
- [Graphviz](https://graphviz.org) — project site and [source](https://gitlab.com/graphviz/graphviz).

## Study first
1. GN99: data model vs layout vs renderer separation
2. cgraph's attribute model (graph-level defaults, per-node overrides)
3. Deterministic rendering practices for regression-tested output
