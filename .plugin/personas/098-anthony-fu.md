# Position — Anthony Fu (Vitest creator; JS/TS tooling)

**Verdict:** TYPESCRIPT
**Confidence:** 3

## Arguments
1. **The authoring audience is JS-shaped.** Evidence §1.5: primary consumers are AI coding agents over MCP, and my anchor `@specforge/vitest` (vitest Reporter → `specforge-report.json` → `specforge collect`) lives in the TS ecosystem. A `collect__*` collector mapping Vitest test names to spec entity IDs is natural in the language of the reports it parses; adapter and plugin runtime should share one surface.
2. **Structural deny-by-default.** C7-04 (sandbox fs allow-by-default) stays open because Extism exposes ambient imports. QuickJS/deno_core inverts the model: the host injects the entire API surface — no fs/net/process exists unless added as capability-scoped host functions. R-2 becomes architecture, not configuration.
3. **R-3/R-5 fit.** QuickJS vendors as C into a single binary (strip types with oxc/esbuild before eval); text plugins re-parse instantly in `specforge watch`, while the wasm-side hot/warm infrastructure (C7-02 AOT byte-copy, C7-08 ledger EnginePool) keeps slipping.

## Biggest risk in my verdict
`@specforge/formal`'s four `#[compiler_pass]` functions (~478 lines, the only logic-bearing guest, evidence §1.1) port graph analysis into a GC'd engine — latency risk on 1.7k-entity graphs, and R-6 determinism requires the host to freeze Date/random rather than wasm's import-free purity.

## What would change my mind
A benchmark showing QuickJS passes over the 1.7k-entity graph materially exceed the wasm path, or a credible plan fixing C7-04/C7-08/C7-10 inside Extism within one release.
