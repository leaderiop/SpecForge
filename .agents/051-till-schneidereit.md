# 051 — Till Schneidereit

**Cluster:** C7 — Wasm plugin runtimes & the extension bet
**Roster role:** Bytecode Alliance co-founder; runtime ecosystem positioning
**SpecForge anchors:** crates/specforge-extism (embeds wasmtime via Extism) vs crates/specforge-wasm/src/runtime.rs (native WasmRuntime abstraction), docs on "Wasm only runtime" positioning claim

## Why this engineer
Schneidereit shepherded Wasm from browser experiment to industry platform: SpiderMonkey's Wasm lead, then co-author of the Bytecode Alliance founding announcement and part of the team that carried wasmtime-class engineering out of Mozilla. The BA thesis — vendors should share secure, well-scrutinized runtime foundations instead of each forking their own — is precisely SpecForge's positioning decision: embed a maintained runtime (wasmtime, via Extism) behind the thin WasmRuntime trait, keep a small native fallback, and never claim to be a runtime vendor. He is the reference voice for what to build vs what to adopt.

## References for SpecForge
**Key works**
- [Announcing the Bytecode Alliance](https://hacks.mozilla.org/2019/11/announcing-the-bytecode-alliance/) — Mozilla Hacks (with Lin Clark), 2019. The manifesto: secure-by-default shared foundations; the strategic frame for SpecForge's embed-don't-fork bet.
- [Bytecode Alliance](https://bytecodealliance.org/) — industry partnership site, 2019+. Governance and shared-roadmap model for wasmtime/Cranelift/WASI — the ecosystem SpecForge's sandbox TCB depends on.
- WebAssembly: A New Hope — Strange Loop, 2015. The talk that framed wasm for the broader developer ecosystem; useful for positioning writing.
- WebAssembly beyond the browser — talks and BA writings, 2019–2023. The ecosystem map: wasmtime vs Wasmer vs V8 vs language-native runtimes, and why a CLI host should embed rather than compete.

## Study first
1. BA security/nondeterminism agenda — what it promises a host that embeds wasmtime
2. Ecosystem map: which runtimes are production-viable behind a CLI (and why wasmtime won here)
3. Governance: what it means for SpecForge that its sandbox TCB is BA-maintained, not in-repo
