# Position — John Ellson (Graphviz/DOT co-creator; release engineering)

**Verdict:** KEEP_WASM
**Confidence:** 4

## Arguments

1. The defect is the seam, not the runtime. Graphviz codegen plugins stayed compatible for decades because the boundary was an explicit model, never ad-hoc text. SpecForge's boundary is stringly JSON over `call_export`, no IDL (C7-03); drift already happened. Swapping runtimes re-creates that drift; fix the contract inside wasm first.

2. The workload is batch compute over a snapshot — exactly what capability-scoped wasm carries. Guests are ~97% manifest, ~3% logic (629 lib.rs lines; formal's 478 pure analysis); even the rendering vocabulary (`dot_shape`/`dot_color`/`dot_fillcolor`, `crates/specforge-emitter/src/builtins/`) is declarative data. Nothing needs ambient fs/network: R-2 deny-by-default is natural; C7-04 is a config bug, not a limit.

3. R-3/R-4/R-6 favor static wasmtime: single binary, no system packages (PyO3 fails outright; V8 bloats), signed sha256-verified blobs, deterministic snapshot-testable output. Open findings — C7-02's fake `.aot` cache, C7-08 (`crates/specforge-wasm/src/engine_pool.rs` is a ledger), C7-10's unenforced deadline — are debt fixable in place; a swap rebuilds the 9.4k-LOC runtime layer and fixes none.

## Biggest risk in my verdict
Rust+wasm32 toolchain friction starves the registry bet: if AI-agent authors won't compile guests, wasm's safety wins ring hollow. Wasmtime's dependency weight (485→511 locked deps) is a release tax.

## What would change my mind
Evidence authoring friction blocks third parties, or a workload needing ambient ecosystems — then protocol-first MULTI: one IDL, two sandboxed engines. Vendored-C Lua with CPU-instruction hooks beating wasm on R-2/R-3 at bounded migration cost would make LUA credible.
