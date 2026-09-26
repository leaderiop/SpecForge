# Position — Evan Czaplicki (compiler-error UX / compiler-as-teacher)

**Verdict:** KEEP_WASM
**Confidence:** 3

## Arguments
1. **Authoring-time errors are the feedback loop.** Plugin authors are AI agents (evidence.md §1.5); the Rust→wasm toolchain makes rustc the first teacher — typed errors with hints before anything runs. Lua/Python defer every mistake to runtime, mid-analyze, as tracebacks an agent must reverse-engineer. Elm's bar is an error you can act on without the manual; rustc already clears it.
2. **Determinism is a diagnostics requirement.** R-6 demands snapshot-testable analyze output; the repo's own `extension_json_sync` byte-compare test sets that bar. Wasm execution is deterministic; CPython (GIL, version drift) and LuaJIT create a new failure genre: same spec, different output.
3. **The diagnostic channel is already structured.** `Diagnostic { span, suggestion }` renders via `crates/specforge-emitter/src/diagnostic_fmt.rs` as `file:line:col` plus a `help:` line — the Elm shape. What's broken is the wire (C7-03 stringly JSON, drift already happened) and the sandbox gate (C7-04 fs allow-by-default); both fixable inside KEEP_WASM. Switching runtimes buys diagnostics nothing.

## Biggest risk in my verdict
The edit→reload loop: a wasm plugin change needs rustc + `wasm32` compile before `specforge watch` re-runs analyze (R-5). If that cycle takes tens of seconds, agents iterate blind — a slow feedback loop is a diagnostics failure by my own standard.

## What would change my mind
Measurement showing watch-loop edit→diagnostic latency over a few seconds for typical plugin edits, or that Rust-toolchain friction stops third-party authors — then a scripting tier behind one typed protocol (MULTI), never Python.
