# Audit Triage — verified against current tree

Source: `.reports/` full audit (generated 2026-09-27 08:20, rev `789d214`, 26 commits behind HEAD at generation time).
195 findings; 35 already marked FIXED in the report. The remaining 160 were re-verified here.
The audit predates the W3–W5 component cutover (`1f9632e`), so findings referencing the extism
runtime, the native tier, `HostContext`, `engine_pool`, or `crates/specforge-extism` are stale.

Legend: STALE = construct already deleted; OPEN = verified still present; PARTIAL = core fixed, residue remains.

## Critical

- C? critical finding: already marked FIXED in the report. None open.

## HIGH — verified status

### STALE (resolved by Phase 7 native-tier deletion + W3–W5 cutover; no action)

| fid | finding | why stale |
|---|---|---|
| C7-06 | "Wasm-only claim false three ways: native BuiltinRuntime…" | native tier deleted; claim now true |
| C7-09 | query_scope ignored in host_context.rs make_query_graph_fn | HostContext/extism host fns deleted entirely |
| C7-11 | three parallel extension implementations | two of three deleted; single component path |
| C7-08 | warm-engine vaporware (engine_pool.rs ledger) | engine_pool.rs deleted; ComponentRuntime shares one wasmtime Engine across plugins — warm-engine claim now real |

### PARTIAL (core fixed; doc/design residue — folded into fixes below)

| fid | finding | residue |
|---|---|---|
| C1-01 | "Wasm (Extism) is the only runtime" contradicted by native path | README:130 still claims Extism + native Rust builtins + composite runtime + auto-bootstrap → rewrite (see F1) |
| C7-02 | AOT caching is a byte-copy; _aot_cache_path ignored; README claims AOT | fake copy logic gone with extism; README "cached by content hash" claim remains wrong (F1); `WasmRuntime::load_module` still threads an unused `aot_cache_path` param (F2 cleanup) |
| C10-00 | process entities uninterpreted (evidence: emitter/builtins/formal.rs) | that file deleted; logic now lives in extensions/formal guest — semantics unchanged, so the finding stands but evidence path is stale |

### OPEN — Wave 1 (S effort, verified)

| fid | finding | plan |
|---|---|---|
| ~~C14-00~~ | VERIFIED ALREADY FIXED — search_query uses form_urlencoded::Serializer; tests pin reserved-char encoding (`search_query_encodes_reserved_characters`) | none |
| C10-04 | diagnostic code collision: E030–E034 mean different things in spec vs code | FIXED: docs dictionary aligned to produced reality (E030–E034 = extension-host errors; phantom formal rows retired; real formal codes are E041/E046/E047); migrate moved off triple-booked E015 onto E019; explain.rs phantom E015/E016 entries corrected |
| C11-02 | coverage matcher: exact free-text verify matching; orphaned test records silently ignored | MOVED TO WAVE 2 — audit evidence paths no longer resolve; test-tracing subsystem needs remapping first |
| ~~C12-00~~ | VERIFIED ALREADY FIXED — specforge.spec declares all four extensions | none |
| C12-04 | authoring flow instructs providers that don't exist | FIXED (see C2-11); extension-model.md provider examples corrected to `extension` field |
| C12-08 | CBCL: `extension` (code) vs `package` (docs) | FIXED: entities/spec.md, quick-reference, spec-writing-flow, extension-model all use `extensions` field spelling; provider configs use `extension` field; Generators section marked planned |
| C2-08 | quick-reference.md teaches removed ID system | FIXED: `{infix}-{n}` patterns → author-chosen identifiers; `plugins` → `extensions`; hard counts de-numbered |
| C2-11 | spec-writing-flow.md onboards onto nonexistent syntax | FIXED: infix/checkpoints removed; extensions spelling; phantom provider examples (@specforge/gh/jira/figma) replaced with the four builtins |
| C2-07 | E013 documented, unimplemented; E014/E015 test mismatch | decide: implement reserved-words check or fix docs+tests |

### OPEN — Wave 2 (M/L effort, engineering)

| fid | finding | area |
|---|---|---|
| C4-02 | LSP positions treated as byte offsets (UTF-16 world) | specforge-lsp document.rs |
| C7-04 | sandbox deny-by-default is allow-by-default | specforge-wasm/sandbox.rs |
| C9-13 | MCP advertised tool schemas drift from implementations | specforge-mcp |
| C11-00 | `specforge collect` is a stub (detects, prints, reads nothing) | specforge-cli collect.rs |
| C11-01 | `specforge coverage --min` gate does not exist | specforge-cli |
| C13-00 | dot_shape/color/fillcolor declared, never consumed | specforge-emitter dot.rs |
| C6-02 | version negotiation hardcoded 1.0.0 | specforge-emitter export.rs/schema.rs |
| C6-06 | Context/Brief exports emit a different node shape | specforge-emitter schema.rs |
| C6-13 | config schema and ProjectConfig disagree both directions | schema/project.rs |
| C8-01 | registry publish non-atomic/racy | specforge-registry-server |
| C5-00 | four divergent cycle detectors (Phase D fixed detect_cycles semantics; consolidation open) | graph/lsp/cli |
| C5-01 | LSP never surfaces cycle diagnostics | specforge-lsp |
| C3-06 | link_references flat/scope-blind; file_scopes computed then ignored | parser/resolver |
| C4-00 | two parallel invalidation implementations | lsp backend vs cli pipeline |
| C14-03 | LSP blocking walkdir/fs/parse in async handlers | specforge-lsp backend.rs |
| C14-04 | registry-server blocking sqlite/hash/multipart on async runtime | specforge-registry-server |
| C1-06 | flagship example has zero test results feeding traceability | examples/docs |
| C1-10 | RES-18 token budget claim without budget mechanism | emitter context export |
| C10-00 | process entities uninterpreted by formal pass | extensions/formal |
| C13-08 | docs promise extension-contributed render formats | emitter model + docs |
| C8-09 | integrity pinned to same-channel SHA-256 (TOFU) | design decision, not code |

## Docs sweep (second pass, `fix(docs)`)

Verified and fixed beyond Wave 1 — stale-docs findings resolved against the current tree:
- C1-002: vision/README + business/08-investment-thesis — nonexistent extensions (@specforge/compliance, atomic-design, api-design, data-pipeline, business-model) marked planned; shipped builtins called out
- C2-008b: entity-model.md count contradiction (23 vs 24) — fixed to 22 domain kinds
- C2-010: EBNF identifier rule corrected to match grammar (`[a-zA-Z_][a-zA-Z0-9_]*`, 1+ unbounded)
- C2-010 INFO: formatter error-recovery documented in spec-troubleshooting.md
- C5-033: cardinality.rs doc comment replaced with the real contract; resolved thinking-out-loud block deleted
- C12-099b: governance decision `wasm_component_runtime` records the wasip2-component build assumption (replaces the wasi/unknown-unknown era)
- C12-104b: spec root has no providers block anymore — stale, no action
- C12-105: root declares @specforge/formal — glossary leakage premise resolved
- C12-106: both dateless decisions now carry dates (2026-03-08, from git history)
- C12-103: "All 9 entity kinds / All 16 edge types" prose counts de-numbered in features.spec (real: 22 kinds / 57 edges)
- C12-109: bare `verify "..."` form documented (grammar: kind optional, empty default)
- C13-114: gen example in extension-model.md marked planned
- C14-117: unused tokio dropped from specforge-registry
- C3-016: interner contract documented (leak-by-design, single writer, empty sentinel)
- C9-069: verified already present (diagnostic summary prints the explain hint)
- Post-cutover staleness found proactively: extension-sdk.md status banner + wasm32-wasip2 targets; extending-specforge.md targets + component_guest! wiring example

## Execution order

Wave 1 (this pass): the nine S-effort OPEN highs above + README:130 rewrite (C1-01/C7-02 residue).
Wave 2: C4-02 UTF-16 positions, C7-04 sandbox default, C11-00/C11-01, C9-13, C13-00, then C6-x, C8-01.
Wave 3: consolidation items (C5-00, C4-00, C3-06) and product-level (C1-06, C1-10, C10-00, C13-08).

Status updates land in this file; the .reports/ data.js is a generated artifact and is not hand-edited.
