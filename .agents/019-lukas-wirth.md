# 019 — Lukas Wirth

**Cluster:** C4 — Incremental compilation, LSP, diagnostics UX
**Roster role:** rust-analyzer lead maintainer; salsa co-maintainer
**SpecForge anchors:** crates/specforge-watch (shared incremental pipeline), specforge-parser parse_incremental, specforge-lsp state.rs

## Why this engineer
Wirth is the current rust-analyzer lead and co-maintains salsa, so he has owned both ends of the problem SpecForge faces: keeping an incremental engine (salsa durability/hydration work) honest while an IDE hammers it with queries. His experience deciding what gets recomputed vs reused when a file changes maps directly onto `IncrementalPipeline`'s delta propagation and the watch/LSP shared pipeline.

## References for SpecForge
**Key works**
- [rust-lang/rust-analyzer](https://github.com/rust-lang/rust-analyzer) — GitHub, 2018. Where Wirth drove diagnostics→quickfix flow and query-based recomputation under real editor load.
- [salsa-rs/salsa](https://github.com/salsa-rs/salsa) — GitHub, 2018. On-demand incremental framework; his durability/hydration work is the state of the art in "don't recompute what didn't change".
- Rust Analyzer — Rustacean Station podcast, 2023. The maintainer's view of running a language server at scale, including contributor-facing design discipline.
- Durable Incrementality — The rust-analyzer blog, 2023. Persisting query results across restarts — a model for SpecForge caching grammar/registry state across watch sessions.

## Study first
1. Salsa durability & hydration: persisting incremental state
2. rust-analyzer diagnostics → code-action pipeline
3. How salsa decides "changed": red-green propagation vs IncrementalPipeline's DAG invalidation
