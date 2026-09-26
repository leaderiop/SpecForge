# KEEP_WASM steel-man

Framed through Dan Gohman's lens (`.agents/049-dan-gohman.md`): wasm security is capability-based — no ambient authority, every import granted explicitly. That is not an aspiration here; it is the shape of the code, and the audit findings are gaps in follow-through, not architecture.

## What the evidence shows

The current stack (extism 1.30.0 → wasmtime 43.0.2, ~9.4k LOC in `crates/specforge-wasm/src/`) already delivers the hard properties by construction:

- **R-1 is enforced mechanically, not by convention.** The four builtins are loaded through the identical path as any registry plugin: `include_bytes!` blobs → `load_module_bytes` → `instantiate` → `call_export` (`crates/specforge-extism/src/builtins.rs:3-29`, `crates/specforge-extism/src/runtime.rs:57-97`). There is no trusted tier in the code — any replacement runtime would have to *build* this equality; KEEP_WASM only keeps it.
- **R-2's foundation is a VM boundary, not a discipline.** Plugins run in linear-memory isolation; the host graph crosses via explicit copy (`plugin.memory_new`, `crates/specforge-extism/src/host_context.rs:196-199`). All authority flows through explicitly registered host functions (`build_host_functions`, `runtime.rs:86`); `host_read_file_check` (`crates/specforge-wasm/src/host_functions.rs:66-148`) already enforces call-site gating, `..`-escape rejection, and symlink canonicalization. Network and process access exist only as host grants — a plugin cannot reach what is never imported. Evidence §4's alternatives: PyO3 "none by default", quickjs "none built-in", mlua interpreter hooks — embedders must *correctly strip* ambient libraries forever; Wasmtime's fault isolation cannot be misconfigured away.
- **R-3/R-4 are done, not promised.** Wasmtime is statically linked — zero system packages. Blobs are vendored (`extensions/*/wasm/`, 324–415 KB each) and content-addressed by SHA-256; the registry verifies downloads against DB hashes, pins signing keys, and commits atomically (evidence §1.4). A wasm blob is *byte-reproducible semantics*; a `.py` or `.lua` script's behavior depends on whichever interpreter version the user has.
- **R-5/R-6 fit the model natively.** A wasm module is inert data — hot reload is re-instantiate-from-bytes, no interpreter global state or GIL. Execution without host-imported clocks or randomness is deterministic, which is what makes R-6's snapshot tests meaningful.
- **The hardest protocol work already exists.** One `PROTOCOL_VERSION` const, semver-typed incompatibility errors, and a deterministic handshake-validate-describe sequence (C7 auditor 050: "the spine the rest of the extension system should be rebuilt around"; auditor 048: the versioned handshake "is the hardest part of an IDL to retrofit" — and it is already present).
- **Polyglot is inherited, not foregone.** `wasm32-unknown-unknown` means any language with a wasm backend can author plugins. KEEP_WASM is not "Rust for everyone"; it is "any wasm-targeting language", which is a strictly larger authoring surface than choosing exactly one scripting language.

## Analysis

The honest question is the marginal cost of fixing C7-02/03/04/08/09/10 *inside* the model versus restarting. Against the source, each fix is bounded engineering with a named lever already identified by the audit:

- **C7-04 (allow-by-default) + C7-09 (query_scope ignored) — same plumbing, do together.** `default_sandbox_policy()` sets `file_system_access: Some(true)` (`crates/specforge-wasm/src/sandbox.rs:22`), and the carefully merged most-restrictive-wins policy (`sandbox.rs:31-103`) is computed then never threaded into instantiate. Meanwhile `compute_extension_query_scope` and `filter_graph_by_query_scope` exist and are tested (`host_functions.rs:25-42, 380-392`) — the only live call site passes `QueryScope::All` (`crates/specforge-extism/src/host_context.rs:196`). Fix: flip the default to deny, add component-boundary path checks, and thread `SandboxPolicy` + a per-plugin `query_scope` field through `HostContext` at load. Auditor 054: "the fix is small... the same day as the sandbox policy threading since it is the same plumbing." Cost: small code, large review surface; builtins are pure graph computation, undisturbed by deny-by-default.
- **C7-10 (max_execution_ms unused).** The 30s default is merged (`sandbox.rs:46`) but never enforced, and the single `plugins.lock()` is held across `plugin.call` (`runtime.rs:117-140`), serializing every extension. Fix: per-extension `Arc<Mutex<LoadedPlugin>>` plus Wasmtime epoch interruption — Extism exposes the engine (auditor 055). Epoch traps yield a typed timeout error, which R-6 needs under adversarial plugins anyway.
- **C7-08 (EnginePool is a ledger).** `WarmInstance { extension_name, memory_mb }` (`engine_pool.rs:22-25`) is bookkeeping only; every load pays a fresh compile (`runtime.rs:87-92`). The LRU/eviction/memory-ceiling logic is correct, thread-safe, tested (`engine_pool.rs:114-264`). Auditor 053: "less a rewrite than a promotion" — store real instantiated handles, measure cold-vs-warm handshake+describe.
- **C7-03 (no IDL).** The drift already happened (`resolve_ref` vs `host_query_graph`). The stopgaps prove the team knows the shape: `extension_json_sync` and `builtin_blob_sync` guard tests already regenerate and byte-compare payloads. Fix: a published JSON Schema + golden conformance fixtures (auditors 050/052), or a WIT package via wit-bindgen at auditor 048's "maybe two days". The versioned handshake — the part that resists retrofit — exists.
- **C7-02 (AOT byte-copy).** `cache.rs:14-25` says it outright: byte-copy with `.aot` suffix, `_aot_cache_path` ignored (`runtime.rs:84`). Wasmtime's `Module::serialize`/`deserialize` sits under Extism (auditor 047: "a contained, high-leverage change"). Honest caveat: serialized artifacts are wasmtime-version/platform-bound, so the cache key needs the runtime version — `InvalidationReason::RuntimeVersionChange` (`cache.rs:7-12`) already anticipates it; payoff is cold-start only, since per-call overhead is already modest at these payload sizes.
- **C7-11 (three parallel implementations).** The native mirrors in `crates/specforge-emitter/src/builtins/` are kept alive only by the guard tests; production runs the blobs (evidence §1.2). The in-model fix is auditor 051/056's: collapse onto the embedded-wasm path and delete the trait and emitter list — painful for a day, and it *is* the R-1 convergence the brief demands.

Aggregate: all six fixes land inside a 9.4k-LOC crate behind 3,070 passing tests, touching no registry, distribution, toolchain, or versioning surface. Switching runtimes re-authors the SDK and proc macros, re-implements every capability host function in a new idiom, migrates the formal guest's four real passes (~478 lines), and forfeits the vendored-blob pipeline — to re-acquire properties the current stack already owns.

## Risks & costs

Staying is not free, and the steel-man must pay its own bills:

1. **Weight.** Wasmtime dominates the dependency tree (485→511 locked deps) and binary size — evaluation criterion 10 is a real, permanent tax.
2. **Authoring friction.** Plugins need a Rust toolchain + wasm32 target. Today's authors are AI agents, but a human shipping a validation rule faces a compile step; the in-model mitigation is SDK ergonomics, and a scripting tier would genuinely lower that floor (that is MULTI's argument, not KEEP_WASM's).
3. **Over-machinery for the current payload.** Guests are ~97% generated manifest, ~3% logic (evidence §2) — heavy runtime for what the builtins compute. The rebuttal is the registry bet: R-4 anticipates third-party plugins, where isolation must precede the first untrusted upload.
4. **The C7-04/C7-09 threading touches many call sites** — one shared `HostContext` serves all extensions today; per-plugin scope means plumbing changes, not just a field.
5. **Wasmtime version churn** binds AOT artifacts and mandates cache invalidation on upgrade.
6. **Deadlines remain aspirational until merged.** None of these findings is fixed by deciding to stay; the risk is winning the verdict and never scheduling the remediation.

## Recommendation for SpecForge

Keep Extism/Wasmtime and run the six findings as a sequenced in-architecture remediation program: (1) sandbox deny-flip + policy/query_scope threading (C7-04, C7-09), (2) per-extension locks + epoch interruption (C7-10), (3) pool promotion to real warm instances with cold/warm benchmarks (C7-08), (4) JSON Schema/WIT IDL + conformance fixtures (C7-03), (5) true AOT serialize/deserialize (C7-02), with the C7-11 collapse onto the wasm-only path as the umbrella.

## Verdict

**Verdict:** KEEP_WASM
**Confidence:** 4
**One-line rationale:** The wasm stack already delivers R-1 through R-6 by construction (memory isolation, explicit capability imports, static single-binary, signed reproducible blobs, data-shaped hot reload, deterministic execution), and the audit findings are bounded follow-through fixes — sandbox flip, policy threading, epoch limits, pool promotion, WIT/Schema IDL, real AOT — not architectural dead ends.
