# 013 — Tim Clem

**Cluster:** C3 — Parsing & grammar infrastructure
**Roster role:** tree-sitter at GitHub scale; code navigation & code-search engineering
**SpecForge anchors:** grammar runtime loading in crates/tree-sitter-specforge; wasm grammar feasibility study spec/research/RES-30-tree-sitter-wasm-feasibility.md

## Why this engineer
Brunsfeld built the parser; Clem made it deployable. An early GitHub employee (first ~20), he was a major tree-sitter contributor, shipped GitHub's first code navigation, then built Blackbird — a GitHub-scale code-search engine written from scratch in Rust — and now works on coding agents and agent context. SpecForge faces the same second act: today the grammar is compiled into the binary via build.rs, while RES-30 studies whether grammars/parsers can instead arrive at runtime through the wasm extension system. His operational experience — parsers as versioned artifacts, per-language distribution, symbol extraction at billions-of-files scale — is the playbook for keeping the zero-domain-knowledge core while vocabulary ships out-of-band.

## References for SpecForge
**Key works**
- [The technology behind GitHub's new code search](https://github.blog/engineering/architecture-optimization/the-technology-behind-githubs-new-code-search/) — GitHub Blog, 2023. Blackbird's architecture: Rust from scratch, sparse indexes, tree-sitter-based symbol extraction at planetary scale.
- [GitHub Code Search is generally available](https://github.blog/news-insights/product-news/github-code-search-is-generally-available/) — GitHub Blog, 2023. The product surface that architecture supports; a benchmark for specforge-mcp search tooling.
- [A small bio for the interested](https://adaptivepatchwork.com/about/) — personal site, ongoing. First-person record: GitHub v3 API, tree-sitter, first code navigation, Blackbird, coding agents.
- [tclem](https://github.com/tclem) — GitHub profile. "Engineer at GitHub since 2011"; repository trail of parsing and platform experiments.

## Study first
1. Blackbird's indexing pipeline: how parser output feeds a search index — model for aggregating specforge-test → specforge-report.json runs
2. RES-30 against his artifact-distribution experience: when runtime-loaded grammars are worth the versioning cost
3. Code navigation as product: one parse tree behind hover, references, and diagnostics — how specforge-lsp could unify its surfaces
