# Position — Kevin Hoffman (wasmCloud co-creator; capability-based Wasm)
**Verdict:** KEEP_WASM
**Confidence:** 4
## Arguments
1. Structural enforcement, not interpreter convention. `crates/specforge-wasm/src/host_functions.rs` exposes exactly four imports (query, emit_diagnostic, resolve_ref, read_file) behind a per-CallSite permission matrix (`is_host_function_allowed`) — the guest's entire world, wasmCloud's capability-provider philosophy at process scale. No ambient fs/net/process exists to escape *from*; Lua/Python/QuickJS start with ambient capabilities, so their "sandbox" is a wrapper around interpreter escapes, one bug from defeat.
2. Hard requirements align: R-3 (wasmtime is static; evidence.md's own table marks PyO3 as system-Python-dependent — auto-fail), R-4 (blobs are sha256-pinned reproducible artifacts), R-6 (wasm32-unknown-unknown determinism).
3. The C7 cluster indicts implementation, not runtime. C7-04 (fs allow-by-default) and C7-10 (`max_execution_ms` unenforced) are enforcement bugs in `sandbox.rs`'s otherwise sound most-restrictive-wins merge; C7-11's three parallel implementations violate R-1 and converge under one runtime — not by adding one.
## Biggest risk in my verdict
The sandbox is partly cosmetic today: C7-08 warm-engine story is vaporware and C7-10 caps are unenforced. If that enforcement debt can't close, KEEP_WASM ships a trust story without enforcement — worse than an honest interpreter.
## What would change my mind
Proof that per-call compile over 1.7k-entity graphs is unfixable via pooling/AOT (making R-5/R-6 unusable), or that call-site checks can't bind at the import boundary — then Luau instruction-count hooks behind explicit capability wrappers earn a second look.
