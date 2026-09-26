# Position — Donald Knuth (literate programming; exactness & determinism)
**Verdict:** KEEP_WASM
**Confidence:** 4
## Arguments
1. A plugin pass is a pure function over the graph artifact; wasm enforces purity structurally — linear memory, no ambient fs/net/entropy absent a granted import (R-2). Lua/Python purity depends on which stdlib functions the host remembers to strip: a convention, and conventions drift. R-6 determinism by construction beats determinism by discipline.
2. The audit indicts SpecForge's prose, not the machine: C7-02's "AOT cache" is a byte-copy, C7-08's pool a ledger, C7-10's `max_execution_ms` never enforced — code claiming what it does not do, the cardinal literate sin. Make program and description coincide, as the `extension_json_sync` guard (crates/specforge-extism) already does; swapping interpreters recreates these gaps elsewhere.
3. The 97%-manifest/3%-logic split (evidence §2) is the essay/compile duality working: declarative categories read like documentation; `@specforge/formal`'s 478-line passes compile like code. Wasmtime links statically (R-3), blobs are sha256-pinned in the signed registry (R-4), modules re-instantiate for R-5 hot reload. Python fails R-3 outright; MULTI violates R-1's one-mechanism answer to C7-11.
## Biggest risk in my verdict
Ergonomics starves the ecosystem: authoring needs a Rust toolchain plus `wasm32-unknown-unknown`, and today's only authors are the project itself (evidence §1.5). The registry bet dies if AI agents won't write guests.
## What would change my mind
Proof that capability-stripped, instruction-counted Lua (Luau-style hooks) matches wasm's determinism and sandboxing at an order-of-magnitude smaller host — or that AI-agent guest authoring is demonstrably impractical, killing the ecosystem before it exists.
