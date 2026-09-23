# 050 — Luke Wagner

**Cluster:** C7 — Wasm plugin runtimes & the extension bet
**Roster role:** Wasm co-designer; Component Model architect
**SpecForge anchors:** crates/specforge-wasm/src/protocol/mod.rs (PROTOCOL_VERSION "1.0.0"), crates/specforge-wasm/src/protocol/host.rs (validate_protocol_version, same-major compatibility), crates/specforge-wasm/src/sandbox.rs (instance isolation)

## Why this engineer
Wagner co-designed WebAssembly (after creating asm.js) and leads the Component Model: interface types, versioned compatibility, sandboxed instances communicating through typed interfaces. SpecForge's handshake — a semver check where same-major means compatible (protocol/host.rs) — is a hand-rolled miniature of the component versioning discipline he specified. His larger lesson matches SpecForge's architecture: keep the core small, stable, and vocabulary-free; move all growth into typed, versioned, independently evolvable extension units.

## References for SpecForge
**Key works**
- [WebAssembly Core Specification](https://www.w3.org/TR/wasm-core-1/) — W3C Recommendation, 2019. The deliberately minimal core he co-designed — the discipline SpecForge's zero-domain-knowledge core imitates.
- [WebAssembly Component Model](https://github.com/WebAssembly/component-model) — Wasm CG proposal, 2021. Design for instances, interfaces, and version compatibility — the reference for evolving handshake v1.0.0 without breaking extensions.
- 10 Years of Wasm: A Retrospective — Bytecode Alliance, 2026. His own retrospective on what aged well (core stability) and what changed (components).
- WebAssembly — Mozilla, 2015. The founding announcement post: the multi-vendor process that kept the core honest.

## Study first
1. Component Model versioning/compatibility rules vs the same-major check in host.rs
2. Why core wasm stayed minimal — map the argument onto SpecForge's registry/extension split
3. Instance isolation semantics (no shared state by default) vs EnginePool instance reuse
