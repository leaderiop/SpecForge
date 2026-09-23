# 046 — Benjamin Eckel

**Cluster:** C7 — Wasm plugin runtimes & the extension bet
**Roster role:** Extism co-author — PDKs, manifests & typed data conversion
**SpecForge anchors:** crates/specforge-wasm/src/manifest_bridge.rs (ManifestV2 validation), crates/specforge-extism/tests/ (PDK-built fixtures), docs/extension-sdk.md (SDK macros → protocol categories)

## Why this engineer
Eckel (Dylibso co-founder, later Figma) built Extism's developer-facing half: extism-convert (typed marshalling over the plugin memory ABI), the manifest format, and PDKs across languages — plus Chicory (pure-Java wasm runtime) and XTP Bindgen (schema → language bindings). That is precisely the layer where SpecForge extension authors live: `manifestVersion: 2`, wasmPath, sandbox policy, and SDK macros that expand into describe-category descriptors. His conversion-crate pattern explains how SpecForge's serde-JSON handshake payloads sit on top of raw bytes in shared memory.

## References for SpecForge
**Key works**
- [extism/extism](https://github.com/extism/extism) — GitHub, 2021. Host + PDK monorepo; manifest format and convert crate live here — the upstream for the ABI SpecForge embeds.
- [extism-convert](https://docs.rs/extism-convert) — docs.rs, 2022. ToBytes/FromBytes traits for typed exchange over plugin memory — the pattern behind handshake/describe payload marshalling.
- XTP Bindgen (preview) — Extism/Dylibso blog, 2024. Schema-driven bindings generation — a live precedent for generating describe-descriptor code instead of hand-writing it.
- Manifest format v2 (extism docs) — Extism docs, 2022. Compare field-by-field with ManifestV2 in specforge-wasm/manifest_bridge.rs.

## Study first
1. extism-convert traits → what SpecForge gains/loses with bare serde_json over bytes
2. Extism manifest vs ManifestV2: wasmPath, hash pinning, peer_dependencies
3. XTP Bindgen's schema→bindings flow as future SDK codegen for the 11 categories
