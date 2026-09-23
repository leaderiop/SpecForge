# 117 — Carl Lerche

**Cluster:** C14 — Direct-dependency maintainers (surface infrastructure)
**Roster role:** Tokio creator; async runtime architecture
**SpecForge anchors:** crates/specforge-lsp/src/backend.rs, crates/specforge-mcp (stdio JSON-RPC server), crates/specforge-watch/src/debounce.rs (runtime `time` features)

## Why this engineer
Lerche created Tokio (released August 2016) and its defining reactor/executor split; the workspace pins tokio 1.50 with `macros`, `rt-multi-thread`, `io-std`, `time` — the exact configuration his design targets. SpecForge's three long-lived surfaces (LSP backend, MCP stdio loop, watch debounce timers) are runtime-hosted services of the kind his mini-redis tutorial teaches to structure: task-per-connection, `select!`, bounded channels, graceful shutdown.

## References for SpecForge
**Key works**
- [tokio-rs/tokio](https://github.com/tokio-rs/tokio) — GitHub, 2016. The async runtime underneath specforge-lsp/mcp/watch; scheduler and reactor design docs live here.
- [tokio-rs/mini-redis](https://github.com/tokio-rs/mini-redis) — GitHub, 2020. The canonical teaching codebase for idiomatic Tokio services — closest pattern source for watch's event/debounce loop and MCP's stdio handling.
- Tokio (software) — Wikipedia. Verified origin record: developed by Carl Lerche, released August 2016.
- Async Runtime for Rust with Carl Lerche — Heavybit, High Leverage podcast, 2026. Runtime design rationale straight from the creator.
- The Evolution of Async Rust: From Tokio to High-Level — JetBrains livestream, 2026. Where runtimes are heading; relevant to LSP/MCP process-lifecycle choices.

## Study first
1. Scheduler/reactor separation and task budgeting in the multi-threaded runtime
2. mini-redis patterns: task-per-connection, select!, graceful shutdown
3. Runtime lifecycle for long-lived servers (LSP/MCP) vs short CLI commands
