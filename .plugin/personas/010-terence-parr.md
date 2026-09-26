# Position — Terence Parr (DSL & language design)

**Verdict:** KEEP_WASM
**Confidence:** 4

## Arguments

1. RES-30 already proved the right pattern at small scale (`spec/research/RES-30-tree-sitter-wasm-feasibility.md`): embedding a runtime inside each plugin is impractical; the host provides capability, guests ship small portable artifacts (15 KB grammar `.wasm` + `.scm` queries). Embedding Lua/Python/JS inverts that — every plugin ships a language, the host shrinks to a syscall surface. Wasm keeps one interpreter in the host; guests stay inert artifacts.
2. The plugin language is already declarative. Evidence: guests are ~97% generated manifest, ~3% logic; all real logic is one 478-line crate (`extensions/formal`). Don't adopt a general-purpose language to carry a manifest protocol — fix the protocol itself (C7-03, no IDL, drift already happened). That's a language-definition defect, orthogonal to runtime choice.
3. Failure semantics: a wasm trap aborts one call, not the host — exactly the rule-scoped error recovery I demand of a parser. Lua/CPython give no memory isolation (R-2) and weak preemption; wasmtime epoch/fuel actually enforces the `max_execution_ms` C7-10 left unenforced, and import-bounded execution yields determinism (R-6) by construction.
4. C7-11's three parallel implementations converge by deleting the `crates/specforge-emitter/src/builtins/` mirrors and running the vendored blobs — a cleanup inside KEEP_WASM, not a migration.

## Biggest risk in my verdict

Authoring ergonomics: a Rust + wasm32 toolchain for a five-line rule deters the AI-agent and human authors the registry bets on; if logic-bearing plugins dominate, compile loops become the bottleneck and scripting pressure wins.

## What would change my mind

Evidence that per-plugin logic is intrinsically large and string-heavy (scripting genuinely better), that graph snapshots can't marshal efficiently across the wasm boundary, or that C7-10-style timeout enforcement proves infeasible in wasmtime.

## Verdict

**Verdict:** KEEP_WASM
**Confidence:** 4
**One-line rationale:** Host-provided capability + portable guest artifacts is the pattern RES-30 already validated; fix the plugin IDL (C7-03), not the runtime.
