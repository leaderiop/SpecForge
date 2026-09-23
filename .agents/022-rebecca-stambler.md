# 022 — Rebecca Stambler

**Cluster:** C4 — Incremental compilation, LSP, diagnostics UX
**Roster role:** gopls lead; unified Go language-server workspace
**SpecForge anchors:** LSP workspace indexing (crates/specforge-lsp/src/state.rs, document.rs, symbols.rs)

## Why this engineer
Stambler led the creation of gopls, collapsing a zoo of Go tools (guru, godef, gocode) into one LSP server with a single workspace package graph — the same consolidation SpecForge performs by giving editors one registry-aware indexed view over `.spec` files instead of per-tool parsers. Her talk and writing explain why a language server needs a coherent snapshot model (state.rs's job) before any hover or completion can be trusted.

## References for SpecForge
**Key works**
- [golang/tools (gopls)](https://github.com/golang/tools) — GitHub, 2019. The Go language server Stambler architected; its snapshot/package-graph model is the reference for SpecForge workspace indexing.
- Go, pls stop breaking my editor — GopherCon, 2019. Retrospective on replacing fragmented tools with one server: reliability, a testing framework, and team ownership as requirements, not features.
- gopls release notes & design docs (gopls/doc) — Go team, 2019–. Running case study in shipping LSP UX decisions (diagnostics toggles, workspace symbols) that editors inherit unchanged.

## Study first
1. gopls snapshot model: how state.rs should version documents vs on-disk files
2. Why one workspace graph beat per-feature indexes
3. gopls's regression-test harness for LSP behavior
