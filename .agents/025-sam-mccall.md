# 025 — Sam McCall

**Cluster:** C4 — Incremental compilation, LSP, diagnostics UX
**Roster role:** clangd creator & maintainer; LSP at C-scale performance
**SpecForge anchors:** LSP performance (crates/specforge-lsp grammar_cache.rs, state.rs), IncrementalPipeline over the spec/ corpus

## Why this engineer
McCall created and maintains clangd, the reference language server for C++ — a language where a full compile is so slow that the server survives by separating cheap syntactic views (preamble, tree) from an asynchronous semantic index that streams diagnostics later. SpecForge inherits the same asymmetry at smaller scale: grammar_cache and hover must answer instantly while IncrementalPipeline resolves the graph in the background. clangd is the study in scheduling around expensive analysis, and in memory-bounding the index of large workspaces.

## References for SpecForge
**Key works**
- [clangd (llvm-project)](https://github.com/llvm/llvm-project/tree/main/clang-tools-extra/clangd) — GitHub/LLVM, 2017. Background index, preamble reuse, and out-of-order request handling — the C++-scale answers to SpecForge's pipeline-latency problems.
- [Clang Tools Extra maintainers listing](https://clang.llvm.org/) — LLVM, ongoing. Confirms McCall's long-term ownership of clangd within the LLVM governance model.
- [clangd.llvm.org](https://clangd.llvm.org/) — LLVM, ongoing. User-facing docs showing how a server communicates gradual degradation (index warming, config) without breaking the LSP contract.

## Study first
1. Preamble + background index: serving syntax-level answers before semantic results exist
2. Memory budgets and index sharding for large corpora
3. clangd's request-priority model vs SpecForge's debounce/dispatch split
