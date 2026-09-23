# 023 — Robert Findley

**Cluster:** C4 — Incremental compilation, LSP, diagnostics UX
**Roster role:** gopls co-lead at Google; Go tooling & telemetry
**SpecForge anchors:** LSP workspace indexing (crates/specforge-lsp/src/state.rs, symbols.rs), CLI collect telemetry

## Why this engineer
Findley co-leads gopls, carrying it through multi-module workspace support — the hard part of indexing where inputs are scattered across directories and boundaries blur, exactly SpecForge's situation with imports spanning a corpus plus Wasm-extension registries. He also designed Go's transparent telemetry and wrote "Telemetry in Go 1.23 and beyond": a privacy-respecting, opt-in feedback loop that SpecForge's CLI collect / specforge-report.json protocol should imitate.

## References for SpecForge
**Key works**
- [golang/tools (gopls)](https://github.com/golang/tools) — GitHub, 2019. His workspace.md design doc and multi-root handling are the deepest published treatment of workspace indexing edge cases.
- Telemetry in Go 1.23 and beyond — The Go Blog, 2024. Transparent, local-first collection with explicit upload consent — the ethical + practical template for SpecForge's test-tracing collection.
- [gopls documentation](https://pkg.go.dev/golang.org/x/tools/gopls) — Go team, ongoing. Documents settings/diagnostic surfacing decisions a server must make explicit to stay usable across editors.

## Study first
1. gopls workspace design: modular workspaces, overlay files, invalid workspace states
2. Transparent telemetry design: local counters, stack-sample reports, opt-in upload
3. How gopls sequences expensive re-indexing without blocking hover/completion
