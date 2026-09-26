# D10 — Migration Cost from Wasm Builtins

*Lens: Martin Fowler — migration as behavior-preserving change; the cost of a refactor is measured by the size of the seam, not the size of the surrounding machinery.*

## 1. What would actually have to move

Measured inventory (wc -l, working tree at rev 4c9e9f2):

| Asset | LOC / size | Runtime-coupled? |
| --- | --- | --- |
| Guest: `extensions/formal/src/lib.rs` | 478 (≈238 logic+docs, ≈168 tests, ≈72 boilerplate) | logic is pure functions — **portable** |
| Guests: product 41 / governance 53 / software 57 | 151 total | none — pure manifest servers |
| SDK `specforge-extension-sdk` lib.rs | 769 | builders neutral; export plumbing Extism-coupled |
| SDK macros (`-sdk-macros/src/lib.rs`) | 169 | bodies emit `#[extism_pdk::plugin_fn]` — **coupled** |
| `specforge-protocol-types` | 566 | neutral — shared host/guest wire shapes |
| SDK `host.rs` | 39 | `#[extism_pdk::host_fn]` — coupled, and **unused by all four guests** (grep: zero references in `extensions/`) |
| Native mirrors `specforge-emitter/src/builtins/{product,software,governance,formal}.rs` | 2,557 | wasm-adjacent scaffolding kept alive only by the sync guard (evidence.md §1.2) |
| Vendored blobs `extensions/*/wasm/*.wasm` | 1,420 KB | wasm-only |
| Host runtime `specforge-wasm` | ≈9,350 (evidence.md §1.1) | wasm-only |

The decisive fact: **the manifest protocol is already runtime-neutral.** The 9 `describe_*.json` categories (223 KB across the four guests), the handshake JSON, and every wire type in `specforge-protocol-types` are plain serde shapes. `PassInput → Vec<PassDiagnostic>` (compiler pass ABI, SDK lib.rs:659-769) is a pure data-in/data-out contract; none of the four guests call the host API (`host.rs` is dead weight for builtins). The seam a new runtime must implement is exactly two functions per guest plus a JSON frame — not a protocol redesign.

## 2. Per-candidate migration cost

**KEEP_WASM — 0 guest LOC.** Nothing ports; cost is confined to closing the open audit gaps inside `specforge-wasm` (C7-02/04/08/09/10), which are wasm-path defects that must be paid under *every* option that keeps wasm, and are deferred-only under the others. Incumbency also keeps the published crates.io SDK (v0.1.0) byte-stable — though with zero external consumers (evidence.md §1.5: the builtins are the only plugins), SDK stability protects no one today.

**LUA — smallest genuine port.** The three declarative guests (151 LOC) dissolve: they only embed and re-serve JSON, so under a scripting host they become data files the host reads directly — a *deletion*, not a rewrite. Formal's four passes port mechanically: condition_check is a 22-line filter; coverage_tracking a 34-line filter+aggregate; event_graph_analyze a 52-line two-map edge tally; only layering_verify (~103 lines: DFS cycle detection + depth walk over `REFINEMENT_EDGE_MARKERS`, formal lib.rs:144-251) is a real algorithm. Call it ~212 logic LOC → Lua plus ~168 test LOC re-homed (or driven through the host bridge to keep them language-neutral). SDK work: `ContributionsBuilder` and the seven builders survive untouched; the 169-LOC macro crate's *interface* survives (`#[extension]`, `#[compiler_pass]`) while its expansion targets change; `describe_dispatch`'s `extism_pdk::WithReturnCode` error type and 39-LOC `host.rs` are replaced. Registry artifacts change type (.wasm → script bundle) behind an unchanged publish/verify/install flow (sha256+semver+signing are blob-agnostic, evidence.md §1.4). Order: days, not weeks.

**TYPESCRIPT — same shape as Lua.** Formal's passes are filter/map/join over entity lists — idiomatic JS is the closest stylistic match to the existing Rust. Cost identical to Lua except the embedding choice (quickjs = vendored C, deno_core = heavy static) replaces mlua. Days.

**PYTHON — port cost equal, shipping cost worst.** Identical port shape, but R-3 (single binary, no system packages) collides with CPython distribution — the migration *surface* is the same while the delivery risk is highest. Not a migration-cost differentiator so much as a migration-adjacency one.

**MULTI — zero port cost now, permanent protocol surface growth.** Keeping wasm AND adding a tier means two SDKs, two sandbox models, a `runtime` discriminator in registry manifests, and per-runtime determinism (R-6) and hot-reload (R-5) testing forever. C7-11 already counts three parallel implementations of the extension concept; MULTI institutionalizes a fourth. R-1 demands convergence to one mechanism "or justify why each remains" — a justification MULTI must write annually and can never finish.

## 3. Which choice keeps SDK/registry/manifest protocol stable?

Split the question three ways, because the answers differ:

- **Manifest protocol** (describe categories, handshake, wire types): stable under *every* option. It is data, not code; `specforge-protocol-types` already enforces no host/guest drift (SDK lib.rs:4-8).
- **Registry protocol** (publish → verify → install, sha256, signing): stable under KEEP_WASM unchanged; under any swap only the artifact type changes; under MULTI a `runtime` field is added — the only option that *extends* the manifest schema.
- **SDK**: byte-stable only under KEEP_WASM. Under a swap, the exported crate API (`extension`, `compiler_pass`, builder types) can survive nearly intact — only the expansion target and two Extism-plumbing signatures change — but it is a breaking release of a v0.1.0 crate with zero third-party dependents, i.e. cost without casualties.

## 4. The honest quantification

The real migration payload is **~212 lines of portable pure-function logic in one guest, plus ~168 lines of its tests** — everything else (151 LOC of declarative guests, 1.4 MB of vendored blobs, 2,557 LOC of native mirrors, two sync-guard test suites) exists *only* to sustain the wasm arrangement and would be deleted, not ported. Against that: KEEP_WASM retains a ≈9,350-LOC host runtime whose open audit findings (C7-02 AOT-as-byte-copy, C7-08 engine pool ledger, C7-10 unenforced timeouts) are permanent upkeep. Refactoring discipline says: pay the one-time bounded cost only when it removes standing structure; here the standing structure (≈12k LOC of wasm superstructure for 212 lines of logic) is roughly 50× the refactor itself. On this dimension alone, though, incumbency is genuinely cheapest and the margin — days of mechanical porting — is too thin to dominate any other dimension.

## Verdict

**Verdict:** KEEP_WASM
**Confidence:** 3
**One-line rationale:** Migration cost is the dimension where wasm's incumbency legitimately wins — zero port, byte-stable SDK/registry — but the honest numbers (~212 portable logic LOC, everything else deletable scaffolding) make the alternatives' one-time cost small enough that other dimensions should decide.
