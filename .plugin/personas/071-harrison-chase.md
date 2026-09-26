# Position — Harrison Chase (LangChain co-founder; agent graph orchestration)

**Verdict:** TYPESCRIPT
**Confidence:** 3

## Arguments

1. **Optimize the authoring loop — the author is an AI agent.** Evidence §1.5: consumers are AI agents, the registry bets on third-party plugins, yet authoring demands a Rust + wasm32 toolchain, and every plugin edit in `specforge watch` (R-5) pays a full cargo compile. Agents write TypeScript fluently, Rust poorly. My ambient agents subscribe via `crates/specforge-mcp/src/subscriptions.rs` to analyze deltas — that loop needs edit→re-analyze in seconds. TS re-parses; wasm re-links.

2. **The wasm machine outruns its payload and misses its promises.** Guests total 629 lib.rs lines, ~97% generated manifest JSON (evidence §2), behind 9.4k LOC of host runtime and wasmtime's dep weight — while C7-04 (fs allow-by-default), C7-10 (`max_execution_ms` unenforced), C7-08 (EnginePool is a ledger) show isolation/timeout advantages unshipped. `deno_core` ships capability permissions and static single-binary embedding (R-2, R-3) as baseline.

3. **Types become the missing IDL.** C7-03: stringly `call_export` JSON already drifted. TS types shared between host protocol and guests make the 9-category manifest contract machine-checkable in any authoring agent's tooling — leverage nothing else offers.

## Biggest risk in my verdict

Ergonomics conviction outruns embedding expertise: deno_core/V8 weight and determinism traps (`Date`, randomness) threaten R-6 and binary size; the four ~478-line `@specforge/formal` passes must rewrite cleanly.

## What would change my mind

Wasmtime landing deny-by-default sandbox, enforced timeouts, real warm engines — plus a one-command agent authoring loop — flips me to KEEP_WASM; so does snapshot divergence in the TS passes.
