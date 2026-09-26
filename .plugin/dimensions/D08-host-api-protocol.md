# D08 — Host API & Interface Design (C7-03)

**Lens (Katz):** a host API is a dependency contract between host and plugin. The question is not which runtime has the nicest objects, but which one makes the contract explicit, versioned, and tool-enforced — the Cargo/Bundler discipline applied to an interface.

## 1. What exists today

The entire host↔plugin surface is one trait method: `call_export(extension_name, export_name, &[u8]) -> WasmCallResult::{Ok(Vec<u8>), Trap{kind, message, export_name}}` (`crates/specforge-wasm/src/runtime.rs:44`, `:5-16`). The API is a set of stringly-named exports — `__handshake`, `__describe`, `__pass_<name>`, `collect__*`, `validate__*`, `scan__*`, `initialize`. Above it sits `specforge-protocol-types` (566 lines of hand-maintained serde structs, `PROTOCOL_VERSION = "1.0.0"`, 13 categories, `crates/specforge-protocol-types/src/lib.rs:14-31`). The shared crate stops host↔SDK drift for *describe*, but the runtime contract is still stringly: `DescribeResponse.items` is raw JSON until `parse_items`, and everything past describe — pass payloads, collector results, lifecycle, surfaces — is ad-hoc JSON parsed per callsite (`crates/specforge-cli/src/analyze.rs:183`, `crates/specforge-wasm/src/contributions.rs:177-180`, `crates/specforge-wasm/src/surface.rs:214-215`). C7-03 stands; drift already happened and sync tests (`extension_json_sync`, `builtin_blob_sync`) are hand-rolled IDL enforcement.

- **Errors:** trap strings for crashes; malformed pass output → `eprintln!` warning and silently dropped (`analyze.rs:194-197`); host functions return `{"error": ...}` JSON in-band (`crates/specforge-extism/src/host_context.rs:150-153`) — permission denials are invisible to the guest. No typed error channel anywhere.
- **Graph access:** push-whole-snapshot per pass — the host serializes all entities+edges and copies them into linear memory once *per pass per extension* (`analyze.rs:156-181`) — plus `host_query_graph`, which clones the entire cached graph on every call with `QueryScope::All` hardcoded (C7-09, `host_context.rs:191-198`). No streaming, no cursors, no partial reads.
- **Versioning:** semver **major-only** gate at handshake (`host.rs:66-71`); capability negotiation is "category present/absent". Any descriptor change requires a new host *and* regenerating every guest blob (`include_bytes!` embedding makes contract evolution a recompile of all four builtins).
- **The symptom pattern:** `query_scope` and `max_execution_ms` are declared protocol surface but ignored (C7-09/C7-10); `validate__*` exports are in the manifest contract yet never fire — custom rules execute natively in the host (evidence.md §1.2); the SDK's `CheckKind::Custom` is annotated "not yet wired in production" (`crates/specforge-extension-sdk/src/lib.rs:60-63`). Interface surface without enforcement is this codebase's recurring failure mode.

## 2. The host API per candidate

### KEEP_WASM — evolve to WIT / component model
Grep confirms zero component-model usage today; the stack is Extism-PDK raw memory. The evolution: a `specforge:plugin@1.0.0` WIT world — typed `describe() -> contributions`, `run-pass(name, snapshot) -> result<list<diagnostic>, pass-error>`; host-side interfaces `specforge:host/graph@1.0.0` (query with a real scope parameter), `specforge:host/diagnostics@1.0.0`, `specforge:host/fs@1.0.0` (capability-checked, fixing the C7-04 surface at the type level).

- **Graph access:** component *resources* allow a `graph-snapshot` handle with methods, so the snapshot crosses the boundary once per instance instead of per pass call — the fix for both the per-pass copies and the clone-per-query.
- **Streaming:** wasmtime's component model has native stream types; chunked snapshot transfer becomes IDL, not a bespoke protocol you invent.
- **Errors:** typed `result<T, E>` replaces trap strings and in-band `{"error": ...}`.
- **Versioning:** WIT packages version interfaces (`@1.0.0`); guests declare the world they target; the host can serve two worlds side by side — additive evolution without recompiling guests. This is the same contract model the project already gestures at in `peer_dependencies` and `crates/specforge-wasm/src/lock_file.rs`.
- **Cost:** replace the thin extism wrapper with direct `wasmtime::component` bindings; regenerate four guests with wit-bindgen; protocol-types becomes generated or WIT-validated, retiring the sync tests.

### LUA (mlua)
Host API = callback tables: `sf.register_kind{...}`, `sf.pass("name", fn)`, graph as Lua tables. Small manifests read beautifully. But describes become *registration-by-execution* — manifests stop being pure data extractable without running untrusted code, a regression against R-4 verification. Graph-as-tables means converting 1.7k entities + edges into Lua tables per call (allocation-heavy versus a memcpy). No native streaming: a cursor API would be another hand-rolled protocol, hosted in Lua. Errors are `pcall` + ad-hoc objects; versioning is whatever convention you bolt on. You end up rewriting protocol-types as a Lua schema document with no tooling to enforce it.

### PYTHON (PyO3)
Richest object surface (`class Pass: def run(self, graph)`), but the GIL serializes all extensions; PyO3 per-object conversion of snapshots is slow; and R-2 is decisive — any exposed host object is escapable via `ctypes`/`os`, so sandboxing forces a sidecar process. That turns the host API into IPC + serialization: the wasm boundary rebuilt with worse tooling, plus R-3's bundle-CPython fragility and R-5's unreliable `sys.modules` hot reload.

### TYPESCRIPT (deno_core / QuickJS)
The honest point in its favor: the protocol is already JSON, so a JS engine consumes `__handshake`/`__describe` nearly unchanged, and deno_core's permission model is the best scripting-tier sandbox. But that also means TS inherits C7-03 as-is — no IDL appears by switching transports — and graph conversion pays per-entity serde_v8 overhead like Lua/Python. The QuickJS variant has no permissions at all. V8 + deno_core roughly re-spends the wasmtime weight budget (evidence.md §4).

### MULTI
The decisive interface question. MULTI forces the shared contract to the lowest common denominator — a generic call shim over JSON — i.e. it institutionalizes C7-03 and multiplies hand-maintained adapters of the same 13-category vocabulary. The only candidate where "one protocol, many guest languages" is the IDL's *job* is the component model: WIT bindings are generated for Rust, C, JS, and Python guests. MULTI's tractable form is therefore KEEP_WASM+WIT, not a fleet of interpreters.

## 3. Assessment

Ranked on this dimension (tractability + evolvability):

1. **KEEP_WASM→WIT** — the only option with a real IDL: tool-checked multi-language bindings, versioned interfaces, typed results, streams, resources. Fixes C7-03 structurally and makes C7-09/C7-10-style declared-but-ignored surface impossible (world types are enforced by construction).
2. **TYPESCRIPT** — unchanged protocol, best scripting sandbox, but zero contract improvement at V8's price.
3. **LUA** — pleasant micro-API, contract anemia, describe-by-execution regression.
4. **PYTHON** — fine interface on paper; sandbox forces it out-of-process; worst embedding story.
5. **MULTI-as-interpreters** — contract dilution by construction.

The evidence pattern — drift already happened, sync tests impersonating an IDL, ignored protocol fields, unwired contract paths — says the disease is interface surface without enforcement. Exactly one candidate's ecosystem makes enforcement the default instead of a discipline you maintain by hand.

## Verdict

**Verdict:** KEEP_WASM
**Confidence:** 4
**One-line rationale:** The host API needs an enforced contract, and the component model is the only candidate whose IDL provides typed, versioned, multi-language bindings — every interpreter option rebuilds protocol-types by hand or dilutes the contract to a JSON shim.
