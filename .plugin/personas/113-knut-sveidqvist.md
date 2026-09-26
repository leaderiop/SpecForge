# Position — Knut Sveidqvist (Mermaid creator)

**Verdict:** LUA
**Confidence:** 3

## Arguments

1. Text-first distribution is my ethos, and it cuts for Lua: a plugin is one plain `.lua` file: diffable, agent-co-authorable, signable under R-4, hot-reloaded by re-reading bytes (R-5). No toolchain, no build step. Wasm inverts this: evidence.md shows guests are ~97% generated manifest, ~3% real logic — 9.4k host LOC and a wasmtime-dominated dep tree to ship mostly static JSON.
2. `render_mermaid` (`crates/specforge-emitter/src/model/mermaid.rs`) is pure host-side Rust emitting deterministic text: why mainstream renderers accept it, and exactly R-6. Lua 5.4 with `io`/`os` stripped gives the same: pure functions over the graph snapshot, deny-by-default by construction — fixing C7-04, not patching it, and making `max_execution_ms` real via mlua instruction hooks (C7-10).
3. C7-11 demands one mechanism: Lua collapses manifest and logic plugins into a single runtime; MULTI re-multilateralizes the failure. PYTHON fails R-3 and R-2 outright; TYPESCRIPT re-imports V8's weight for Rust-fluent authors.

## Biggest risk in my verdict

Interpreter sandboxing is capability hygiene, not memory isolation — an mlua misconfiguration (FFI, bytecode) reopens R-2. If passes over ~1.7k-entity graphs outgrow Lua's analyze budget, I've traded measured wasm speed for elegance. C7-03's missing IDL needs fixing under any verdict.

## What would change my mind

Profiling showing formal's four passes can't meet analyze budgets in Lua, or proof third-party plugins will be logic-heavy Rust — then KEEP_WASM with audit gaps closed wins on isolation and speed. A deno-grade permission sandbox at acceptable binary size flips me to TYPESCRIPT.
