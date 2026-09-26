# Position — Andrej Karpathy (LLM-context & agent-economics)

**Verdict:** LUA
**Confidence:** 3

## Arguments
1. The authoring surface is the product. Primary consumers are AI agents (evidence.md §1.5), yet the SDK demands a Rust toolchain + wasm32 target — every agentic edit→verify iteration pays a cross-compile tax. Guests are ~97% manifest, ~3% logic; agents author that shape best in a scripting language, and Lua's tiny surface is token-cheap — RES-18's arithmetic applies to the plugin loop too.
2. Workload physics. All four guests total 629 lines of lib.rs (formal: 478); the runtime carries a feather while the host carries ~9.4k LOC plus wasmtime's dependency weight (485→511 locked deps). Vendored Lua 5.4 via mlua carries the same payload, satisfies R-3, and makes R-5 hot reload a file re-read.
3. R-1 convergence. One interpreter runs builtin manifests and third-party plugins identically, letting the native mirrors in `crates/specforge-emitter/src/builtins/` and the blob-sync guard tests die — closing C7-11's three-implementation drift at the root, not with another test.

## Biggest risk in my verdict
mlua's sandbox is interpreter-level, not memory isolation; R-2 rests on denying io/os plus CPU budgets — a historically escapable surface. Porting formal's 4 passes risks transient R-6 snapshot drift.

## What would change my mind
Evidence that plugins need wasm-grade isolation or compute beyond an interpreter over 1.7k-entity graphs, or that mlua can't be made deny-by-default with instruction limits — then KEEP_WASM with C7-04/C7-10 fixed.
