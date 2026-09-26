# Position — Nathan Sobo (Wasm extension runtimes in editors)

**Verdict:** KEEP_WASM
**Confidence:** 4

## Arguments

1. **The capability model is already built and is the right one.** `crates/specforge-wasm/src/host_functions.rs` implements exactly what Zed extensions do — a permission matrix (`is_host_function_allowed`) crossing call sites with host functions, path-confined `host_read_file_check`, Provider-only domain-allowlisted `host_http_get_check`, typed graph-mutation checks. This is memory isolation plus capability imports; no interpreter alternative (Lua, QuickJS) ships an equivalent — you'd rewrite enforcement from scratch and trust it less. R-2 is a config fix (C7-04), not an architecture change.
2. **R-1 kills the alternatives.** The audit's real sin is C7-11: three parallel implementations. Python fails R-2/R-3 outright (ambient pip capabilities, system interpreter); MULTI re-entrenches parallelism as policy. KEEP_WASM is the only option whose migration is *deletion* — drop the native mirrors in `crates/specforge-emitter/src/builtins/` and route `NativeCustomRules` through guest `validate__*`, converging on one mechanism.
3. **The workload fits.** Guests are 629 LOC total, ~97% generated manifest (evidence.md §2); the wasm payload is small, snapshot-deterministic (R-6), hot-reloadable by re-instantiation (R-5), and statically linked (R-3). Registry blobs are signed and sha256-pinned (R-4). The Zed lesson stands: Atom died of an unsandboxed core scripting tier; don't rebuild it.

## Biggest risk in my verdict

Third-party authoring ergonomics: requiring a Rust toolchain + `wasm32-unknown-unknown` target may starve the registry ecosystem the product is betting on.

## What would change my mind

Hard evidence the C7 gaps are unfixable in-host (e.g., per-call instantiation overhead surviving a real EnginePool/AOT fix and dominating analyze latency), or a product mandate that non-Rust guest languages are first-class requirements.
