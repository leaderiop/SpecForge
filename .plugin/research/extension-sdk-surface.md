# Extension SDK Surface — Proc Macros, Generated Code, Authoring Experience, Per-Runtime Redesign

Analyst: `research-specforge-sdk-surface` · Scope: `crates/specforge-extension-sdk` (769-line `lib.rs` + `host.rs` + `testing.rs`), `crates/specforge-extension-sdk-macros` (169 lines), and the four guest crates under `extensions/`. All line references verified against the working tree.

## 1. Inventory: what the SDK actually is

The SDK is three crates, but conceptually **two layers with one seam**:

1. **`specforge-protocol-types`** — wire types shared with the host (`HandshakeResponse`, `DescribeRequest/Response`, all 9 descriptor types, `SandboxPolicy`, `PeerDependency`, `ContributionFlags`). The SDK lib.rs doc (lines 4–6) states the design intent: "the same definitions the host uses — so the protocol cannot drift between the two sides." The types are plain serde structs; nothing in them is wasm-specific.
2. **`specforge-extension-sdk`** — the author-facing vocabulary: `Contributions` trait, `ContributionsBuilder` + 7 nested fluent builders, the pass ABI types (`PassEntity`, `PassEdge`, `PassInput`, `PassDiagnostic`, `PassSpan`, `PassSeverity`), `HostApi` (3 host-function wrappers), and `testing::MockHost`.
3. **`specforge-extension-sdk-macros`** — two proc-macro attributes whose *only* job is to glue layer 2 onto the Extism/wasm ABI.

### 1.1 Proc macro #1: `#[specforge_extension(...)]` (on a struct)

Arguments: `name` (required — compile error without it), `version` (defaults to `CARGO_PKG_VERSION`, falling back to `"0.0.0"` outside cargo), `short` (optional). Unknown keys are rejected at compile time (`macros/src/lib.rs:23-44`).

Expansion (`macros/src/lib.rs:80-101`) — three items:

- the struct, unchanged;
- `fn specforge_extension_build() -> ContributionsBuilder` — hidden function that constructs `ExtensionMeta`, calls `<Struct as Contributions>::contribute(&mut b)`, returns the builder. The author's `impl Contributions for Struct` is the single hook;
- `#[extism_pdk::plugin_fn] pub fn __handshake(Vec<u8>) -> FnResult<Vec<u8>>` — serializes `handshake_json(&specforge_extension_build())`. The handshake is fully **derived**: protocol version comes from `specforge_protocol_types::PROTOCOL_VERSION`, and `ContributionFlags` are computed from what was actually contributed (`sdk/src/lib.rs:218-245`), so flags cannot drift from content. This was a deliberate fix — the `raw_category_flag_tests` module (lines 629–657) documents that formal's migration broke precisely because static-JSON describes didn't raise flags.
- `#[plugin_fn] pub fn __describe(Vec<u8>) -> FnResult<Vec<u8>>` — delegates to `describe_dispatch`: parses `DescribeRequest{category}`, returns the pretty-JSON `DescribeResponse`, and **mirrors the hand-written extensions' error behavior** (return code 1 via `extism_pdk::WithReturnCode` for unsupported categories, lines 298–310).

Notable: the SDK deliberately re-implements the hand-written guests' *failure mode* (exit code 1) for host compatibility — the macro layer is ABI-faithful, not just functional.

### 1.2 Proc macro #2: `#[compiler_pass(name = "...", ...)]` (on a function)

Expansion (`macros/src/lib.rs:151-168`): keeps the original function and adds `#[plugin_fn] pub fn __pass_<name>(input: Vec<u8>)` which `serde_json::from_slice::<PassInput>`s the host snapshot, calls the author's function **with the whole `&PassInput`**, and serializes `Vec<PassDiagnostic>`. The pass function itself is pure Rust over plain data — no extism-pdk types touch author code.

Two defects found by inspection:

- **Doc drift**: the macro's doc example (lines 132–136) shows `fn pass_condition_check(entities: &[PassEntity]) -> Vec<PassDiagnostic>`, but the expansion calls `#fn_name(&request)` — the real signature is `fn(&PassInput)`. The formal guest (extensions/formal/src/lib.rs:73) confirms `&PassInput` is what authors must write. This is C7-03's "stringly/no-IDL" pattern reproducing *inside the SDK's own docs*.
- **Silently ignored arguments**: `CompilerPassArgs` parses `before` and `phase` (doc line 104 advertises them) but the struct only retains `name` (`macros/src/lib.rs:106-128`) — an author writing `#[compiler_pass(name = "x", before = "emit")]` gets a silently no-op hint. Ordering actually lives in the *descriptor* contributed via `PassBuilder::after/before/phase`, i.e. declared twice in two places that can disagree (macro attr vs `c.pass(...)` builder call).

### 1.3 What is *not* macro-wrapped: the naming-convention ABI

The rest of the export surface is convention-based, with no macro and in some cases no SDK type at all:

- **`validate__*` custom validators** — declared via `RuleBuilder::wasm_function("validate__type_field_annotations")` (sdk lib.rs:538); the guest must hand-write an export with that exact name. All four builtins declare these in their describe JSON (`extensions/software/src/describe_validation_rules.json` has `validate__type_field_annotations`, `validate__port_methods`, `validate__event_triggers`) — but per the evidence pack the wasm path never executes them at runtime (native dispatch since the C6-11 fix), so the SDK ships a declared-but-dead ABI.
- **`collect__*` collectors, prompts, grammars, body parsers, migration hooks** — no builder method, no type, no macro. `ContributionsBuilder`'s only door is `raw_category(category, serde_json::Value)` (lines 211–216), the documented escape hatch. The typed builders cover 8 of ~15 categories: entities, edges, fields, shared_fields, enhancements, validation_rules, passes, feature_flags. `surfaces`, `prompts`, `grammars`, `body_parsers`, `collectors`, `analyzers`, `providers`, `renderers` are raw-JSON-only.

### 1.4 The host-context port: `HostApi`

`host.rs` wraps exactly three injected host functions via `#[extism_pdk::host_fn] extern "ExtismHost"`: `query_graph(json) -> String`, `emit_diagnostic(json)`, `read_file(rel_path) -> Option<String>` (sandbox-policy subject). This is the **port boundary of the whole SDK** — 40 lines, gated `#[cfg(target_arch = "wasm32")]`, and the only place extism-pdk leaks into author-reachable API besides the generated exports' signatures.

### 1.5 Testing story

`testing::MockHost` (80 lines) wraps a `ContributionsBuilder` and offers `handshake_json()`, `describe_json(category)`, and order-insensitive JSON `assert_handshake`/`assert_describe` — golden-file testing of wire output **without building wasm or loading a host**. The formal guest additionally unit-tests pass logic as plain functions over `PassInput` structs (extensions/formal/src/lib.rs:309–478). So the SDK already achieves "runtime-free testing" for everything except the wasm ABI itself.

## 2. Authoring experience today (measured on the four builtins)

Minimal plugin:

```rust
use specforge_extension_sdk::prelude::*;

#[specforge_extension_sdk::extension(name = "@you/greet", version = "0.1.0")]
struct Greet;

impl Contributions for Greet {
    fn contribute(c: &mut ContributionsBuilder) {
        c.kind("Greeting", |k| k.description("A greeting").testable(false));
        c.rule("G001", |r| r.check(CheckKind::NoIncomingEdges)
            .target_kind("Greeting").severity(ValidationSeverity::Warning)
            .message_template("greeting '{id}' is unused"));
    }
}
```

…plus `Cargo.toml` with `crate-type = ["cdylib"]`, `specforge-extension-sdk` (pulls `extism-pdk 1.4.1`, serde, serde_json, anyhow), then `cargo build --release --target wasm32-unknown-unknown`. The guest manifest (extensions/formal/Cargo.toml) shows the real dep weight authors inherit.

**But the four builtins barely use the typed path.** `@specforge/product` (extensions/product/src/lib.rs:23-41) serves *all nine* categories through `raw_category` over `include_bytes!` JSON envelopes extracted from the native mirrors by `xtask extract-extension-json`. Only `@specforge/formal` mixes typed calls (`c.pass(...)` × 4, a `peer_dependencies` push) with raw JSON for everything else (extensions/formal/src/lib.rs:25-62). That is: **the fluent builder API — the SDK's raison d'être — is exercised on exactly one category type by exactly one builtin.** The payload is ~97% static manifest data; the typed builders mostly re-describe data that already exists as JSON.

The developer loop is Rust-compile-shaped: edit guest → full cargo rebuild for wasm32 → swap blob. No interpret-and-rerun. For the primary author population (AI agents, per the evidence pack) this is the single worst ergonomic property: an agent editing a validation rule pays a wasm toolchain round-trip per iteration, and cannot run the plugin at all without the wasm target installed.

Strengths worth preserving: the single `Contributions` hook; derived flags (can't drift); runtime-free MockHost golden tests; passes as pure functions over plain data (trivially testable, trivially portable); pretty-JSON parity with hand-written output (byte-comparable, which the `extension_json_sync` guard test depends on).

## 3. What the SDK would become under each candidate runtime

The decisive observation: **the macro layer exists only to satisfy one ABI (Extism `plugin_fn` exports).** Everything an author touches — builder calls, pass functions, the three `HostApi` methods — is runtime-neutral already. Under any non-wasm runtime, the two proc macros (169 lines) evaporate entirely and are replaced by the runtime's native registration idiom. The protocol-types and builder layer survive unchanged, because manifests were always language-neutral JSON.

### 3.1 KEEP_WASM — SDK stays, needs four concrete fixes

1. Close the macro gaps: `#[validator]` / `#[collector]` attributes generating `validate__*` / `collect__*` exports (the ABI that's currently convention-only and runtime-dead), and make `compiler_pass` honor or reject `before`/`phase` instead of silently dropping them.
2. Fix the `compiler_pass` doc signature (C7-03 in miniature).
3. Typed builders for the remaining categories (surfaces, prompts, grammars, collectors) so `raw_category` stops being the path of least resistance — today the SDK's typed layer is a dead-weight demo for 3 of 4 builtins.
4. An IDL (schema for the 9 descriptor categories + pass ABI) generating `specforge-protocol-types` — this is the structural fix that makes any *future* second SDK (3.2–3.5) derivable rather than hand-ported.

### 3.2 LUA (mlua) — SDK becomes a ~200-line Lua module + host loader

```lua
-- greet.lua  (the whole plugin)
local specforge = require("specforge")
specforge.extension {
  name = "@you/greet", version = "0.1.0",
  kinds = { { name = "Greeting", description = "A greeting" } },
  rules = { { code = "G001", check = "no_incoming_edges",
              target_kind = "Greeting", severity = "warning" } },
  passes = {
    condition_check = function(input)        -- input.entities, input.edges
      local out = {}
      for _, e in ipairs(input.entities) do
        if e.kind == "behavior" and nonempty(e.fields.requires)
           and not nonempty(e.fields.ensures) then
          out[#out+1] = { code = "W096", severity = "warning",
            message = ("behavior '%s' declares requires but no ensures"):format(e.id) }
        end
      end
      return out
    end,
  },
}
```

The builder callbacks (`|k| k.description(...)`) collapse into plain tables; `Contributions`/`contribute` becomes one registration table; `#[compiler_pass]` becomes a table key. `HostApi` maps to three mlua-injected Rust functions — the port is unchanged, only the FFI. Authoring: **no toolchain, no build step**, iteration is edit-and-rerun — the strongest possible answer to R-5 hot reload (reload script = re-`load` the chunk) and to AI-agent loop speed. Determinism R-6 is achievable: Lua 5.4 (skip LuaJIT's FFI entirely) with `os`, `io`, `require`, `package` removed from the sandboxed environment is a pure deterministic language for pure-data-in/diagnostics-out workloads like formal's 478 lines (they're all loops and string formatting — translate mechanically). Corroboration (mlua README): mlua's `Lua::sandbox` exists for **Luau only**, and its docs flag the `debug` library as an unsafety source — so on Lua 5.4 the env-stripping is hand-rolled by the host (remove `os`, `io`, `debug`, `require`/`package`, then inject exactly the three HostApi functions); mlua now supports Lua 5.5 as well, and its `vendored` feature builds Lua statically from source, satisfying R-3 with no system packages. Costs: no static types (agent authors get feedback only at run time), pass logic loses the compiler's backstop, and the host must carefully freeze the env per plugin (capability story = "we removed the dangerous stdlib", weaker than a permission prompt). Testing: host embeds mlua in `cargo test`, loads the plugin chunk, asserts describes — MockHost survives nearly intact.

### 3.3 PYTHON (PyO3) — SDK becomes a pip package; API shape maps 1:1, runtime properties don't

```python
# greet.py
import specforge

@specforge.extension(name="@you/greet", version="0.1.0")
class Greet:
    def contribute(self, b):
        b.kind("Greeting").description("A greeting")
        b.rule("G001").check("no_incoming_edges").target_kind("Greeting")

    @specforge.pass("condition_check", after="resolve")
    def condition_check(self, input):        # input.entities
        return [specforge.Diagnostic.warning("W096", f"...") for e in input.entities if ...]
```

The decorator API is a near-literal translation of the attribute macros — **the conceptual migration is the cheapest of the three**, and every AI agent on earth writes fluent Python. But the SDK inherits CPython's problems wholesale: R-2 has no real answer (any `import os`/`subprocess` is ambient; no capability model, and "trust the author" is banned by R-1), R-3 requires shipping a pinned CPython (~15–40 MB per platform, historically fragile) or depending on system Python (explicitly banned by R-3's "no system packages"), and R-6 is undermined by stdlib nondeterminism surfaces (hash randomization, locale, time). GIL also serializes plugin calls against host threads. Verdict for the SDK specifically: the nicest API on paper, the worst runtime contract.

### 3.4 TYPESCRIPT (deno_core or QuickJS) — SDK becomes an npm package; the typed-builder DX is preserved

```ts
// greet.ts
import { defineExtension } from "@specforge/sdk";

export default defineExtension({
  name: "@you/greet", version: "0.1.0",
  contribute(b) {
    b.kind("Greeting").description("A greeting");
    b.rule("G001").check("no_incoming_edges").target("Greeting");
  },
  passes: {
    condition_check: (input) =>          // input: PassInput (typed)
      input.entities.filter(e => e.kind === "behavior" && e.fields.requires && !e.fields.ensures)
        .map(e => ({ code: "W096", severity: "warning", message: `behavior '${e.id}' ...` })),
  },
});
```

This is the only scripting candidate that **keeps the current SDK's defining property — a typed, discoverable builder surface**. The `PassEntity`/`PassDiagnostic` structs port to TS interfaces verbatim; a TS LSP gives agent authors the same feedback loop the Rust SDK gives humans (criterion 2 and 8), and tree-shakes the "no static typing" objection that sinks Lua. Under deno_core, the permissions story is verified (Deno security docs): no fs/net/env/subprocess access by default, resource-scoped grants (`--allow-read=./data`), `NotCapable` errors on violation — a natural mapping for capability-scoped `HostApi.read_file` (R-2 friendly). Two verified caveats: all code on one thread shares one privilege level, so per-plugin isolation means running each extension in a Web Worker with a reduced permission set (Deno's own documented guidance for untrusted code), and the initial static module graph loads imports without permission checks — the host must own module loading. Maintenance note: the standalone `deno_core` repo has been merged into the `denoland/deno` monorepo. Under QuickJS you get small binaries but must build the capability wall yourself. Manifest half: tables/objects, same as Lua. Hot reload: fresh isolate per edit — V8 isolate creation is milliseconds; QuickJS faster still. Determinism: enforce by freezing `Date`, `Math.random`, disabling remote imports (`--no-remote` in deno terms). Cost: deno_core's static weight competes with wasmtime's (criterion 10) — the SDK gets nicer but the binary may not get smaller; QuickJS keeps the binary small but you own TS transpilation (or accept JS-only authoring).

### 3.5 MULTI — the SDK forks into three artifacts

Under MULTI the right structure is: (a) a **language-neutral protocol spec + IDL** (the real C7-03 fix — descriptors and pass ABI are already JSON-shaped; only the export/calling convention needs writing down), (b) **thin per-language authoring layers** (the Rust crate/macros stay for wasm guests; `specforge.lua`, `@specforge/sdk`, `specforge-py` as ports of the same vocabulary), (c) a host-side runtime trait with one implementation per engine. The manifest half needs zero change — it is already data. The risk is R-1 drift: sandbox guarantees differ per runtime (wasm memory isolation vs removed-stdlib vs V8 permissions vs CPython none), so "all plugins equal" becomes per-runtime policy reconciliation, and every protocol evolution now lands in 3–4 SDKs plus the spec. This is the maximal-maintenance option and should be priced accordingly.

## 4. Cross-cutting findings the decision should hear

1. **The SDK proves the workload is portable.** Plugin logic = pure functions over `(entities, edges)` snapshots returning diagnostics; host context = 3 calls; manifest = static JSON. Nothing in the four builtins needs Rust. The 478-line formal guest is loops and string formatting — every candidate runtime carries it trivially. The *hard* part of any migration is the host's wasm machinery (9.4k LOC, someone else's scope), not the SDK or the plugins.
2. **C7-03 (no IDL) is an SDK disease.** The hand-mirrored `specforge-protocol-types`, the doc-signature drift in `compiler_pass`, and the two-places-to-declare-`after` problem are all the same root cause: the contract lives in code on both sides with no generating schema. Whatever runtime wins, fixing this first makes the SDK question cheap to revisit later.
3. **The typed builder layer is currently a proof-of-concept, not the product.** Builtins bypass it via `raw_category` because the data originates as JSON in the native mirrors. A scripting SDK that makes the manifest *the source* (tables/objects in the plugin itself) would dissolve the C7-11 three-implementations problem rather than inherit it — the mirror + extraction + sync-guard apparatus (`xtask extract-extension-json`, `extension_json_sync`, `builtin_blob_sync`) exists only because manifests are JSON living inside Rust-compiled blobs.
4. **Hot reload (R-5) and author ergonomics point the same direction.** A wasm guest cannot be "edited and rerun" — it must be recompiled. Every scripting candidate collapses the author loop to file-save. For AI-agent co-authoring (the product's stated primary mode), loop latency is the authoring experience.
5. **R-4 survives all options.** Signed-registry distribution of source files (lua/ts/py) is simpler than blobs; reproducibility then requires pinning the interpreter/engine version in the lockfile — a new but small mechanism (Lua 5.4 version pin, deno/TS transpile pin, or CPython patch pin, the last being the fragile one).

## External corroboration (sources)

`web_search` was unavailable (provider sign-up errors); verified via direct source reads per fleet guidance. All read 2026-09-27.

- **mlua README** (github.com/mlua-rs/mlua): Lua 5.5/5.4/…/LuaJIT/Luau support; `vendored` feature builds static Lua(JIT) from source via `lua-src`/`luajit-src` (R-3 with no system packages); `Lua::sandbox` is **Luau-only**; module-mode docs call out the `debug` library as an unsafety source.
- **Deno "Security and permissions"** (docs.deno.com/runtime/fundamentals/security): secure-by-default — no fs/net/env/subprocess without explicit, resource-scoped `--allow-*` grants; `NotCapable` errors; all code on the same thread shares one privilege level; executing untrusted code → Web Workers with reduced permission sets plus OS-level sandboxing; the initial static module graph loads imports without permission checks; an external permission broker (v1 JSON schemas) exists for policy-driven decisions.
- **extism-pdk 1.4.1** (docs.rs/extism-pdk/1.4.1): `plugin_fn`, `host_fn`, `FnResult`, `WithReturnCode` all present — confirms the exact ABI surface the SDK macros generate against (§1.1–1.2; pinned `extism-pdk = "1.4.1"` in both the SDK and guest Cargo.tomls).
- **denoland/deno_core** README: the standalone repo has been merged into `denoland/deno`; issues/PRs live in the monorepo.

## Bottom line

The SDK is a thin, cleanly-ported thing wearing a heavy toolchain coat: two 169-line macros generate wasm exports for what is, everywhere else, runtime-neutral data and pure functions. KEEP_WASM keeps the coat and must patch real gaps (dead `validate__*` ABI, silently-dropped macro args, untyped categories, no IDL). LUA buys the fastest author loop and easiest determinism at the price of static typing; TYPESCRIPT is the only candidate that preserves the typed-builder DX the SDK was designed around, at V8's binary weight; PYTHON has the friendliest API and the least defensible sandbox/distribution story. Whoever wins, the protocol/IDL — not the macro layer — is the durable asset, and it is runtime-independent today.

## Verdict

**Verdict:** TYPESCRIPT
**Confidence:** 3
**One-line rationale:** The SDK's own evidence — typed-builder DX as its core value, pure-function workload, agent-first authors — says the runtime should preserve static typing and a fast edit-rerun loop, which deno_core/QuickJS uniquely combine; Lua is the close runner-up if binary weight dominates, and the IDL should be built first under any verdict.
