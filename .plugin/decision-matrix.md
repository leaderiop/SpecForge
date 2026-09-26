# Decision Matrix — Plugin Runtime Options × Criteria

Options: **KEEP_WASM** (Extism/Wasmtime, current) · **LUA** (mlua, Lua 5.4) ·
**PYTHON** (CPython via PyO3) · **TYPESCRIPT** (quickjs-ng/deno_core) ·
**MULTI** (several runtimes behind one protocol).

Ratings: ●●●● strong · ●●●○ adequate · ●●○○ weak · ●○○○ poor.

| Criterion | KEEP_WASM | LUA | PYTHON | TYPESCRIPT | MULTI |
| --- | --- | --- | --- | --- | --- |
| **C1 Safety/sandbox** (R-2) | ●●●● memory isolation; imports = capabilities; fuel/epoch limits (wasmtime-native) | ●●○○ no ambient fs/net by default, but shared address space; CPU hooks LuaJIT-only | ●○○○ ambient-capable stdlib, GIL, C-extension escape hatches | ●●●○ deno permissions strong; quickjs/boa = convention only | ●○○○ weakest: N sandboxes to audit |
| **C2 Authoring (AI + human)** (R-1 UX) | ●●○○ Rust toolchain + wasm32 target + SDK macros; AI writes it but builds fail on hallucinated APIs | ●●●● tiny API, single file, LLMs fluent in Lua | ●●●● largest LLM corpus; but packaging/venv footguns | ●●●● largest typed corpus; .d.ts doubles as the missing IDL | ●○○○ N authoring surfaces to document/test |
| **C3 Performance** | ●●●○ ~40–120 ms compile per cold plugin (pooled/AOT fixable — C7-02/C7-08); near-native exec | ●●●● sub-ms start; ~2–10× slower compute than native | ●○○○ slowest start; GIL serializes passes | ●●●○ quickjs ~10–50× slower compute; V8 near-native after warmup | ●○○○ per-runtime overhead stacks |
| **C4 Distribution** (R-3) | ●●●● static wasmtime in the binary (already shipped) | ●●●● vendored C, static link | ●○○○ system python (fails R-3) or +30–50 MB bundled | ●●●○ quickjs ~1–3 MB; deno_core/V8 tens of MB | ●○○○ sum of all runtime weights |
| **C5 Embedding maturity in Rust** | ●●●● wasmtime/extism production-grade (already integrated) | ●●●● mlua mature, async-capable | ●●●○ PyO3 mature; embedding-for-distribution is the fragile part | ●●○○ deno_core mature; quickjs-rs younger | ●●○○ glue of N engines |
| **C6 Host API evolution** (C7-03) | ●●●○ WIT/component model is the standards-based fix (migration cost real) | ●●●● plain tables/callbacks; describe-by-convention | ●●○○ objects + pickle temptations; contract anemia | ●●●● shipped `.d.ts` doubles as the IDL | ●○○○ N API dialects |
| **C7 Hot reload** (R-5) | ●●●○ re-instantiate ~10–30 ms per changed blob (authoring rebuild is the real latency) | ●●●● re-read script; instant | ●●●○ import reload fragile | ●●●○ context re-creation | ●○○○ |
| **C8 Debuggability** | ●●○○ wasm stack traces are hostile | ●●●● line-numbered errors, print | ●●●● tracebacks, but env-dependent | ●●●● stack + source maps (engine-dependent) | ●○○○ |
| **C9 Migration cost** | ●●●● zero (current state) | ●●○○ rewrite formal's 4 passes (~212 LOC logic) + SDK surface | ●○○○ full rewrite + runtime packaging | ●●○○ rewrite passes + new TS SDK; registry artifact-type change | ●●●● zero (superset) |
| **C10 Binary size / deps** | ●○○○ wasmtime/cranelift family dominates (already paid) | ●●●● ~1 MB | ●○○○ +30–50 MB bundled or system dep | ●●●○ quickjs ~1–3 MB; V8 tens of MB | ●○○○ |

## Hard requirements check

| Requirement | KEEP_WASM | LUA | PYTHON | TYPESCRIPT | MULTI |
| --- | --- | --- | --- | --- | --- |
| **R-1** all plugins equal, no first-party tier | ✅ (after deleting the native mirrors) | ✅ (after migrating the 4 builtins) | ✅ (after migrating) | ✅ (after migrating) | ✅ by definition — but N mechanisms ≈ N tiers |
| **R-2** sandbox untrusted | ✅ | ⚠️ interpreter convention, shared process | ❌ | ⚠️ strong only under deno_core/V8 | ⚠️ strongest per-sub-runtime |
| **R-3** single binary | ✅ (already) | ✅ | ❌/⚠️ | ✅ quickjs / ⚠️ V8 | ⚠️ sum |
| **R-4** signed registry | ✅ (blobs hash-pinned) | ⚠️ scripts sign, but they are plain text | ⚠️ | ⚠️ | ⚠️ |
| **R-5** hot reload | ⚠️ (re-instantiate; authoring rebuild dominates) | ✅ | ⚠️ | ✅ | ⚠️ |
| **R-6** deterministic analyze | ✅ (fuel + import allowlist; deterministic-profile wasmtime) | ⚠️ (GC, no fuel) | ❌ (hash randomization, env) | ⚠️ (GC, JIT) | ❌ |

## Reading of the evidence

- The four builtins' wasm payloads are **~97% generated manifest, ~3% real logic** — and
  the only logic-bearing guest is `@specforge/formal` (4 compiler passes, ~212 LOC of
  pass logic + 168 LOC of tests).
- Custom validation rules already dispatch **natively in the host**
  (`NativeCustomRules`) — the wasm `validate__*` path is bypassed for builtins today.
- The audit's C7 findings (AOT byte-copy, ledger engine pool, stringly callExport,
  sandbox allow-by-default, unused fuel) are **host bugs**, not wasm-model bugs — every
  one has a fix inside KEEP_WASM.
- The single strongest argument for a scripting language is **C7-11 consolidation**:
  one authored artifact = one executed = one distributed artifact (D12's LUA verdict).
  The strongest counters: R-2 sandboxing by construction and R-6 determinism (D05, D09),
  and R-3 for Python.

## Verdicts of the 12 dimension deep-dives

| Dimension | Verdict | Conf |
| --- | --- | --- |
| D01 KEEP_WASM steel-man | KEEP_WASM | — |
| D02 LUA embedding | (see file) | — |
| D03 PYTHON assessment | KEEP_WASM | 4 |
| D04 TYPESCRIPT assessment | TYPESCRIPT | 3 |
| D05 sandbox/security | KEEP_WASM | 4 |
| D06 performance/distribution | KEEP_WASM | 4 |
| D07 AI-agent authoring | KEEP_WASM | 4 |
| D08 host API | KEEP_WASM | 4 |
| D09 determinism/formal | KEEP_WASM | 4 |
| D10 migration cost | KEEP_WASM | 3 |
| D11 hot reload | (see file) | — |
| D12 consolidation | LUA | 4 |
