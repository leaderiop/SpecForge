# 021 — Niko Matsakis

**Cluster:** C4 — Incremental compilation, LSP, diagnostics UX
**Roster role:** Rust language co-lead; salsa incremental framework creator
**SpecForge anchors:** salsa-style query model for graph recompute (crates/specforge-graph, crates/specforge-watch)

## Why this engineer
Matsakis led rustc's query-system work and then extracted it into salsa — incremental recompilation as a general, declarative framework. That is the natural evolution path for SpecForge's graph recompute: instead of hand-managed `IncrementalPipeline` invalidation, express resolve→graph→validate as pure queries keyed by file inputs, and let the framework memoize for watch, LSP and CLI simultaneously. Senior Principal Engineer at AWS and Rust language-design co-lead.

## References for SpecForge
**Key works**
- [salsa-rs/salsa](https://github.com/salsa-rs/salsa) — GitHub, 2018. Generic on-demand incrementalized computation, inspired by adapton and rustc's query system — the exact dependency SpecForge would add for query-based graph recompute.
- [The Salsa book](https://salsa-rs.github.io/salsa/) — salsa-rs, ongoing. Tutorial explaining inputs, interning, and cycle recovery — cycles matter for SpecForge since the resolver must detect import cycles.
- [Salsa in 2019: incremental recompilation](https://smallcultfollowing.com/babysteps/) — baby steps blog, 2019. Matsakis's own account of extracting rustc's incremental techniques into a reusable framework.

## Study first
1. Salsa query model: inputs, derived queries, LRU memoization
2. Cycle handling in a query framework vs specforge-graph's explicit cycle detection
3. Mapping IncrementalPipeline's invalidation onto salsa's red-green propagation
