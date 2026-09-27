# mlua deep-dive — embedding Lua 5.4/LuaJIT in Rust for SpecForge plugins

Research date: 2026-09-27. Method: direct reads of the mlua repo, docs.rs, lib.rs/crates.io
metadata, OSV, and an empirical probe written and run for this report (release build,
mlua 0.12.1, features `lua54,vendored,macros`, opt-level 3, macOS arm64 / Apple M3 Pro).
Probe numbers below are first-party measurements; all other claims are cited.

## 1. What mlua is

`mlua` is the dominant Rust binding crate for Lua: Lua 5.5/5.4/5.3/5.2/5.1 (incl.
LuaJIT) plus Luau, async/await support, serde integration, native Rust Lua modules
([README](https://github.com/mlua-rs/mlua)). It started as an `rlua` fork, and `rlua`
itself is now **deprecated in favour of mlua** — since v0.20.1 (Jul 2024) rlua is a
thin re-export wrapper
([lib.rs/crates/rlua](https://lib.rs/crates/rlua)). Ecosystem consolidation is total:
one crate to evaluate, not two.

Feature flags choose exactly one interpreter (`lua54`, `luajit`, `luau`, …) plus
`vendored` (static C build via the author's `lua-src`/`luajit-src` crates — no system
packages, satisfying R-3's no-system-deps clause), `async`, `send` (makes `Lua:
Send+Sync` via a global lock), `serde`, `macros`
([README](https://github.com/mlua-rs/mlua)).

## 2. Maintenance status

| Signal | Value | Source |
| --- | --- | --- |
| Latest release | v0.12.1, 2026-08-29 (Lua 5.5.1 + Luau 0.736 bumps) | [releases](https://github.com/mlua-rs/mlua/releases) |
| Cadence | 0.12.0 Jul 2026, 0.11.6 Jan 2026 (added Lua 5.5), 85 releases since 0.2.0 (2019) | [lib.rs](https://lib.rs/crates/mlua) |
| Downloads | **744,863/month**, used in 603 crates runtime (466 direct) | [lib.rs/rev](https://lib.rs/crates/mlua/rev) |
| Stars/forks | 2,877 / 212 | [GitHub API](https://github.com/mlua-rs/mlua) |
| MSRV / license | Rust 1.88+ / MIT; binding is ~22K SLoC | lib.rs |
| RustSec advisories | none found for `mlua` in advisory-db `crates/mlua` | [advisory-db](https://github.com/rustsec/advisory-db/tree/main/crates/mlua) |

Caveat: development is driven almost entirely by one maintainer (khvzak), who also
rejects adding more VM backends for maintenance reasons
([FAQ](https://github.com/mlua-rs/mlua/blob/master/FAQ.md)). Low bus factor, high
activity. The 0.x series breaks API on each minor: 0.12 reorganized modules and
changed hook callbacks to return `Result<VmState>` (hit first-hand when compiling the
probe). Budget for periodic migration work, or pin versions.

## 3. API design

The surface is a single `Lua` struct (one VM state) with: `load(chunk)` returning a
builder (`set_name`, `set_environment`, `set_mode`, `exec`/`eval`/`call`/`into_function`),
`create_function`/`create_async_function` for Rust→Lua callbacks, `UserData` +
`#[derive(UserData)]`/`#[mlua::userdata_impl]` (new in 0.12) for exposing Rust types,
registry/app-data for host storage, and serde conversion between Rust values and
`mlua::Value` ([docs: Lua](https://docs.rs/mlua/latest/mlua/state/struct.Lua.html),
[Chunk](https://docs.rs/mlua/latest/mlua/chunk/struct.Chunk.html),
[release notes](https://github.com/mlua-rs/mlua/releases/tag/v0.12.0)).

Safety model (self-described): every longjmp from the Lua C API is wrapped in
`lua_pcall`, Rust panics in callbacks become catchable Lua errors (optionally
uncatchable via `LuaOptions::catch_rust_panics(false)`) — but the README is explicit
that mlua "contains a huge amount of unsafe code" and "almost certainly bugs still
lurking"
([README Safety/Panic sections](https://github.com/mlua-rs/mlua#safety)). This is
honest and relevant to R-2: the binding is memory-safe-ish by convention, not by
construction like a wasm boundary.

## 4. Sandboxing under R-2 — measured, not assumed

Probe results (my run, code as described in §Method):

1. **`Lua::new()` is NOT an R-2 sandbox.** It loads the "safe subset", which per
   source is `StdLib::ALL_SAFE = (1<<30)-1` — everything except LuaJIT `ffi` (bit 30)
   and `debug` (bit 31), explicitly marked "(unsafe)"
   ([stdlib.rs:86-100](https://github.com/mlua-rs/mlua/blob/main/src/stdlib.rs)). My
   inventory confirms `Lua::new()` globals include **`os`, `io`, `package`, `require`,
   `dofile`, `loadfile`** (only `debug` is absent).
2. **Flag exclusion has a trap.** `new_with(StdLib::ALL_SAFE & !StdLib::OS &
   !StdLib::IO & !StdLib::PACKAGE, LuaOptions::new())` removes `os`/`io`/`package`
   (verified: `os.time()` → "attempt to index a nil value (global 'os')") — but
   **`dofile` and `loadfile` survive**, because they live in the base library, not the
   `io` library. A hardened host must additionally nil them out or gate every chunk's
   environment.
3. **Per-chunk environments work**: `lua.load(code).set_environment(env)` makes
   `os` resolve to nil inside that chunk even on a state that has it (measured). This
   is the Lua 5.1–5.4 sandbox mechanism mlua recommends for untrusted code, pointing
   to [Luau's sandbox page](https://luau.org/sandbox) as the reference
   ([README Sandboxing](https://github.com/mlua-rs/mlua#sandboxing)). Note
   `Lua::sandbox()` itself is **Luau-only**
   ([README](https://github.com/mlua-rs/mlua#sandboxing)).
4. **Memory limits work on Lua 5.4**: `set_memory_limit(2MB)` → a 50MB
   `string.rep` fails cleanly with `memory error: not enough memory`; 1KB succeeds.
   Implemented via mlua's custom allocator; unavailable only in module mode
   ([state.rs:1097-1112](https://github.com/mlua-rs/mlua/blob/main/src/state.rs)).
5. **CPU timeouts work**: instruction hook every 100k instructions checked a wall-clock
   deadline; `while true do end` was killed at exactly 200.0ms with a clean Lua error.
   Cost caveat in §5.
6. **Bytecode must be blocked separately.** `Chunk::set_mode(ChunkMode::Text)` exists
   ([docs](https://docs.rs/mlua/latest/mlua/chunk/struct.Chunk.html)) and must be
   enforced: Luau's authors removed `load`-of-bytecode entirely because "untrusted
   bytecode may lead to exploits" ([luau.org/sandbox](https://luau.org/sandbox)).

The full hardening checklist for stock Lua is what Luau does natively: drop
`io`/`package`/`debug`/`dofile`/`loadfile`, reduce `os` to clock/date/difftime/time,
forbid bytecode, protect globals, per-script env tables, no `__gc`, interrupt-based
CPU limits ([luau.org/sandbox](https://luau.org/sandbox)). mlua gives you the raw
tools for every item; you assemble and maintain the sandbox yourself.

**Security model evidence that interpreter sandboxes leak:** Redis shipped Lua
sandbox escapes that became RCEs — CVE-2022-24834, a heap overflow in the sandboxed
`cjson` library "triggered by a specially crafted Lua script … potentially remote code
execution", fixed in 7.0.12 ([OSV](https://osv.dev/vulnerability/CVE-2022-24834)).
A bug in any linked C library reachable from plugin code is a host compromise. Wasm's
linear-memory isolation removes this class structurally — that is the categorical
difference R-2 turns on.

## 5. Performance & footprint (probe, M3 Pro) + third-party benchmark

| Probe measurement | Result |
| --- | --- |
| Lua VM baseline memory (`used_memory` after init) | **~23 KB/state** |
| Full hot reload: fresh state + compile + exec 500-entity validation chunk | **399 µs/op** |
| Pure Lua fib(27), no hook | 5.6 ms |
| fib(27) with hook installed @ every 1M instructions | 21.0 ms (**~3.7x**) |
| fib(27) with hook @ every 100 instructions | 22.9 ms |
| Host↔Lua no-op round-trip call | **19 ns/call** |
| 1000-field table Rust→Lua→Rust push+pull | 362 µs |
| Binary size delta (stripped, macOS arm64): empty bin → with mlua+lua54 vendored | 341 KB → 826 KB (**+0.46 MB**) |
| Max RSS, trivial process with one state | 2.1 MB |

Two findings matter for SpecForge:

- **Hot reload (R-5) is essentially free**: 0.4ms to bring up an entire plugin VM and
  run a workload, vs. the current wasm path's engine-pool problem (audit C7-08:
  "EnginePool is a ledger, no warm instances").
- **The timeout mechanism (fixes audit C7-10) works but has a fixed cost**: any
  installed hook roughly 4x's interpreter speed regardless of firing granularity
  (5.6→21ms at both 1M and 100 granularity), i.e. the slowdown comes from the VM's
  hook-enabled dispatch path, not callback frequency. Host should arm hooks only
  around untrusted calls. Luau's purpose-built `set_interrupt` (mlua exposes it for
  Luau only) avoids this.

Third-party comparative numbers — the author's own
[script-bench-rs](https://github.com/khvzak/script-bench-rs) (bias note: mlua's
maintainer wrote it; environment M5 Max, mlua 0.12-rc.2, wasmtime 45.0.1), workload
"sort Rust objects" (evaluation + Rust interop), lower is better: **wasmtime 2.73 ms
(fastest)**, wasmi 10.02, mlua_luau 11.94, roto 13.29, mlua_lua55 15.72, rquickjs
35.05, koto 49.21, boa 101.87, rhai 114.27. Honest reading: an **AOT-compiled wasm
module beats embedded Lua on raw compute** in this benchmark; Lua's edge is
interoperability overhead, startup, and authoring — not being faster than compiled
wasm. (LuaJIT was not benchmarked and would narrow the gap; it also carries Lua 5.1
semantics and the `ffi` library, which is an unrestricted sandbox bypass if enabled.)

Baseline footprint is tiny by design: the Lua 5.5 interpreter with all std libs is
293K and the library 484K on 64-bit Linux, ~32K lines of C, MIT
([lua.org/about](https://www.lua.org/about.html)) — consistent with my measured +0.46 MB.

## 6. Async support

Mature: `async` feature implements async/await over Lua coroutines with any executor
(tokio/async-std), including `exec_async`/`call_async` and async userdata methods
([README](https://github.com/mlua-rs/mlua#asyncawait-support),
[docs](https://docs.rs/mlua/latest/mlua/state/struct.Lua.html)). Proven in production:
KumoMTA's SMTP daemon runs Lua config/policy inside tokio with
`mlua = { features = ["vendored", "lua54", "async", "send", "serialize"] }`
([kumod/Cargo.toml](https://github.com/KumoCorp/kumomta/blob/main/crates/kumod/Cargo.toml)).
0.12 added `VmState::Yield` for hook/interrupt-driven yields
([docs](https://docs.rs/mlua/latest/mlua/struct.VmState.html)). Determinism note for
R-6: awaited interleaving is host-scheduled, so snapshot determinism requires running
plugin passes synchronously (the SpecForge analyze path is sync today).

## 7. Real projects embedding Lua via mlua

- **WezTerm** (GPU terminal) — entire config/plugin API is Lua on mlua 0.9
  ([Cargo.toml](https://github.com/wezterm/wezterm/blob/main/Cargo.toml)).
- **KumoMTA** (production mail transfer agent) — Lua 5.4 config/policy layer, async+send
  ([kumod/Cargo.toml](https://github.com/KumoCorp/kumomta/blob/main/crates/kumod/Cargo.toml)).
- **yazi** file manager — Lua plugin system (yazi-plugin/yazi-core via mlua 0.11).
- **lune** — Roblox-flavoured Lua CLI runtime; **vfox** — version-manager whose plugins
  are Lua scripts; **xplr** — Lua-configurable file manager; **mprocs**, **fblog**,
  **minijinja-lua**, **pact-plugin-driver** (optional Lua plugin tier), **bevy_mod_scripting_lua**
  (Bevy game scripting), **nvim-oxi** (optional), **libafl** (optional scripting in a fuzzer)
  ([lib.rs reverse deps](https://lib.rs/crates/mlua/rev)).

Pattern: mlua hosts are developer tools whose plugin authors are their own users on
their own machines. None combine mlua with a registry distributing untrusted
third-party binaries — the closest analogy (Roblox/Luau) hardened the VM itself and
still recommends separate VMs per script ([luau.org/sandbox](https://luau.org/sandbox)).

## 8. Known limitations (consolidated)

1. **No structural isolation** — interpreter and all exposed C libraries share the
   host's address space; one memory-safety bug reachable from plugin code = host
   compromise (precedent: [CVE-2022-24834](https://osv.dev/vulnerability/CVE-2022-24834)).
2. **Sandbox is a self-assembled checklist**; defaults are unsafe for R-2 (§4.1-4.2),
   and the correctness burden (dofile/loadfile, bytecode mode, env gating, `math.random`,
   `os` removal) stays on SpecForge forever.
3. **Hook-based CPU limits cost ~4x** while armed (measured); Luau-only interrupt API
   is cleaner.
4. **Threading**: one Lua state is `!Send` by default; `send` feature serializes on a
   global lock, or you pool one state per thread. No intra-state parallelism.
5. **Single-maintainer bus factor**, 0.x breaking-change cadence (§2).
6. **LuaJIT trade-offs**: Lua 5.1 semantics; `ffi` (excluded from `ALL_SAFE`) is a
   total sandbox bypass if ever enabled; not covered in the author's benchmark.
7. **Binding contains substantial `unsafe`** by the project's own admission (§3).
8. **Migration cost**: the four builtin guests are Rust (the only logic-bearing one,
   `@specforge/formal`, is ~478 lines); LUA means rewriting that analysis logic in Lua
   and re-validating under R-6, plus losing the wasm SDK's typed exports in favor of a
   hand-maintained host-API surface (audit C7-03: no IDL today — mlua does not fix
   that by itself).

## Bottom line

**Verdict:** KEEP_WASM
**Confidence:** 3
**One-line rationale:** mlua is a mature, tiny, fast-to-reload, production-proven
embedding layer whose sandbox tools (memory limits, instruction-hook timeouts,
env gating) demonstrably work — but R-2 for registry-distributed untrusted plugins
turns on structural isolation, which an in-process interpreter cannot offer without
re-assembling and forever maintaining the Luau-style hardening checklist, while the
existing wasm model already carries the workload and its audit gaps (C7-04/C7-10)
are fixable inside the model.
