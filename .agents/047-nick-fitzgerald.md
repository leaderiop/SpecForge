# 047 — Nick Fitzgerald

**Cluster:** C7 — Wasm plugin runtimes & the extension bet
**Roster role:** Wasmtime/Cranelift engineering; wasm-bindgen creator
**SpecForge anchors:** crates/specforge-wasm/src/cache.rs (engine/module caching), crates/specforge-wasm/src/engine_pool.rs (warm instances), CLI cold-start budget in specforge-cli

## Why this engineer
Fitzgerald led the rust-wasm tooling generation (wasm-bindgen, wasm-pack) and now engineers Wasmtime and Cranelift, with a public focus on correctness at speed. SpecForge is a CLI: every invocation pays engine setup plus module compilation unless artifacts are cached. His domain — AOT compilation, serializable modules, and Wasmtime's on-disk CLI cache — is the exact playbook for making `specforge` extension calls cheap on fresh invocations and for justifying the warm-pool design in EnginePool for the long-running MCP/watch surfaces.

## References for SpecForge
**Key works**
- [wasmtime](https://github.com/bytecodealliance/wasmtime) — GitHub, Bytecode Alliance, 2019. The engine under Extism; AOT compilation and serialized modules are first-class.
- [CLI cache configuration (docs/cli-cache.md)](https://github.com/bytecodealliance/wasmtime/blob/main/docs/cli-cache.md) — Wasmtime docs, 2020. Concrete design for persisting compiled modules across process invocations — the AOT-caching strategy to copy into specforge-wasm/cache.rs.
- [JavaScript to Rust and Back Again: A wasm-bindgen Tale](https://hacks.mozilla.org/2018/04/javascript-to-rust-and-back-again-a-wasm-bindgen-tale/) — Mozilla Hacks, 2018. Lessons from designing a host↔guest bridge — applicable to host-function signature design.
- Correctness in Wasmtime and Cranelift — WasmCon 2023 talk, fitzgeraldnick.com, 2023. Fuzzing/differential discipline for a runtime SpecForge trusts in its TCB.

## Study first
1. Wasmtime cache config + module serialization → cold-start strategy for the CLI
2. Cranelift correctness discipline (fuzzing, spec-level testing) for the sandbox TCB
3. wasm-bindgen bridging decisions vs SpecForge's hand-rolled JSON boundary
