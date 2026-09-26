# Evidence Pack — SpecForge Plugin Runtime Decision

All facts verified against the working tree at rev `4c9e9f2` (2026-09-26) unless noted.
Sources: this session's engineering work, the full-project audit (`.reports/js/data.js`,
195 findings), and direct code inspection. File paths are relative to the repo root.

## 1. Current architecture (as-built)

### 1.1 The wasm path (production runtime for CLI/LSP/MCP)

- Host: `crates/specforge-extism` (ExtismRuntime) on **extism 1.30.0 → wasmtime 43.0.2**.
  ~9,350 LOC across `crates/specforge-wasm/src/` (runtime, engine_pool, sandbox,
  cache, integrity, lock_file, install, lifecycle, toposort, protocol/ bridge).
- 4 builtin guests: `extensions/{product,software,governance,formal}/` — Rust crates
  compiled to `wasm32-unknown-unknown` (release), artifacts **vendored at
  `extensions/<name>/wasm/specforge_ext_<name>.wasm`** (324–415 KB each, committed),
  embedded via `include_bytes!` in `crates/specforge-extism/src/builtins.rs`.
  Guest crates: 41–478 lines of lib.rs; `@specforge/formal` is the largest (4
  `#[compiler_pass]` functions with real analysis logic, ~478 lines).
- SDK: `crates/specforge-extension-sdk` (+ `-macros`, `-protocol-types`, published on
  crates.io v0.1.0). Proc macros: `#[specforge_extension(...)]` (generates `__handshake`,
  `__describe` exports wired to the embedded JSON), `#[compiler_pass(name, after)]`
  (generates `__pass_<name>` exports), collectors via `collect__*` naming.
- Wire protocol: **no IDL** — stringly-typed JSON over `runtime.call_export(name,
  export, &payload)`; `WasmCallResult::{Ok, Trap}`. Manifest described via 9
  `describe_*.json` categories (entities/edges/fields/shared_fields/enhancements/
  validation_rules/surfaces/passes/feature_flags), embedded in guests via
  `include_bytes!` and also mirrored as committed JSON next to the guest sources.
- Sync guard: `extension_json_sync` test regenerates the payloads in-memory from the
  native `builtins/*.rs` mirrors and byte-compares with the committed JSON.
  `builtin_blob_sync` test asserts each vendored blob embeds the current payloads.

### 1.2 The native mirrors (the C7-11 duplication)

`crates/specforge-emitter/src/builtins/{product,software,governance,formal}.rs` —
native Rust `Contributions` implementations of the same four manifests (kinds, edges,
rules, enhancements). Used by: in-process native execution in tests, the
`extension_json_sync` extraction, and historical native runtime paths. ~1,000+ lines
across the four mirrors. The CLI/LSP/MCP production path loads the **wasm blobs**, not
these mirrors — the mirrors are kept in sync by the guard test only.

Custom validation rules (E004/E006/E010/W010) currently execute **natively in the
host** (`NativeCustomRules` in `crates/specforge-emitter/src/compile.rs`) — bypassing
the guest `validate__*` exports entirely (that host-side dispatch was the fix for
audit findings C6-11/C10-10 "custom wasm rules can never fire / fail silent").

### 1.3 Known wasm-path audit findings (C7 cluster, all HIGH unless noted)

| id | finding | state |
| --- | --- | --- |
| C7-00 | fresh clone cannot build (blobs in target/) | FIXED (vendored `extensions/*/wasm/`) |
| C7-02 | "AOT cache" is a byte-copy with `.aot` suffix; `_aot_cache_path` ignored | open |
| C7-03 | no IDL: stringly-typed call_export + ad-hoc JSON; drift already happened | open |
| C7-04 | sandbox deny-by-default is allow-by-default for `file_system_access` | open |
| C7-06 | "Wasm is the only runtime" claim false three ways (native builtins, embedded blobs, SDK crates) | open |
| C7-08 | warm-engine story vaporware: EnginePool is a ledger, no warm instances | open |
| C7-09 | `query_scope` ignored: host always uses QueryScope::All | open |
| C7-10 | `max_execution_ms = 30_000` default never enforced at runtime | open |
| C7-11 | three parallel implementations of the extension concept | open |

Also: C14-04 registry-server runs rusqlite/SHA-256/multipart inline in async handlers;
C14-03 LSP does blocking walkdir/parse inline.

### 1.4 Registry & distribution (R-4 surface)

- `crates/specforge-registry-server`: axum + rusqlite (SQLite), scoped bearer tokens
  (SHA-256-hashed, expiry, admin), semver-validated publishes, atomic blob commit
  (temp + fsync + DB arbitrate + rename), download sha256-verified against DB
  (INTEGRITY_VIOLATION on mismatch), percent-encoded package dirs, publish signing
  (signing key auto-provisioned at `~/.specforge/signing-key.json`; signature + key_id
  stored, clients pin keys at install).
- Client: `specforge login/add/publish/search`, credential store prefers OS keyring
  with verified 0600 file fallback.

### 1.5 Consumers & authors

- Primary consumers: **AI coding agents** (MCP), plus humans via CLI/LSP.
- Plugin authors today: this project itself (the 4 builtins are the only plugins).
  The registry is the bet that third parties will author plugins. The product
  explicitly targets AI-agent authoring of specs; the SDK requires a Rust toolchain +
  `wasm32-unknown-unknown` target + wasmtime-capable host.
- Workload shapes: manifest metadata (static), validation over ~1.7k-entity graphs
  (per-entity rule checks), compiler passes over the graph snapshot (analyze),
  collectors scraping external test formats, prompts (text templates).

## 2. Measured facts

- Workspace: 129,828 LOC Rust, 443 files; `specforge-wasm` ≈ 9.4k LOC; blobs 1.4 MB
  total; wasmtime 43.0.2 dominates dependency weight (audit: 485→511 locked deps).
- Tests: 3,070 passing / 0 failing (workspace), including wasm round-trip suites
  (protocol handshake/describe/bridge), storage atomicity, keyring fallback,
  registry e2e (login→publish→search→install verified live on 127.0.0.1:4873).
- Plugin-side LOC: guests 629 lines of lib.rs total (formal 478 — the only
  logic-bearing guest), i.e. **the wasm guest payload is ~97% generated manifest,
  ~3% real logic** across the four builtins.
- Custom-rule dispatch: native (host) since the C6-11 fix; guest `validate__*`
  exports exist in the manifest contract but the wasm path is not exercised by
  builtins at runtime.

## 3. Constraints recap (hard)

- R-1: all plugins equal — no first-party tier, no trusted native path for builtins.
- R-2: sandboxable untrusted plugins (capability-scoped fs/network/process).
- R-3: single-binary host, macOS arm64 + Linux x64, no system packages.
- R-4: signed registry distribution, verifiable + reproducible.
- R-5: hot reload in watch.
- R-6: deterministic, snapshot-testable analyze output.

## 4. Candidate runtimes (facts to verify per candidate)

| Runtime | Embedding crate(s) in Rust | System deps | Sandbox story | Notes |
| --- | --- | --- | --- | --- |
| Wasm (Extism/Wasmtime) | current | none (static) | memory isolation + capability imports | per-call compile unless pooled/AOT; heavy dep tree |
| Lua 5.4 / LuaJIT | `mlua` | none (vendored C) | interpreter-level: no fs/net by default; CPU-instruction hooks (Luau) | tiny, proven for embedding (Neovim, games) |
| Python (CPython) | PyO3 | **system Python or bundled distribution** | none by default; GIL; pip ecosystem is ambient-capability | huge ecosystem; embed+ship is notoriously fragile |
| TypeScript/JS | `deno_core` (V8), `quickjs`, `boa`, or node sidecar | deno_core = heavy static; quickjs = C vendor | deno: permissions model; quickjs: none built-in | TS authoring DX; V8 = large dep |
