# Position — Paul Gauthier (aider creator; token-budgeted repo maps)

**Verdict:** KEEP_WASM

**Confidence:** 4

## Arguments

1. The workload barely needs a runtime: evidence §2 shows the guest payload is ~97% generated manifest, ~3% logic (629 lines of lib.rs total; only `@specforge/formal` carries real analysis). Declarative JSON, not scripting — swapping interpreters optimizes the 3%, not the 97%.

2. R-6 determinism is my home turf: `crates/specforge-emitter/src/budget.rs` (`estimate_tokens`, `emit_json_with_budget`) fits exports to `--max-tokens` in pure, snapshot-tested Rust — and the budget applies only to schemaless JSON (`crates/specforge-cli/src/export.rs:127`). Wasm execution is deterministic by construction; CPython's GIL and ambient pip, V8 GC timing, and Lua stdlib drift across interpreter versions all jeopardize snapshot-stable analyze output.

3. R-2/R-3/R-4 mechanically hold only on wasm: memory isolation plus capability imports, static single binary, sha256-verified signed blobs (registry e2e verified live per evidence §1.4). Fix the audit gaps inside the model — C7-04 allow-by-default sandbox, C7-10 unenforced 30s cap, C7-11's three parallel implementations — rather than swapping runtimes.

## Biggest risk in my verdict

The registry bet dies on authoring ergonomics: C7-00 showed even this repo fumbled the wasm toolchain (fresh-clone build failure). If agent authors can't reliably emit `wasm32-unknown-unknown` guests, third-party plugins never materialize — KEEP_WASM optimizes for the four builtins that exist, not the ecosystem R-4 assumes.

## What would change my mind

Measured evidence that agent-authored wasm guests fail cold builds at meaningful rates, plus a quickjs/deno_core tier demonstrably meeting R-3, R-4, and R-6 — then TYPESCRIPT with a capability-scoped host API wins.
