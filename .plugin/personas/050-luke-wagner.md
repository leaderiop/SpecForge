# Position — Luke Wagner (Wasm/Component Model architect)

**Verdict:** KEEP_WASM
**Confidence:** 4

## Arguments
1. Sandboxing must be a property of the execution model, not a policy layer bolted on. Wasm gives memory isolation and capability-scoped imports structurally; evidence.md's own candidate table concedes CPython has "none by default" and QuickJS "none built-in." The C7-04 hole is one line — `crates/specforge-wasm/src/sandbox.rs:22` sets `file_system_access: Some(true)` — a default to flip, not an architecture to replace. Under R-1 (all plugins equal), every interpreter escape hatch exposes all plugins identically.
2. The versioning skeleton is already Component-Model-shaped: `crates/specforge-wasm/src/protocol/host.rs::validate_protocol_version` enforces same-major compatibility against PROTOCOL_VERSION 1.0.0. The real fix for C7-03 is a typed IDL (WIT-style) over `call_export`, not a runtime swap that resets five candidates' compatibility stories to zero and strands signed registry artifacts (R-4).
3. R-3/R-6 are met today: static-linked wasmtime needs no system packages, and fixed wasm semantics give deterministic snapshots across macOS arm64/Linux x64. PyO3 is "notoriously fragile" to embed-and-ship; LuaJIT vs 5.4 drift breaks snapshot determinism.

## Biggest risk in my verdict

KEEP_WASM inherits six open HIGH findings — C7-08's EnginePool ledger and C7-10's unenforced `max_execution_ms` (knob already at `sandbox.rs:8`). Unfixed, we pay wasmtime's full dependency weight for promises that never fire, and Rust-toolchain-gated authoring may starve the third-party registry bet entirely.

## What would change my mind

A Lua/TS embedding demonstrating memory-isolated, deterministic, zero-system-dep sandboxing would shrink wasm's uniqueness to toolchain maturity. And MULTI only after a typed IDL exists: two runtimes over today's stringly protocol doubles C7-03's drift surface.
