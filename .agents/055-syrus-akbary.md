# 055 — Syrus Akbary

**Cluster:** C7 — Wasm plugin runtimes & the extension bet
**Roster role:** Wasmer founder — competitive runtime landscape
**SpecForge anchors:** crates/specforge-wasm/src/runtime.rs (WasmRuntime trait — the swap point), crates/specforge-extism (wasmtime via Extism), docs claiming "Wasm only runtime" (positioning language to keep accurate)

## Why this engineer
Akbary founded Wasmer (YC 2019), the main independent alternative to wasmtime: pluggable Singlepass/Cranelift/LLVM backends, the WASIX superset of WASI (threads, fork, sockets), and Wasmer Edge for deployment. For SpecForge he matters as the competitive counterfactual: knowing what the other runtime ecosystem offers keeps the WasmRuntime/Extism coupling honest — the abstraction must stay real — and WASIX shows where "WASI isn't enough" pressure goes, which is exactly where SpecForge's sandbox and host-function surface would feel strain.

## References for SpecForge
**Key works**
- [wasmerio/wasmer](https://github.com/wasmerio/wasmer) — GitHub, 2018. Multi-backend runtime and packaging stack; the concrete alternative embedding target if the Extism/wasmtime path ever fails a requirement.
- WASIX — Wasmer, 2023. A superset of WASI adding threads/fork/sockets — the pressure test for what SpecForge's deny-by-default sandbox must keep denying.
- Wasmer Edge (GA) — wasmer.io, 2023. Edge deployment of Wasm apps — where a future hosted extension registry would live.
- Wasm I/O 2024 — conference talk. State of the runtime ecosystem from the competitor's side.

## Study first
1. Backend pluggability (Singlepass vs Cranelift vs LLVM) — cost/startup tradeoffs for a CLI
2. WASIX vs WASI 0.2 divergence — which extensions SpecForge's protocol could never express
3. Wasmer's registry/packaging model vs the extension specifier + lock file
