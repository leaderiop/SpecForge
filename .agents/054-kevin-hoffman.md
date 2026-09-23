# 054 — Kevin Hoffman

**Cluster:** C7 — Wasm plugin runtimes & the extension bet
**Roster role:** wasmCloud co-creator — capability-based distributed Wasm
**SpecForge anchors:** crates/specforge-wasm/src/sandbox.rs (default SandboxPolicy, most-restrictive-wins merge), crates/specforge-wasm/src/host_functions.rs (query/emit_diagnostic/resolve_ref/read_file allowlist), SandboxPolicy field in ManifestV2

## Why this engineer
Hoffman's wasmCloud gives modules authority only through explicit capability providers, constrained by signed claims — nothing else exists from the module's point of view. SpecForge's extension sandbox is the same philosophy at process scale: network denied by default, memory/time caps, an output-extension allowlist, and exactly four imported host functions as the module's entire world. His book is the canonical introduction to precisely this stack (Rust → wasm → host functions → capabilities), written before wasmCloud existed and the origin of the project itself.

## References for SpecForge
**Key works**
- [wasmCloud](https://github.com/wasmCloud/wasmCloud) — GitHub, 2020. Capability-provider architecture and claims-based authority — the design SpecForge's sandbox/host-function boundary mirrors.
- [Programming WebAssembly with Rust](https://pragprog.com/titles/khrust/programming-webassembly-with-rust/) — Pragmatic Bookshelf, 2019. Host-function and capability chapters are the onboarding text for specforge-wasm's host_functions.rs.
- Reflections on Three Years of wasmCloud — wasmcloud.com, 2022. Origin essay: how the book's capability thinking became a platform.
- Beyond the Twelve-Factor App — O'Reilly, 2016. Attach dependencies, don't embed them — the cloud-native argument for SpecForge's extension-as-capability model.

## Study first
1. wasmCloud capability providers & claims vs SandboxPolicy layers in sandbox.rs
2. Book's host-function exercises → compare with the four SpecForge host imports
3. What a "lattice" would buy SpecForge if extensions ever run out-of-process
