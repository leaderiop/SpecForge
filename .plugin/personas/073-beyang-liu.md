# Position — Beyang Liu (context-platform engineer; ex-Sourcegraph CTO, Amp co-founder)

**Verdict:** TYPESCRIPT
**Confidence:** 3

## Arguments
1. The author base decides this: evidence.md §1.5 — plugin authors are AI agents; the product thesis is agent-authored specs. Agents iterate fastest in TypeScript; wasm forces a Rust toolchain, wasm32-unknown-unknown target, proc-macro exports (`crates/specforge-extension-sdk-macros/src/lib.rs`) agents cannot step through. That ceremony is why the builtins are ~97% generated manifest, ~3% logic (evidence §2): the runtime tax ate the plugin.
2. The workload is a context API, not compute. Plugins get a graph snapshot, return diagnostics — the same shape as `crates/specforge-mcp/src/tools/find_definition.rs`. For graph-in/diagnostics-out, embedded QuickJS with deny-by-default host bindings is capability-scoped by construction (R-2), cold-starts instantly (C7-08's warm pool is vaporware), hot-reloads trivially (R-5).
3. KEEP_WASM preserves isolation but freezes C7-03 (no IDL) and C7-11 (three parallel implementations) into the substrate. Migration forces the typed IDL — the durable asset, per RES-19: context quality is the moat, not the execution vehicle. MULTI recreates C7-11 by design; PYTHON fails R-3; LUA viable but weaker for typed payloads.

## Biggest risk in my verdict
QuickJS lacks wasm's memory isolation and built-in permissions — the sandbox is only as good as our binding discipline; `@specforge/formal`'s 478 lines of Rust pass logic must be ported correctly and R-6 determinism re-proven.

## What would change my mind
If formal-style passes grow into CPU-heavy graph algorithms where wasm speed and isolation become load-bearing, or a first-class IDL hides wasm authoring from agents, KEEP_WASM-with-fixes wins on safety margin.
