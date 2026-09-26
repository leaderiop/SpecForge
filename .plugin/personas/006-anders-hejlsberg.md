# Position — Anders Hejlsberg (DSL & language design; LSP-first tooling)

**Verdict:** KEEP_WASM
**Confidence:** 4

## Arguments
1. The real defect is the contract, not the engine. C7-03: stringly-typed JSON over `runtime.call_export`, no IDL, drift already shipped. My playbook — Turbo Pascal, C#, TypeScript — is one declared vocabulary served to compiler and editor alike through a language service; `crates/specforge-lsp/src/backend.rs` already exists. Author a protocol IDL, generate host+guest types from it: C7-03 dies in any engine, and "what a plugin is" becomes portable intelligence instead of ad-hoc JSON.
2. The measured workload makes the runtime choice cheap: guests are ~97% generated manifest, ~3% logic (evidence.md §2; only `@specforge/formal`'s four `#[compiler_pass]` functions, 478 lines, carry analysis). Push the 97% fully into declarative data via the `crates/specforge-extension-sdk` macros; wasm already satisfies R-2 (memory isolation + capability imports), R-3 (static single binary), R-4 (hashable vendored blobs), R-6 (determinism).
3. Swapping runtimes re-buys solved work — ~9.4k LOC host (`crates/specforge-wasm/`), SDK on crates.io, vendored blobs — while keeping C7-11's three-parallel-implementations debt. Converge first: delete the `crates/specforge-emitter/src/builtins/` mirrors, actually enforce C7-10's `max_execution_ms`, make C7-02's AOT cache real.

## Biggest risk in my verdict
Authoring ergonomics: a Rust toolchain plus `wasm32-unknown-unknown` target is precisely the friction that stalls the registry's bet on third-party — mostly AI-agent — plugin authors.

## What would change my mind
Evidence that third-party authoring stalls on toolchain weight even after a typed IDL ships — then add TYPESCRIPT as a second authoring surface behind the same protocol (MULTI-lite), with embedding cost (V8/quickjs under R-3) kept off the host's critical path. A runtime swap that doesn't first fix C7-03 just repeats it in a new engine.

## Verdict

**Verdict:** KEEP_WASM
**Confidence:** 4
**One-line rationale:** Fix the undeclared contract (IDL, generated types, one converged implementation) inside the existing wasm runtime; a ~3% logic surface doesn't justify re-embedding an interpreter.
