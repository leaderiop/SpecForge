# Position — Dan North (BDD inventor; JBehave creator)

**Verdict:** KEEP_WASM
**Confidence:** 4

## Arguments

1. The verify chain decides this. Every `behavior` in the .spec DSL carries machine-checkable `verify` obligations (see `behaviors.spec`); their whole value is re-runnable evidence, and R-6 makes that a hard requirement. Wasm execution is deterministic and memory-isolated; embedded interpreters import ambient ecosystems — Python's pip is ambient-capability per evidence.md §4 — which turns "verify" into "usually verify".
2. The workload is 97% declarative (evidence.md §2): guests are ~97% generated manifest, ~3% logic, and the declarative part is host-executed already. Switching runtimes optimizes the 3% (formal's 478 lines) while destabilizing the substrate carrying 100% of behaviors. The BDD move is vocabulary convergence, not engine swap: delete the `crates/specforge-emitter/src/builtins/` mirrors (C7-11) and fix C7-03 with a real IDL so the manifest has one name everywhere.
3. R-2 is structural in wasm, cosmetic elsewhere. The sandbox holes in today's path (C7-04 fs allow-by-default, C7-08 ledger-only engine pool, C7-10 unenforced timeout) are fixable inside the model: capability imports deny-by-default, enforced fuel. Lua's interpreter sandbox and deno's permission model are host-enforced promises; wasmtime isolation is the engine itself.

## Biggest risk in my verdict

The Rust-toolchain SDK (`crates/specforge-extension-sdk`) may starve the registry bet — third parties won't buy a `wasm32-unknown-unknown` toolchain to ship one validator.

## What would change my mind

Registry evidence that authorship stalls specifically on toolchain friction, while a Lua or deno_core candidate demonstrably meets R-2 deny-by-default, R-3, and byte-identical R-6 snapshots.
