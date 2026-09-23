# 024 — Muir Manders

**Cluster:** C4 — Incremental compilation, LSP, diagnostics UX
**Roster role:** gopls core contributor; completion engine
**SpecForge anchors:** LSP workspace indexing (crates/specforge-lsp/src/state.rs, symbols.rs), completion.rs

## Why this engineer
Manders is one of gopls's most prolific core contributors, best known for reworking its completion engine — candidate ranking, fuzzy matching, and hard latency budgets so completions return fast even when deep semantic analysis is still running. SpecForge's `completion.rs` faces the same constraint: registry-backed keyword/field suggestions over the graph must stay interactive while the workspace index (state.rs) may be stale or mid-recompute.

## References for SpecForge
**Key works**
- [golang/tools (gopls)](https://github.com/golang/tools) — GitHub, 2019. His completion work shows how to serve ranked results from partial state and reconcile cheap syntactic candidates with expensive semantic ones.
- [Static Analysis in Go — Muir Manders](https://www.youtube.com/watch?v=YhmeKW3Yw_8) — GDG DevFest Gorky, 2018. Walkthrough of building analysis on go/ast, the mindset for tree-sitter-based SpecForge completions.

## Study first
1. gopls completion budgeting: bounding work per request, degrading gracefully
2. Ranking/scoring candidates before semantic filtering — applicable to registry field completions
3. Splitting syntactic (instant) from semantic (deferred) completion sources
