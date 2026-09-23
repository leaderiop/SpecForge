# 118 — Alice Ryhl

**Cluster:** C14 — Direct-dependency maintainers (surface infrastructure)
**Roster role:** Tokio core maintainer; async Rust performance & teaching
**SpecForge anchors:** crates/specforge-lsp/src/backend.rs, crates/specforge-mcp (stdio JSON-RPC server), crates/specforge-watch/src/debounce.rs

## Why this engineer
Ryhl is a core Tokio maintainer who perf-drives and reviews the runtime SpecForge's LSP, MCP, and watch crates run on, and her ryhl.io essays are the reference for the one mistake that degrades such servers: blocking the reactor. Her guidance (spawn_blocking for fs-heavy passes, cancellation safety in select! loops, runtime sizing) maps one-to-one onto specforge-watch's debounce/dispatch loop and specforge-mcp's stdio request handling.

## References for SpecForge
**Key works**
- [tokio-rs/tokio](https://github.com/tokio-rs/tokio) — GitHub, 2016. The runtime under specforge-lsp/mcp/watch; her maintenance and optimization work shapes its behavior.
- [Async: What is blocking?](https://ryhl.io/blog/async-what-is-blocking/) — ryhl.io, 2020. The definitive explanation of cooperative scheduling; directly governs how fs/registry work must be spawned off the reactor.
- Rust for Linux live with Alice Ryhl and Greg Kroah-Hartman — corrode.dev, 2026. Runtime and Rust-infrastructure constraints at the kernel boundary.

## Study first
1. What counts as blocking: timers, mutexes, fs — and spawn_blocking discipline
2. Cancellation safety of select! branches (watch dispatch under file-event storms)
3. Runtime configuration knobs relevant to editor-hosted servers (LSP)
