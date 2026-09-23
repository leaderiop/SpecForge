# 028 — Eyal Kalderon

**Cluster:** C4 — Incremental compilation, LSP, diagnostics UX
**Roster role:** tower-lsp creator
**SpecForge anchors:** crates/specforge-lsp (tower-lsp as direct dependency, backend.rs)

## Why this engineer
Kalderon authored tower-lsp, the framework SpecForge's `crates/specforge-lsp` is built directly on. Its design — each LSP request as a Tower `Service`, typed `lsp_types`, half-duplex client/server JSON-RPC over stdio — is the substrate backend.rs runs on, and its concurrency model (interleaved requests, server→client notifications) constrains how SpecForge can publish diagnostics while IncrementalPipeline recomputes. Understanding the tool from its author's perspective is the fastest path to debugging lifecycle and backpressure issues in SpecForge's server.

## References for SpecForge
**Key works**
- [ebkalderon/tower-lsp](https://github.com/ebkalderon/tower-lsp) — GitHub, 2019. The crate SpecForge depends on; its examples are the canonical patterns backend.rs's LanguageServer impl extends.
- [tower-lsp on crates.io](https://crates.io/crates/tower-lsp) — crates.io, 2019. Version history and Tower-stack dependencies; relevant to upgrading against lsp_types changes.
- [tower-rs/tower](https://github.com/tower-rs/tower) — GitHub, 2017. The underlying Service abstraction; explains tower-lsp's request middleware/backpressure semantics.

## Study first
1. tower-lsp concurrency: per-request services vs shared server state (state.rs)
2. Server-to-client requests (workspace/config) and cancellation
3. Error handling and panics: what a handler crash does to the session
