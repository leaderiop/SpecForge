# 049 — Dan Gohman

**Cluster:** C7 — Wasm plugin runtimes & the extension bet
**Roster role:** WASI lead — system interfaces & capability-based sandboxing
**SpecForge anchors:** docs/extension-sdk.md (says wasm32-wasi) vs crates/specforge-extism/src/builtins.rs (embeds wasm32-unknown-unknown builds) — the doc drift; crates/specforge-wasm/src/sandbox.rs (deny-by-default SandboxPolicy)

## Why this engineer
Gohman co-invented Wasm, wrote the LLVM wasm backend, and created WASI and wasi-libc — he is the authority on what "running outside the browser" should mean. His core teaching: wasm security is capability-based, there is no ambient authority, imports must be granted explicitly. That is exactly SpecForge's sandbox.rs (network denied by default, most-restrictive-wins merge, output-extension allowlist) — and exactly the lens for resolving SpecForge's known drift: docs say extensions build for wasm32-wasi while the four builtins are wasm32-unknown-unknown, two targets with different import surfaces and therefore different implicit authority.

## References for SpecForge
**Key works**
- [WebAssembly/WASI](https://github.com/WebAssembly/WASI) — GitHub, 2019. The subgroup documents he founded: what a portable system interface for wasm must and must not grant.
- [WASI capabilities in Wasmtime (docs/WASI-capabilities.md)](https://github.com/bytecodealliance/wasmtime/blob/main/docs/WASI-capabilities.md) — Wasmtime docs, 2021. Canonical capability enumeration (preopens, no network by default) — the model SandboxPolicy already imitates.
- [What is a Wasm component?](https://blog.sunfishcode.online/what-is-a-wasm-component/) — sunfishcode's blog, 2022. A component as a closed, capability-complete unit — frames extensions as grants, not trusted code.
- WASI: Wasm's system interface — Wasm I/O 2025 keynote. Where WASI preview 2 / 0.2 is heading and what it means for extension targets.

## Study first
1. Capability-based security in WASI: preopens, opt-in clocks/network — map onto SandboxPolicy layers
2. wasm32-unknown-unknown vs wasm32-wasi import surfaces → fix the docs/impl drift in builtins.rs
3. First-Class I/O and component essays: what WASI 0.2 changes for host-function design
