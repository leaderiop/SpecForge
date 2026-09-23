# 027 — Nathan Sobo

**Cluster:** C4 — Incremental compilation, LSP, diagnostics UX
**Roster role:** Atom creator; Zed co-founder; Wasm extension runtimes in editors
**SpecForge anchors:** extension runtime design (crates/specforge-wasm handshake/describe, specforge-extism), integrations/vscode

## Why this engineer
Sobo built Atom, whose package ecosystem taught the industry that editors are platforms, then co-founded Zed to rebuild that idea with performance discipline — including a WebAssembly-based extension runtime (Zed extensions execute in wasmtime) after seeing native-plugin costs. SpecForge makes the same bet one level down: a zero-domain-knowledge core whose entire vocabulary comes from Wasm extensions over the handshake/describe protocol. Sobo's arc — extension model first in JS, then sandboxed Wasm — is the direct precedent for `crates/specforge-wasm` and for shipping the VS Code integration.

## References for SpecForge
**Key works**
- [We Have to Start Over: From Atom to Zed](https://zed.dev/blog/zed-decoded-from-atom-to-zed) — Zed Blog, 2024. The founders' retrospective on what Atom's architecture cost and why Zed chose Rust + Wasm extensions.
- [zed-industries/zed](https://github.com/zed-industries/zed) — GitHub, 2021. Production editor whose extension system is Wasm sandboxes: the closest shipped comparison for SpecForge's host/guest handshake.
- Zed extensions are WebAssembly — Zed Blog, 2023. Design rationale for Wasm guest extensions: isolation, language freedom, hot-reload — mirroring specforge-extism's builtin blobs.
- [Nathan Sobo interview — AI Engineer](https://ai.engineer/) — 2024–2025. Sobo on editors as AI-era platforms, relevant to SpecForge's agent-facing surfaces.

## Study first
1. Zed's Wasm extension API: host functions ↔ guest exports, vs SpecForge handshake/describe v1.0.0
2. Atom's package architecture: what overextending a core costs
3. Distribution of builtin vs third-party extensions
