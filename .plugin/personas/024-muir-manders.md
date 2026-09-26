# Position — Muir Manders (gopls core contributor; completion engine)

**Verdict:** KEEP_WASM
**Confidence:** 4

## Arguments
1. **Budget enforcement and determinism are runtime-native only in wasmtime.** C7-10 (`max_execution_ms` never enforced) is fatal for a host running plugins inline on analyze/LSP paths, compounding C14-03's blocking in `crates/specforge-lsp`. Wasmtime 43 ships fuel metering and epoch interruption: per-call budgets enforced *by the runtime*, deterministically, keeping snapshots (R-6) stable. In mlua, budgets mean DIY debug hooks; memory isolation across the C boundary doesn't exist — one bad plugin corrupts the host (R-2 fails).
2. **The latency problem is a host bug, not the runtime.** EnginePool is a ledger (C7-08) while per-entity rules fan out over a ~1.7k-entity graph. The in-model fix — warm pooled instances plus one batched payload per rule sweep over the existing `call_export` bridge in `crates/specforge-wasm/src/protocol/` — keeps per-call overhead sub-ms. The gopls move: cheap full sweeps against warm state, never re-instantiate mid-request.
3. **Migration cost dominates the alternatives.** Guest payload is ~97% generated manifest, 629 LOC total (evidence §2); switching runtimes rewrites `@specforge/formal`'s 478-line passes and all four manifests, while the real R-1 convergence — deleting the `crates/specforge-emitter/src/builtins/` mirrors (C7-11) — is runtime-independent.

## Biggest risk in my verdict
I'm betting the team funds the in-model fixes (pool, fuel, honest AOT). If not, KEEP_WASM keeps shipping vaporware at 511 locked deps, and a µs-warm Lua interpreter was the pragmatic call.

## What would change my mind
A measured warm-instance rule sweep over the 1.7k-entity fan-out still blowing an interactive budget (>50 ms per analyze) after pooling + batching; or a hard binary-size/cold-start constraint wasmtime provably cannot meet.
