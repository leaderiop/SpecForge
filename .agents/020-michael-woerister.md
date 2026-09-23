# 020 — Michael Woerister

**Cluster:** C4 — Incremental compilation, LSP, diagnostics UX
**Roster role:** rustc incremental compilation designer
**SpecForge anchors:** IncrementalPipeline (crates/specforge-watch/src/pipeline.rs), import_dag.rs DAG invalidation, delta.rs

## Why this engineer
Woerister designed rustc's incremental compilation: the dep-graph, red-green marking, and hash-based change detection that make recompiles proportional to the edit, not the project. SpecForge's `IncrementalPipeline` faces the same shape at DSL scale — a changed `.spec` file must invalidate only the downstream nodes of the import DAG, reusing the petgraph result otherwise. His design is the proven blueprint for correctness of that invalidation.

## References for SpecForge
**Key works**
- [Incremental Compilation](https://blog.rust-lang.org/2016/09/08/incremental.html) — The Rust Blog, 2016. The founding write-up: dep-graph, fingerprinting, and why MIR-level reuse beats unit-level rebuilds.
- Incremental Compilation chapter — The Rustc Book / rustc-dev-guide. The maintained spec of the dep-graph and red-green algorithm; the checklist for auditing import_dag.rs invalidation soundness.
- [Rust's incremental compiler architecture](https://lwn.net/) — LWN.net, 2024. Independent retrospective on what held up and what didn't after years in production.

## Study first
1. Dep-graph + red-green marking: proving an invalidation is neither too big nor too small
2. Fingerprinting: hashing query inputs to detect semantic (not textual) change
3. LWN retrospective: failure modes of incremental systems in practice
