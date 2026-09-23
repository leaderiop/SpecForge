# 110 — John Ellson

**Cluster:** C13 — Diagrams & rendering
**Roster role:** Graphviz/DOT co-creator; tooling, editors, release engineering
**SpecForge anchors:** `emit --format=dot` (crates/specforge-emitter/src/dot.rs), specforge-graph → dot rendering (crates/specforge-emitter/src/outline/dot.rs), extension-rendering vocabulary (dot_shape/dot_color/dot_fillcolor in emitter builtins)

## Why this engineer
Ellson co-built Graphviz into what SpecForge implicitly relies on: a stable DOT text format consumed by independent layout tools. His work on lefty/dotty/tcldot and Graphviz's plugin/codegen architecture is the canonical example of separating a graph *data model* from interchangeable renderers — the exact seam SpecForge draws between specforge-graph's in-memory petgraph and the emitter's DOT strings. His long run as release manager shows what it takes to keep an output format backward-compatible for decades.

## References for SpecForge
**Key works**
- [Graphviz and Dynagraph — Static and Dynamic Graph Drawing Tools](https://graphviz.org/documentation/EGKNW03.pdf) (with Gansner, Koutsofios, North, Woodhull) — Graph Drawing Software (Springer), 2003. Defines the batch-format → layout → render pipeline SpecForge's emitter mimics.
- [Graphviz](https://graphviz.org) — open-source graph visualization site, 1991–. The DOT dialect SpecForge's `emit_dot` must stay compatible with.
- [Graphviz source](https://gitlab.com/graphviz/graphviz) — GitLab (GitHub mirror graphviz/graphviz). Reference for plugin/codegen renderer architecture.
- [Graphviz credits](https://graphviz.org/credits/) — documents the role split (Ellson: Tcl/Tk, codegen, plugins, build) behind a multi-decade toolchain.

## Study first
1. Graphviz's plugin/codegen renderer architecture
2. lefty's editable-graph model vs batch `dot`
3. Release engineering of a long-lived, format-stable C codebase
