# 045 — Steve Manuel

**Cluster:** C7 — Wasm plugin runtimes & the extension bet
**Roster role:** Extism creator — plugin-system ergonomics
**SpecForge anchors:** crates/specforge-extism/src/runtime.rs (PluginBuilder host), crates/specforge-wasm/src/protocol/ (__handshake/__describe host), docs/extension-protocol.md

## Why this engineer
Manuel designed Extism: one universal plugin runtime (built on wasmtime) with thin host SDKs and PDKs in many languages, so any app gains a secure plugin system without adopting someone's framework. SpecForge made exactly this bet — embedding Extism in crates/specforge-extism to load vocabulary contributions from .wasm instead of hand-rolling a loader. His discipline (data in/out over linear memory, host functions as the only authority) is the direct precedent for the `__handshake`/`__describe` protocol and the four imported host functions (query, emit_diagnostic, resolve_ref, read_file).

## References for SpecForge
**Key works**
- [Extism — the WebAssembly framework](https://github.com/extism/extism) — GitHub, extism/extism, 2021. The runtime SpecForge embeds; study plugin lifecycle and manifest handling against specforge-extism/src/runtime.rs.
- [Extism: make all software extensible](https://extism.org) — Dylibso, 2022. The founding design statement: wasm as a neutral plugin ABI, not a runtime vendors must adopt wholesale.
- [Extism Rust host SDK (extism crate)](https://docs.rs/extism) — docs.rs, 2022. The API surface SpecForge actually calls (PluginBuilder, host functions, traps).
- Episode #58: Steve Manuel & Ben Eckel on Extism — devtools.fm, 2023. Design history: why a memory ABI + PDKs before WIT/component tooling matured.

## Study first
1. Extism host contract: manifest → PluginBuilder → call_export — mirror of runtime.rs
2. PDK/memory-ABI marshalling vs SpecForge's JSON payloads over shared memory
3. The "squishy software" thesis: which surfaces to keep pluggable (compare the 11 describe categories)
