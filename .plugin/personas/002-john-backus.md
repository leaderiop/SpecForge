# Position — John Backus (FORTRAN lead; BNF; declarative-intent advocate)

**Verdict:** KEEP_WASM
**Confidence:** 3

## Arguments

1. BNF discipline (ALGOL 60): a tiny exact contract lets independent tools agree. The plugin boundary already is one — `__handshake`, `__describe`, `__pass_<name>`, `validate__*` over JSON via `crates/specforge-extension-sdk`. Wasm keeps that contract language-neutral: any language compiles to it. Lua/Python/TS couple the contract to one language's semantics and version skew forever.

2. Declarative intent (1978 Turing lecture): evidence.md measures the guest payload at ~97% generated manifest, ~3% real logic — declarative data in a code costume. The fix is not a new interpreter but promoting the describe-JSON manifest to first-class data-only plugins; wasm remains the machine for the 3% (`@specforge/formal`'s four passes, ~478 lines, are stateless pure functions over a graph snapshot).

3. FORTRAN economics: the translation cost lives in the harness, not the concept — wasmtime 43.0.2, 9.4k LOC in `crates/specforge-wasm`. C7-02 AOT byte-copy, C7-08 engine-pool ledger, C7-10 unenforced fuel, C7-04 fs allow-by-default are harness defects fixable in place. R-3 kills PyO3 (system Python); V8 is heavy; only mlua competes on weight but forks the single contract R-1 demands — C7-11's three parallel implementations (native mirrors in `crates/specforge-emitter/src/builtins/`) converge only under one runtime.

## Biggest risk in my verdict

The harness's promises are vapor today: isolation and determinism claimed but unenforced (C7-04, C7-10). Fixing them — real AOT, enforced limits, deny-by-default, an IDL (C7-03) — is months inside a heavy dependency; stalled, KEEP_WASM is a 511-dep tax wrapping JSON in code.

## What would change my mind

Measurement, not taste: cold-start/per-call cost on 1.7k-entity graphs that pooling and real AOT cannot amortize, or third-party authoring staying Rust-only despite a data-only manifest tier — then mlua with instruction-count hooks is the honest small machine.
