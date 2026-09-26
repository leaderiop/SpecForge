# Position — James Munns (wire-format author; embedded Rust & frugal serialization)

**Verdict:** KEEP_WASM
**Confidence:** 4

## Arguments
1. A plugin runtime is a wire-format decision, and this repo proves what happens without schema discipline: C7-03's stringly-typed `call_export` JSON already drifted pre-v1. The fix is the postcard pattern SpecForge applied to its binary report — pinned external contract (`schema/specforge-binary-report.schema.json`) — at the guest ABI. Switching interpreters trades that boundary for host-API couplings that churn per Lua/CPython/V8 release; R-4 reproducibility then means pinning the interpreter forever.
2. Only wasm satisfies R-2 and R-3 simultaneously (evidence §4): memory isolation plus capability imports, zero system deps. C7-04 (fs allow-by-default) is a flag flip in `crates/specforge-wasm/src/sandbox.rs`, not an architecture; CPython offers no wall, and mlua's sandbox is convention, not isolation.
3. The extism-convert path already moves typed payloads via postcard (varint, no_std): byte-stable, cheap decode over the 1.7k-entity graph snapshot each pass receives; R-6 snapshot tests become trivial. Guests are ~97% static manifest — make manifests first-class serialized artifacts, shrinking the dynamic surface, not an interpreter hauling JSON.

## Biggest risk in my verdict
KEEP_WASM inherits open audit debt — byte-copy AOT (C7-02), ledger-only EnginePool (C7-08), unenforced `max_execution_ms` (C7-10) — plus wasmtime 43's dependency weight. Without the IDL, sync-guard tests are the only contract.

## What would change my mind
Measured proof that decode/conversion overhead dominates pass latency, or that the Rust+wasm32 SDK blocks AI-agent authoring so badly the signed registry stays empty. Any MULTI tier must still clear R-2+R-3 — wasm or vendored QuickJS only.
