# 018 — Aleksey Kladov (matklad)

**Cluster:** C4 — Incremental compilation, LSP, diagnostics UX
**Roster role:** rust-analyzer creator; resilient parsing & shared IDE pipeline
**SpecForge anchors:** crates/specforge-watch (IncrementalPipeline shared by watch + LSP), specforge-parser parse_incremental

## Why this engineer
Kladov created rust-analyzer from scratch around one principle: the IDE and the compiler must share a single incremental pipeline, not two codepaths — precisely SpecForge's goal for `IncrementalPipeline` serving both `specforge-watch` and the LSP. His writing on resilient parsing (produce a tree for every input) justifies `parse_incremental` keeping diagnostics flowing from partially-broken `.spec` files.

## References for SpecForge
**Key works**
- [rust-lang/rust-analyzer](https://github.com/rust-lang/rust-analyzer) — GitHub, 2018. The reference IDE-compiler built on salsa: parse → index → on-demand queries, the layering to emulate in watch+LSP.
- [rust-analyzer docs/dev/architecture.md](https://github.com/rust-lang/rust-analyzer/blob/master/docs/dev/architecture.md) — project doc. Concise statement of "same core serves IDE and batch compile" — the exact contract for IncrementalPipeline.
- [matklad.github.io](https://matklad.github.io/) — blog, 2016–. Essays on resilient parsers, 16ms latency budgets, and incremental architecture from the implementer's seat.

## Study first
1. rust-analyzer architecture doc: parse → index → query layering
2. Resilient parsing: never fail, always produce a tree with errors attached
3. matklad's latency-budget essays vs specforge-watch debounce windows
