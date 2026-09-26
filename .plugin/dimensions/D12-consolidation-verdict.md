# D12 — C7-11 Consolidation Synthesis

## What the duplication actually is

C7-11 is usually described as "three parallel implementations." Inspecting the tree, it is worse and more precise than that: the extension concept exists as **three copies plus two guard tests plus one build pipeline**, and the copies do not even agree on how many builtins exist.

- **Copy 1 — native mirrors.** `crates/specforge-emitter/src/builtins/{product,software,governance,formal}.rs` (plus two orphans, `rust.rs` 398 lines and `typescript.rs` 686 lines, registered in `builtins/mod.rs:18-25` but absent from `extensions/`). ~3,600 lines building `EntityKindDescriptor`/`EdgeTypeDescriptor`/`ValidationRuleDescriptor` values programmatically.
- **Copy 2 — committed JSON.** Nine `describe_*.json` files per extension, extracted from the mirrors by `xtask extract-extension-json` and committed under `extensions/<name>/src/` (documented in `crates/specforge-emitter/tests/extension_json_sync.rs:1-17`).
- **Copy 3 — blob bytes.** The guests `include_bytes!` the JSON (`extensions/software/src/lib.rs:10-18`) and serve it via `raw_category`; the host embeds the vendored blobs via `include_bytes!` (`crates/specforge-extism/src/builtins.rs:3-13`).
- **Guards.** `extension_json_sync` byte-compares mirror-regenerated payloads against the committed JSON; `builtin_blob_sync` asserts each blob embeds current payloads.

The proof that guards are not ownership: `SoftwareExtension`'s `Behavior` field list literally repeats itself — `software.rs:28-40` vs `41-53` declare the same descriptors twice (`contract` ×2, 27 entries where 14 were intended). I verified the committed `describe_entities.json` ships all 27, and therefore the vendored blob serves the duplication to CLI/LSP/MCP today. Both sync guards are green. When agreement is enforced but ownership lives nowhere, all three copies converge on the bug, faithfully.

Meanwhile the *logic* side of the house never converged: ~97% of guest payload is generated manifest (evidence.md §2), the only wasm-executed logic that fires is `formal`'s four passes, and custom validation rules execute **natively** via `NativeCustomRules` (`crates/specforge-emitter/src/compile.rs:464-486`), bypassing the guest `validate__*` exports entirely. So wasm today is: a manifest delivery pipeline, one real logic guest, and one native bypass.

## Why the duplication exists

The runtime cannot consume the authored artifact. The authored artifact is a Rust crate; the consumable artifact is a blob. Between them grew: extract (xtask), commit (JSON), embed (`include_bytes!`), vendor (`.wasm`), guard (two sync tests). Each layer exists to bridge author-form to run-form, and each bridge is a copy. Declaring one copy canonical does not fix this — the `extension_json_sync` header already calls the mirror "the source of truth," and the pipeline reproduced a copy-paste bug into two artifacts anyway. You fix it by making the runtime accept the author-form directly.

## Candidates against C7-11 specifically

**KEEP_WASM** *can* collapse copies 1–2: guests could build descriptors programmatically with SDK types (the mirrors are just Rust constructors; the `fd`/`edge` helpers move into the SDK). But the author-artifact split survives: the thing you edit (a crate) is still not the thing the host runs (a blob). Every plugin edit needs a `wasm32-unknown-unknown` cross-compile before `watch` can re-run analyze (R-5) — for a product whose primary author is an AI agent, the edit loop owns a toolchain. The 1.4 MB of vendored blobs stay, wasmtime's dependency weight stays, and C7-02/C7-08/C7-10 must be *repaired* rather than deleted. Consolidation grade: the pipeline shrinks; it does not die.

**PYTHON** consolidates the same way on paper, but R-3 fails honestly: system Python or a bundled distribution, notoriously fragile embedding, and a package ecosystem whose ambient capabilities fight R-2. Out.

**MULTI** is definitionally ≥2 mechanisms; R-1 demands one mechanism or a justification for each. Out for this dimension.

**TYPESCRIPT** collapses C7-11 only to re-open it: `deno_core` executes TS directly but drags V8 into the static binary (heavy, snapshot-coupled to tokio), while QuickJS needs a transpile step — source form and executed form diverge again, and the transpiled-byte cache is C7-11 regrowing in miniature.

**LUA** has a property none of the others have: a `.lua` file is simultaneously the authored manifest, the executed program, the distributed artifact, and the signed bytes. Zero derived artifacts. That is the maximum consolidation any candidate can achieve.

## End-state architecture — the one mechanism

**Plugin = one sandboxed script.** Lua 5.4 via vendored `mlua` (no system packages; R-3). A small host crate, `specforge-plugin`, is the entire runtime: verify registry signature → instantiate an interpreter with `io`/`os`/`package`/`require` removed and capability modules injected (graph read, spans, registries, diagnostics emit) → call `describe`/`__pass_<name>`/`validate__*` hooks. Sandboxing is the interpreter surface (R-2); memory via an allocator cap; CPU via an instruction-count hook — which makes C7-10 (`max_execution_ms` never enforced) real enforcement instead of a config field.

**Builtins ride the same loader.** `extensions/<name>/plugin.lua` is the single source. The host embeds its text (`include_str!`) and the registry publishes the identical bytes; one CI assertion that embedded == published replaces both sync tests — and it guards *distribution equality*, not content duplication, because content exists once. No first-party tier: the builtins get the same sandbox, the same capability grants, the same registry path as any third-party plugin (R-1, literally).

**Deleted:**

- all six native mirrors (~3,600 lines) — including the `rust`/`typescript` orphans, which must become plugins like everything else;
- the committed `describe_*.json` payloads and vendored blobs (1.4 MB; C7-00's vendoring closes by having nothing to vendor);
- the four guest crates and `specforge-extension-sdk`/`-macros` (`-protocol-types` survives as the one host-owned schema);
- `xtask extract-extension-json`, `extension_json_sync`, `builtin_blob_sync`;
- `NativeCustomRules` — with one call path, `validate__*` either fires or errors loudly; C6-11's silent-miss class cannot quietly recur;
- the engine-pool/AOT-cache/blob-integrity/sandbox-fs-allowlist machinery in `specforge-wasm` (~most of its 9.4k LOC) and wasmtime+extism from the lockfile. C7-02, C7-06, C7-08, C7-11 close **by deletion, not repair**.

**Survives:** pass toposort (the host must order by `after`-constraints — `extensions/formal/src/lib.rs:33-46` proves the host already must), the registry (it signs text; reproducibility becomes trivial because the artifact is text; R-4), and watch hot reload as file-reload (R-5: edit → re-analyze with no compile step).

**Costs, honestly:** `formal`'s ~478 lines of pass logic port to Lua — mechanical filters and folds over the snapshot, one-time. Rust's type-checking on the 3% is lost; the mitigation is that the host owns the typed protocol boundary and R-6 snapshots pin plugin behavior deterministically. Choose 5.4 over LuaJIT for vendored-C simplicity and reproducible semantics. CPU limits are coarser than wasm's trap model — adequate for per-entity checks over ~1.7k-entity graphs.

The duplication was never a code-hygiene problem; it was the shadow cast by a runtime that could not run the thing the author wrote. Remove the shadow by changing what the runtime accepts.

## Verdict

**Verdict:** LUA
**Confidence:** 4
**One-line rationale:** Duplication is the extract/embed/vendor/guard pipeline wrapping "Rust crate → blob"; a runtime that executes the authored artifact directly (sandboxed Lua 5.4, vendored, single static binary) deletes the pipeline and all three copies, while KEEP_WASM merely shrinks them.
