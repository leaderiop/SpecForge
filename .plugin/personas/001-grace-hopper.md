# Position — Grace Hopper (compiler pioneer; FLOW-MATIC/COBOL)

**Verdict:** KEEP_WASM
**Confidence:** 4

## Arguments
1. A compiler's plugin runtime should itself be a compilation target, not a bolted-on interpreter. Wasm makes every plugin a machine-independent compiled artifact — the founding 1952 move. Compiled blobs give reproducibility serving R-4 (sha256-pinned installs, `builtin_blob_sync` byte-compare, evidence §1.1) and R-6 determinism. Scripting interpreters drag ambient ecosystems: evidence §4 names Python's pip as ambient-capability — an R-2 violation by construction.
2. R-1 demands one mechanism, all plugins equal. Evidence §1.2 shows the real disease is *three parallel implementations* — native mirrors in `crates/specforge-emitter/src/builtins/`, vendored blobs, SDK crates — with custom rules still running natively via `NativeCustomRules` (`crates/specforge-emitter/src/compile.rs`). Swapping runtimes adds a fourth; converging mirrors and rule dispatch onto wasm cures C7-11.
3. The guest payload is ~97% declarative manifest (evidence §2). That is FLOW-MATIC's lesson: make the manifest an English-like declarative block the host compiles — `#[specforge_extension]` already generates it — reserving the runtime for the 3% real logic (`@specforge/formal`'s four passes, ~478 lines, evidence §1.1). Lua/TS optimize the 3% while forfeiting compile-time checking of the 97%.

## Biggest risk in my verdict
Staying ships open wasm findings (C7-02 fake AOT, C7-08 ledger engine pool, C7-10 unenforced timeout), and the Rust+wasm toolchain may gate the AI-agent authors the registry bet depends on.

## What would change my mind
Measured proof that agents won't author Rust→wasm, plus a scripting candidate passing the same sandbox conformance suite (R-2) and single-binary test (R-3) while *reducing* implementation count.
