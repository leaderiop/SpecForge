# Research: clearing RUSTSEC-2026-0269 — extism release timing vs host-side direct-wasmtime vs documented-risk acceptance

> Ticket: issue #10 on leaderiop/SpecForge (wayfinder map #8, registry trust).
> Researched 2026-09-24. Research only — no code, lockfile, or audit.toml changes made.
> Companion ground truth: `inventory.md` (§ Trust gaps, § Runtime exposure), `.cargo/audit.toml`.

## TL;DR

The patched runtime **already exists upstream**: extism PR #912 ("Upgrade to Wasmtime 48 (LTS)") is merged to `extism/extism` main (wasmtime 48 ≥ the 47.0.4 fix line). What does not exist is a **release**: crates.io still has extism 1.30.0 (2026-06-04) pinning `wasmtime ^43`, and 43.x has no patched build for this advisory at all. **Recommendation: keep the audit.toml ignore (documented-risk stands) and wait for the extism release; if the registry-trust effort needs to ship remote installs before that release lands, bridge with a pinned git dependency on extism main (2–4 h) rather than reimplementing wasmtime integration (1–2+ weeks, permanent maintenance).** Direct-wasmtime is not worth it for a ~460-LOC integration surface.

## 1. The advisory

**RUSTSEC-2026-0269** — wasmtime: "Filesystem sandbox escape when paths or symlinks contain trailing slashes."
- CVSS 8.8 HIGH; reported 2026-08-20, issued 2026-08-31; alias [GHSA-vqjp-4c8c-hfgg](https://github.com/bytecodealliance/wasmtime/security/advisories/GHSA-vqjp-4c8c-hfgg).
- **Patched ranges** ([rustsec.org/advisories/RUSTSEC-2026-0269](https://rustsec.org/advisories/RUSTSEC-2026-0269.html)): `>=24.0.13,<25` · `>=36.0.14,<37` · `>=46.0.3,<47` · `>=47.0.4`.
- Our lock: **wasmtime 43.0.2** (`Cargo.lock`, via extism 1.30.0). There is **no patched 43.x** — the 43 line receives no security backports (stated verbatim in extism PR #912: "Extism currently depends on a Wasmtime release line that does not receive CVE or RustSec backports"). Bumping to 43.0.x can never clear this; only an extism release on 46.0.3+/47.0.4+/48.x can.
- Correction to `.cargo/audit.toml:30` for the map owner: the comment says "extism 1.30 still pins the 41.x line"; the actual pin is **43.x** (extism 1.30.0 = wasmtime ^43, released as the "Upgrade wasmtime to v43" release). Doesn't change the conclusion, but the recorded version is wrong.
- Same bump clears **RUSTSEC-2026-0222** (wasmtime 43.0.2, fixed ≥46.0.2/47.0.3) — already in the ignore list, tracked upstream as extism issue #910.

## 2. Upstream state (as of 2026-09-24)

| Fact | Evidence |
| --- | --- |
| extism 1.30.0 (2026-06-04) is the latest crates.io release; its sole change was "Upgrade wasmtime to v43" (PR #901) | [release v1.30.0](https://github.com/extism/extism/releases/tag/v1.30.0), [docs.rs versions](https://docs.rs/crate/extism/1.30.0) |
| 1.30.0 depends on `wasmtime ^43`, `wasi-common ^43`, `wiggle ^43` | [docs.rs/crate/extism/1.30.0](https://docs.rs/crate/extism/1.30.0) (dependency list) |
| **main is already on wasmtime 48**: `wasmtime = "48"`, `wasmtime-wasi = { version = "48", features = ["p1"] }` | [main `runtime/Cargo.toml`](https://raw.githubusercontent.com/extism/extism/main/runtime/Cargo.toml) |
| PR #912 "Upgrade to Wasmtime 48 (LTS)" (opened 2026-09-01, closed 2026-09-02) is the change on main: replaces `wasi-common` (removed in wasmtime 48) with `wasmtime-wasi` p1 (`WasiP1Ctx`, `add_to_linker_sync`, `FsPerms` read-only preopens), requires Rust ≥1.95, adds optional `with_initialization_fuel_limit` | [extism/extism#912](https://github.com/extism/extism/pull/912) |
| **No release in the 22 days since #912 merged**, and the last release was **3.7 months ago** | docs.rs version history (1.30.0 · 2026-06-04 ← 1.21.0 · 2026-03-26 ← 1.20.0 · 2026-03-19 ← 1.13.0 · 2025-11-25 ← 1.12.0 · 2025-07-14) |
| Release timing is not maintainer-committed: issue #909 ("Release 0.130.1?", 2026-07-17) has zero responses | [extism/extism#909](https://github.com/extism/extism/issues/909) |
| Maintainer posture on security bumps: responsive (nilslice invited the #910 PR within hours), and the community PR path worked (#912 by @bgmerrell) | [extism/extism#910](https://github.com/extism/extism/issues/910) |

**Answer to "is a patched release shipped or imminent?"** Shipped: no. Imminent: plausible but unscheduled. Historical cadence is roughly quarterly (2–4 month gaps are normal), and a wasmtime-48 release could land any month — but nothing on the tracker promises one, and #909 shows release requests can sit unanswered for months. The repo lives at `extism/extism` (the org formerly known as dylibso).

## 3. Our usage surface (local evidence — what extism actually does for us)

`extism = "1.30"` appears in exactly one crate: `crates/specforge-extism` (workspace `Cargo.lock`: extism 1.30.0 → wasmtime 43.0.2). `crates/specforge-wasm` is a pure abstraction layer with **no** wasmtime/extism dependency.

Host-side API surface used (`crates/specforge-extism/src/`):
- **Construction** (`runtime.rs:86-92`): `Manifest::new([Wasm::data(bytes)])` → `PluginBuilder::new(manifest).with_wasi(true).with_functions(fns).build()`. No `allowed_paths`, no config, no fuel limit, no HTTP/filesystem registration — plugins are always built from in-memory bytes.
- **Invocation** (`runtime.rs:140`): `plugin.call::<&[u8], Vec<u8>>(export_name, input)` — raw bytes in/out; errors mapped to `WasmTrapInfo { kind: "call_failed" | "extension_not_found" | "lock_poisoned" }` (kinds pinned by `crates/specforge-extism/tests/*`).
- **Three host functions** (`host_context.rs:56-62`), all `[I64] → [I64]` in namespace `extism:host/user`, using extism's handle-based memory API (`memory_get_val` / `memory_new` / `memory_to_val`):
  - `host_emit_diagnostic` → validates JSON, pushes `Diagnostic` into shared collector (`host_context.rs:64-103`);
  - `host_read_file` → `specforge_wasm::host_read_file_check` policy gate (spec_root containment, call-site matrix) **in host-side Rust**, then reads the file itself (`:105-174`);
  - `host_query_graph` → returns scope-filtered graph JSON (`:176-206`).
  - The protocol docs also name `resolve_ref` as a 1.0.0 host API (`docs/extension-protocol.md:487`), but there is **no host-side `resolve_ref` import today** — the guest SDK declares exactly the three externs above (`crates/specforge-extension-sdk/src/host.rs:34-39`). One less thing for any reimplementation to cover.
- **Composition**: `CompositeRuntime` = `BuiltinRuntime` (native stubs) with extism fallback (`composite.rs`); builtins embed four first-party `.wasm` binaries via `include_bytes!` (`builtins.rs:3-21`); the production entry is `crates/specforge-cli/src/pipeline.rs:17-18` (`ExtismRuntime::with_host_context`).
- **Not actually used**: AOT — `instantiate` ignores `_aot_cache_path` (`runtime.rs:84`), and `crates/specforge-wasm/src/cache.rs:16-19,36-38` states true compile-to-native is "deferred until the Extism runtime exposes an AOT compilation API" (the cache stores byte copies). `engine_pool.rs` is LRU bookkeeping, not real wasmtime engines.

Guest side (would **not** change under any host-side fix): all `.wasm` modules are compiled with **extism-pdk 1.4.1** — the SDK macros generate `#[extism_pdk::plugin_fn]` exports `__handshake` / `__describe` (`crates/specforge-extension-sdk-macros/src/lib.rs:89-97`), the SDK declares `#[extism_pdk::host_fn] extern "ExtismHost"` imports (`extension-sdk/src/host.rs:34-39`), and the four first-party extensions + `fixtures/greet-extension` depend on `extism-pdk = "1.4.1"` directly. Host calls used in production: `__handshake`, `__describe`, and scan/validator exports (`crates/specforge-wasm/src/builtin.rs:66-77`, `specforge-emitter/src/scanner_dispatch.rs:69`). extism-pdk is a guest-only crate — it is unaffected by the host wasmtime version, so the SDK and all first-party extensions stay as-is on every path below.

**What host-side direct-wasmtime must reimplement** (Path B scope):
1. `Engine`/`Module`/`Linker`/`Store` lifecycle incl. pooling allocator and cache config on wasmtime 48 (~200 LOC replacing `ExtismRuntime`).
2. **The extism guest calling convention** — the hard part. Our shipped `.wasm` files speak extism's ABI (kernel-style input/output memory protocol, handle-based i64 host calls, `extism:host/user` import namespace). To keep every existing binary working, the new host must implement the guest side of that convention exactly; PR #912's own text ("Extism kernel input and output operations") shows it is a real protocol, not just a `func(...)` wrapper. Getting it subtly wrong breaks every extension under specific call shapes.
3. WASI Preview 1: `wasmtime_wasi::p1` + `add_to_linker_sync` + `WasiCtxBuilder` (mirroring extism: stdio only, no preopens — see §5).
4. The three host functions re-signed as wasmtime host funcs with equivalent linear-memory marshaling.
5. Trap/error mapping preserving the test-pinned `WasmTrapInfo.kind` taxonomy.
6. (Bonus now unlocked) real AOT via `Module::serialize`/`deserialize` — the thing cache.rs explicitly deferred.

Effort: **ABI-compatible shim: ~1–2 weeks** plus regression exposure across `specforge-extism` tests, CLI e2e, and extension golden tests; **with a guest-ABI migration off extism-pdk (self-defined canonical ABI): ~3–6 weeks** plus SDK major-version churn across 4 extensions + fixtures + docs. After either: permanent ownership of wasmtime API and extism-ABI drift. The extism integration we'd be replacing is ~460 LOC.

## 4. The three paths

### Path A — Wait for the extism release (recommended default)

- **Action now:** none (keep `.cargo/audit.toml` ignore). Monitor crates.io/docs.rs for extism > 1.30.0.
- **Action on release:** bump extism (expect a version-jump like 1.31/1.40 given their history), run the extism/CLI/extension test suites, drop the ignore entries (RUSTSEC-2026-0269 + the 13 companions incl. -0222 — audit.toml already says "drop it when the underlying bump lands"). Host-side code change: **none expected** — `PluginBuilder::with_wasi(true)` survives #912 unchanged (the wasi-common→wasmtime-wasi migration is internal to extism); `with_initialization_fuel_limit` is optional and we configure no fuel (unmetered, same as today).
- **Effort:** ~1–3 h when the release lands. Risk: timing unknown (§2).
- **Bridge sub-option A′ — pinned git dependency:** since #912 is merged, `extism = { git = "https://github.com/extism/extism", rev = "<post-#912 sha>" }` delivers the wasmtime-48 runtime **today**. ~2–4 h of Cargo/lock/CI churn, trivially reversible when the crates.io release lands, but it builds a non-published rev (supply-chain hygiene cost: pin the rev hash, note it in audit.toml, revert on release).

### Path B — Host-side direct-wasmtime

- **Action:** new runtime (or rewrite of `specforge-extism`) on wasmtime 48 + wasmtime-wasi p1, per §3 scope.
- **Effort:** 1–2 weeks (ABI-compatible) / 3–6 weeks (incl. guest SDK migration). Plus permanent maintenance of wasmtime upgrades (extism absorbed wasi-common's removal for us in #912 — on Path B that class of churn becomes ours).
- **Risk:** the reimplementation itself is new attack/correctness surface — a hand-rolled marshaling layer around a sandbox is exactly where memory-safety and protocol bugs live; wasmtime's core sandbox would be used correctly either way, but the advisory is a reminder that host-side glue matters.
- **Payoff:** decoupled from extism's release cadence; ignores dropped now; real AOT unlocked.
- **Verdict:** disproportionate. The dependency exists to provide precisely the machinery we'd be rebuilding; its whole value to us is that dylibso tracks wasmtime LTS so we don't have to.

### Path C — Documented-risk acceptance (current standing state)

- **Action:** keep the ignore; refresh its commentary (research-only note for the map owner, not edited here): (i) correct "41.x" → 43.x; (ii) the fix is not merely "an extism release on a patched wasmtime line" in the abstract — **it is merged on extism main (#912) and only a publish step away**, which materially changes the expected wait; (iii) exposure notes from §5.
- **Effort:** ~1–2 h of doc updates. Risk: unchanged from status quo.
- **Soundness condition:** this acceptance is only defensible while the practical-exposure gate holds (§5) — i.e., until running third-party wasm from untrusted registries is an enabled, user-facing flow. That gate is currently social (audit.toml comment + issue #7), not technical.

## 5. Risk exposure analysis

**How wasm reaches our wasmtime today** (all paths instantiate via `runtime.rs:80-97`):
1. First-party builtins embedded at compile time (`builtins.rs`) — trusted.
2. `specforge.json` `extensions` entries pointing at local `.wasm` paths (`pipeline.rs:24-44`) — first-party/trusted in practice.
3. **Registry installs are live** (`crates/specforge-cli/src/add.rs:58` `install_from_registry` → `specforge-wasm/src/install.rs:19`, sha256-verified against the registry's own metadata, then loaded by the same runtime on next compile) — per inventory §1, §5: no signatures anywhere, the registry is the only integrity anchor, and `http://` registries are silently allowed. A malicious or compromised registry can therefore already get a crafted module onto a user's machine and into `ExtismRuntime`. Git sources are rejected at CLI level (`add.rs:42-54`).

**Why practical exposure is still low today:**
- The vulnerability is a *sandbox escape* — it matters when running **untrusted** modules. Today's default flows execute only first-party/embedded modules; the third-party trigger requires the composite registry attack above (attacker-controlled or MITM'd registry + user installing from it).
- **Plugin sandboxes are configured with nothing to escape from**: manifests carry no `allowed_paths` and no preopens (only `Wasm::data`), so the wasi context extism builds is stdio-only. The advisory's escape primitive is trailing-slash path/symlink resolution in filesystem preopens — with zero preopens, the vulnerable path-resolution surface is likely unreachable from our guests `[INFERENCE — grounded in extism manifest semantics (preopens come only from manifest allowed_paths) and runtime.rs:87-92]`. Note the escape class does **not** affect our `host_read_file`: that check and the file read happen in host-side Rust (`host_functions.rs:66-148`), outside the wasm sandbox.
- This matches audit.toml's own "practical exposure is low today" assessment — with one precision the record should adopt: remote **install** already exists; what the advisory gate actually protects is the decision to **execute third-party modules** whose provenance the trust effort hasn't yet secured.

**When exposure becomes real:** any of (a) a community/untrusted registry is documented or defaulted for users, (b) signing/pinning from the registry-trust work lands *without* this advisory being cleared (signing proves provenance but the module still runs in a vulnerable sandbox), (c) we ever add `allowed_paths`/preopens or per-plugin fs grants.

## 6. Recommendation

**Stay on Path A (wait) with Path C as the standing documented state; use A′ as the triggered bridge; do not start Path B.** Concretely:

1. **Keep the audit.toml ignore** with corrected commentary (43.x not 41.x; fix merged upstream, release pending).
2. **Trigger T1 — extism publishes any release on wasmtime ≥46.0.3:** execute the bump within days (1–3 h), drop all 14 wasmtime-class ignores, confirm `PluginBuilder` API compatibility (expected: none needed).
3. **Trigger T2 — the registry-trust effort reaches "third-party remote installs are a user-facing flow" before T1:** execute **A′** (pinned git dep on post-#912 extism main) to clear the advisory ahead of shipping; revert to crates.io at the next release. A′ is preferred over B in every scenario short of extism going dormant.
4. **Trigger T3 — extism appears dormant** (no release ~8 weeks after the #912 merge, i.e. ~2026-11-01) **and** T2 is approaching: re-open Path B with the ABI-compatible shim scope (1–2 weeks), guest-SDK migration deferred.
5. **Standing rule:** adding `allowed_paths`/preopens, enabling `http://` registries, or shipping any flow that executes non-first-party wasm re-opens this assessment immediately.

Monitoring pointers: docs.rs extism version feed; extism issue #910 (tracks the same advisory class on 43.0.2) and #909 (release pressure); PR #912 as the merged-fix anchor.

## Sources

- https://rustsec.org/advisories/RUSTSEC-2026-0269.html (patched ranges, CVSS, dates)
- https://github.com/bytecodealliance/wasmtime/security/advisories/GHSA-vqjp-4c8c-hfgg (upstream advisory alias)
- https://github.com/extism/extism/releases/tag/v1.30.0 (wasmtime-43 release, 2026-06-04, PR #901)
- https://raw.githubusercontent.com/extism/extism/main/runtime/Cargo.toml (main on wasmtime 48 / wasmtime-wasi p1)
- https://github.com/extism/extism/pull/912 (Upgrade to Wasmtime 48 LTS; wasi-common removal handling; merged 2026-09-02)
- https://github.com/extism/extism/issues/910 (RUSTSEC-2026-0222 on wasmtime 43.0.2 — same-bump class)
- https://github.com/extism/extism/issues/909 (unanswered release request, 2026-07-17)
- https://docs.rs/crate/extism/1.30.0 (published version history; wasmtime ^43 deps)

Local evidence: `Cargo.lock` (extism 1.30.0, wasmtime 43.0.2) · `.cargo/audit.toml:29-34` · `crates/specforge-extism/src/{runtime,host_context,composite,builtins}.rs` · `crates/specforge-wasm/src/{runtime,host_functions,cache,engine_pool,builtin}.rs` · `crates/specforge-extension-sdk/src/host.rs` · `crates/specforge-extension-sdk-macros/src/lib.rs` · `crates/specforge-cli/src/pipeline.rs` · `crates/specforge-cli/src/add.rs` · `docs/extension-protocol.md:487` · `.scratch/wayfinder/registry-trust/inventory.md`.
