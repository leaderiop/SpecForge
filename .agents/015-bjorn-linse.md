# 015 — Björn Linse (bfredl)

**Cluster:** C3 — Parsing & grammar infrastructure
**Roster role:** Neovim core; designed its tree-sitter integration and incremental reparse pipeline
**SpecForge anchors:** parse_incremental in crates/specforge-parser/src/parse.rs; incremental invalidation in crates/specforge-watch (delta.rs, import_dag.rs, debounce.rs, subscribers.rs)

## Why this engineer
Linse took tree-sitter from library to editor fabric: Neovim's integration (shipped experimental in 0.5, hardened through his follow-ups) made per-keystroke incremental reparse, query-driven highlighting, and the decorations API feel native. SpecForge's watch daemon is the same problem in miniature — on save, delta.rs computes the change, parse_incremental threads the old tree back through the parser, and import_dag.rs invalidates only affected files. Neovim's hard-won rules — build a correct InputEdit, reparse before observing, debounce bursts, cache query results — are the exact correctness and performance contracts the watch pipeline must honor as the 219-file corpus grows.

## References for SpecForge
**Key works**
- [Treesitter — Neovim user docs](https://neovim.io/doc/user/treesitter/) — official docs, 2021–present. The stable API (parsers, Query, highlighter) that grew out of his integration; a template for specforge-lsp's incremental cycle.
- [Tree-sitter discussion #11724](https://github.com/neovim/neovim/discussions/11724) — neovim/neovim, 2020. bfredl's own integration thread: parser loading, buffer edits → tree edits, rollout strategy.
- [NVIM v0.5.0](https://github.com/neovim/neovim/releases/tag/v0.5.0) — GitHub release, 2021. Ships the experimental tree-sitter core: incremental parsing wired to buffer events.
- [bfredl](https://github.com/bfredl) — GitHub profile. Neovim core maintainer: tree-sitter, extmarks/decorations, Lua performance.
- [nvim-treesitter](https://github.com/nvim-treesitter/nvim-treesitter) — community parser/query manager. The install-and-update model worth studying against RES-30's runtime-grammar question.

## Study first
1. Buffer edit → InputEdit → Parser.parse(old_tree): the discipline parse_incremental requires from future callers
2. Neovim's highlighter loop (attach/detach, incremental query runs) — a template for watch subscribers.rs
3. Redraw throttling under rapid typing — mirrors debounce.rs tuning in specforge-watch
