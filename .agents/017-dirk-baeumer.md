# 017 — Dirk Bäumer

**Cluster:** C4 — Incremental compilation, LSP, diagnostics UX
**Roster role:** Microsoft LSP spec lead
**SpecForge anchors:** crates/specforge-lsp/src/backend.rs (server_capabilities), capabilities.rs, integrations/vscode

## Why this engineer
Bäumer (with Erich Gamma) created the Language Server Protocol at Microsoft, the exact decoupling SpecForge relies on: one tower-lsp server in `crates/specforge-lsp` serves VS Code, Neovim and any LSP client over the `.spec` grammar instead of N editor plugins. He also co-authored LSIF, the offline-indexing format — the pattern for precomputing symbol/registry data so `backend.rs` capabilities stay fast on the 219-file corpus.

## References for SpecForge
**Key works**
- [Language Server Protocol specification (3.17)](https://microsoft.github.io/language-server-protocol/) — Microsoft, ongoing. The normative contract SpecForge's server_capabilities, hover, and code_action handlers must honor; the reference for capability negotiation in `capabilities.rs`.
- [microsoft/vscode-languageserver-node](https://github.com/microsoft/vscode-languageserver-node) — GitHub, 2016. Reference client/server implementation; the cleanest example of capability gating and JSON-RPC plumbing to compare against backend.rs.
- Language Server Index Format (LSIF) specification 0.4.0 — Microsoft, 2019. Offline precomputed-index model relevant to SpecForge's registry-backed symbols/navigation.

## Study first
1. LSP 3.17 spec: capability negotiation and lifecycle (initialize → didChange → publishDiagnostics)
2. vscode-languageserver-node: how the reference server structures handlers vs backend.rs
3. LSIF: precomputing cross-file navigation data
