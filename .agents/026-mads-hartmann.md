# 026 — Mads Hartmann

**Cluster:** C4 — Incremental compilation, LSP, diagnostics UX
**Roster role:** bash-language-server creator; LSP for config-like DSLs
**SpecForge anchors:** LSP for config-like DSLs (crates/specforge-lsp), tree-sitter grammar (crates/tree-sitter-specforge)

## Why this engineer
Hartmann created bash-language-server (2016), proving a small team can deliver a genuinely useful LSP experience for a "glue" language by leaning on a tree-sitter grammar plus curated metadata instead of deep semantic analysis. That is SpecForge's exact situation: `.spec` files are config-like, parsed by `crates/tree-sitter-specforge`, with intelligence coming from Wasm-extension registries rather than a heavyweight type system. His integration of external linters (shellcheck) and doc sources (explainshell) maps one-to-one onto SpecForge's validator diagnostics and registry hovers.

## References for SpecForge
**Key works**
- [bash-lsp/bash-language-server](https://github.com/bash-lsp/bash-language-server) — GitHub, 2016. Tree-sitter-driven completions, hovers, and external-linter diagnostics for a scripting language — the closest architectural sibling to SpecForge's LSP.
- [bash-language-server on npm](https://www.npmjs.com/package/bash-language-server) — npm, 2016–. Shows packaging/distribution of a DSL server via npm install — a comparison point for SpecForge's CLI-bundled server.
- [Bash IDE (VS Code extension)](https://marketplace.visualstudio.com/items?itemName=mads-hartmann.bash-ide-vscode) — VS Code Marketplace, 2017. End-to-end editor integration layer, analogous to integrations/vscode.

## Study first
1. Serving completions from a syntax tree + static word lists before any resolver exists
2. shellcheck-style integration: forwarding external diagnostics through publishDiagnostics
3. explainshell-style contextual hovers vs SpecForge registry describe
