# 048 — Alex Crichton

**Cluster:** C7 — Wasm plugin runtimes & the extension bet
**Roster role:** wit-bindgen & Component Model tooling; early Rust/Cargo core
**SpecForge anchors:** crates/specforge-wasm/src/protocol/types.rs (HandshakeRequest/Response structs), docs/extension-protocol.md (JSON-over-memory protocol), component-model migration path for protocol v1.0.0

## Why this engineer
Crichton co-built Rust and Cargo, then spent years on wasmtime/wasi and authored wit-bindgen, the reference WIT → bindings generator for the Component Model. SpecForge's extension protocol is hand-rolled: JSON structs exchanged over linear memory, versioned by a semver string in the handshake. Crichton's work is the standards-grade replacement — typed interfaces (WIT), generated bindings, component-level versioning — that would let the protocol evolve without breaking every extension at once.

## References for SpecForge
**Key works**
- [wit-bindgen](https://github.com/alexcrichton/wit-bindgen) — GitHub, 2022. Generator from WIT interfaces to Rust/C/… bindings — the successor to hand-maintained describe descriptors.
- [WebAssembly Component Model](https://github.com/WebAssembly/component-model) — Wasm CG proposal, 2021. Typed, composable, versioned components; how `__handshake`/`__describe` could become a WIT world with host functions as imports.
- [Cargo](https://github.com/rust-lang/cargo) — rust-lang/cargo, 2012. His package-manager design (lockfiles, registries, semver care) is the direct ancestor of specforge-wasm/lock_file.rs and the extension specifier grammar.
- WebAssembly and Rust in Practice — netstack.fm podcast, 2026. Current-state interview on wasm beyond the browser.

## Study first
1. WIT worlds: model the host API (query/emit_diagnostic/resolve_ref/read_file) as a world
2. wit-bindgen codegen vs the 11 describe categories — what a componentized protocol deletes
3. Component versioning semantics vs the same-major-compat rule in protocol/host.rs
