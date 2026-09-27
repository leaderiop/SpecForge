# Registry Protocol Implications — Artifact Format & Signing per Plugin Language

Scope: `crates/specforge-registry-server/src/handlers.rs` (server) and
`crates/specforge-registry/src/client/registry_ops.rs` (+ `client/http_client.rs`,
`signing.rs`, `manifest/types.rs` for the contracts they reference). All line refs
verified at rev `4c9e9f2`.

## 1. What publish uploads today

`PUT /v1/packages/{@scope/name}/{version}` with a manually-built multipart body
(`http_client.rs:298-353`) carrying **exactly three fields**:

| Part | Type | Content |
| --- | --- | --- |
| `manifest` | `application/json` | `ManifestV2` serialized once; the same bytes are hashed into the signature and stored verbatim (`registry_ops.rs:113-131`) |
| `wasm` | `application/wasm`, `filename="extension.wasm"` | the raw guest binary (`http_client.rs:328-334`) |
| `signature` | text | `PackageSignature` wire object, **mandatory** server-side (`handlers.rs:490-498`) |

Server-side validation pipeline (`handlers.rs:291-566`), in order:

1. Scoped name check (`@scope/name`, one `/`, no `%`) — handlers.rs:302-311
2. SemVer version — handlers.rs:315-320
3. Bearer auth + scope permission + per-token/IP publish rate limit — handlers.rs:323-395
4. Duplicate-version reject (409) — handlers.rs:398-415
5. Signature presence — `UNSIGNED_PACKAGE` — handlers.rs:490-498
6. **Wasm magic bytes**: `!wasm_data.starts_with(b"\0asm")` → `INVALID_WASM` — handlers.rs:507-512
7. Manifest parses as `ManifestV2` + `validate_manifest` schema pass — handlers.rs:521-541
8. Manifest name/version ≡ URL path — `NAME_MISMATCH` — handlers.rs:544-552
9. `sandbox_policy.network_access = true` rejected (v1 policy) — handlers.rs:555-565

Then: server computes SHA-256 itself over the uploaded bytes (never trusts the
client, `handlers.rs:570-577`), extracts `keyId` from the signature object and
stores the full signature **verbatim** (`handlers.rs:638-655`), and commits
atomically — fsynced temp file → DB `UNIQUE(name,version)` arbitrate → atomic
rename, with DB-row rollback if the rename fails (C8-06, `handlers.rs:601-726`).

Download mirrors this: `read_wasm` + SHA-256 re-check against the DB digest,
`INTEGRITY_VIOLATION` on mismatch, served as `Content-Type: application/wasm`
(`handlers.rs:208-252`). Version metadata serves `wasm_url`, `signature`,
`key_id`, `manifest` (`handlers.rs:176-195`).

## 2. How signing works (artifact-type analysis baseline)

`signing.rs:1-7,81-115`: publisher-held **Ed25519** key over a canonical payload
`{name, version, wasm_sha256, manifest_sha256, signed_at}` — camelCase JSON,
struct field order = wire order (deterministic). Wire object:
`{"sig", "keyId", "pubkey", "signedAt"}` where `keyId` = first 16 hex chars of
`SHA256(pubkey)`. Key auto-provisioned at `~/.specforge/signing-key.json`.

Client publish (`registry_ops.rs:104-164`): serialize manifest **once** → sign
`{name, version, sha256(package), sha256(manifest_json), now}` → PUT multipart.
Client verify on install (`registry_ops.rs:203-265`): parse signature →
cross-check server-extracted `key_id` vs signature object (`R-TRUST-004`) →
`verify_signature` over **downloaded bytes** + **served manifest bytes**. The
registry is not the trust anchor (spec #21); clients pin keys at install.

**Key property: the signature binds byte strings, not artifact semantics.** Both
digests are `SHA256` over opaque bytes (`registry_ops.rs:240-241`), so the
Ed25519 scheme, canonical payload shape, key-id derivation, offline
verification, and key pinning are **identical for .wasm, .lua, .py, and .ts
artifacts with zero cryptographic changes**. Only the misnomer `wasm_sha256`
(`signing.rs:46`, `registry_ops.rs:127,241,245`) becomes wrong on the wire.

## 3. Per-artifact-type deltas

### .lua (mlua)

- **Artifact**: single UTF-8 text file, typically 1–50 KB (vs 324–415 KB wasm
  blobs). No container format; multi-file would need a zip convention.
- **Upload change**: part name/content-type only (`text/x-lua`,
  `filename="extension.lua"`).
- **Validation change**: the `\0asm` gate (handlers.rs:507) has no analogue —
  Lua has no magic bytes. Options: (a) require valid UTF-8 + non-empty +
  manifest-declared runtime; (b) vendored syntax check server-side (a `luac -p`
  equivalent is feasible but adds interpreter weight to the server). Today's
  gate is already shallow — it never compiles the wasm — so (a) loses nothing
  real; integrity is enforced by SHA-256 + signature, not by format sniffing.
- **Sandbox policy mapping** (`SandboxPolicy`, types.rs:200-215):
  `max_memory_mb` → mlua memory limit; `max_execution_ms` → instruction-count /
  debug hooks; `file_system_access`/`allowed_paths` → interpreter capability
  table (note C7-04: current wasm sandbox is allow-by-default for fs, so the
  script runtimes would need to do *better*, not equal); `network_access`
  rejection logic is runtime-agnostic and stays.

### .py (PyO3 or sidecar)

- **Artifact shape is the real fork**: single `.py` module vs package tree.
  Cleanest protocol answer: keep the single-blob contract; multi-file ships as
  a zip (which *does* have magic bytes, `PK\x03\x04` — but only if the
  convention is adopted; a single `.py` still has none).
- **Extra policy surface**: CPython's stdlib is an ambient-capability problem
  (evidence.md §4) — a py plugin's `SandboxPolicy` would need an allowlist
  dimension (e.g. allowed stdlib modules) that wasm never needed. That's a
  manifest schema extension, not a protocol break.
- **R-3 tension is host-side, not registry-side**: whether the interpreter is
  bundled or system-installed changes nothing in the publish contract.
- Signing/integrity: identical (bytes are bytes).

### .ts (deno_core / QuickJS / sidecar)

- **The one candidate with a genuine protocol decision**: publish **source**
  (`.ts`) or a **transpiled bundle** (JS bytes)?
  - Source: the digest/signature would bind *source*, not *executed bytes*;
    install-time transpilation breaks R-4 reproducibility (bundler version
    drift → different bytes from the same package) and makes the server-side
    `sha256` verification of "what runs" vacuous.
  - Bundle: pin the bundler at *publish* time, sign the bundle — "sign what
    executes" is preserved. This is the only R-4-compatible option.
- Content-Type `application/javascript`; everything else as per .lua.

## 4. What stays stable under any candidate

1. **Trust model end-to-end**: Ed25519 over canonical payload, verbatim
   signature storage, keyId cross-check, offline verify, key pinning. Zero
   changes — digests are artifact-agnostic.
2. **Multipart shape**: `{binary, manifest, signature}` — only the binary
   part's name/content-type/filename change (or generalize).
3. **Atomic publish** (temp+fsync → DB arbitrate → rename + rollback,
   `storage.rs:3-4,42-88`): byte-agnostic, untouched.
4. **Download integrity gate** (`INTEGRITY_VIOLATION`): byte-agnostic.
5. **DB schema**: `sha256`, `size_bytes`, `signature`, `key_id`, `manifest`
   columns are all type-free. No migration.
6. **Everything around the artifact**: scoped names, SemVer, scope claiming
   (first-claim-wins), bearer auth + scopes, rate limits, duplicate-version
   arbitration, yank, search, multi-registry resolution (`registry_ops.rs:25-94`).
7. **ManifestV2 contributions schema**: entity kinds, edges, fields, rules,
   enhancements, collectors, surfaces — all language-agnostic already.
8. **Description/keyword extraction** from manifest JSON (handlers.rs:579-599).

## 5. What breaks (enumerated)

| # | Break | Location | Severity |
| --- | --- | --- | --- |
| B1 | `\0asm` magic-byte gate rejects every non-wasm artifact outright | handlers.rs:507-512 | hard |
| B2 | `wasm_path` is a **required** manifest field (no serde default); `validate_manifest` E030 requires `wasmPath`, and grammar contributions require `grammarWasmPath` | manifest/types.rs:12,314-322,328-332 | hard |
| B3 | multipart field `wasm` + `filename="extension.wasm"` + `application/wasm` | http_client.rs:329-333, handlers.rs:425 | wire |
| B4 | signature payload field `wasm_sha256` | signing.rs:46, registry_ops.rs:127,241,245 | wire |
| B5 | metadata field `wasm_url` + download `Content-Type: application/wasm` | handlers.rs:60,166,250 | wire |
| B6 | storage/storage-fn naming (`store_wasm_temp`, `commit_wasm`, `read_wasm`, `wasm_path()`) | storage.rs | cosmetic |
| B7 | e2e/tests and tooling pinned to wasm naming (registry e2e login→publish→install) | tests | soft |
| B8 | `SandboxPolicy` semantics assume wasm-shaped limits; script runtimes need interpretation layers (and py needs new dimensions) | types.rs:200-215 | semantic |

B1–B4 are the only structural ones. Note B2 bites **immediately**: a `.lua`
publish with a generalized `entry` field would fail `validate_manifest` before
the magic-byte check is even reached.

## 6. Migration shape (if MULTI / a scripting tier is chosen)

1. Add an artifact discriminator to `ManifestV2` — e.g. `artifact: { type:
   "wasm" | "lua" | "python" | "js-bundle", entry: string }` or flat
   `runtime` + `entryPath` — replacing `wasm_path`; keep `grammar_wasm_path`
   behind the same generalization (B2's second half).
2. Dispatch artifact validation per type at handlers.rs:500-512 (magic bytes
   for wasm; UTF-8 for scripts; `PK` for zip convention if adopted).
3. Rename `wasm_sha256` → `artifact_sha256` in the canonical payload. This is
   a wire break — do it under a `manifest_version: 3` bump, **now**, while the
   only publishers are the four builtins (evidence.md §1.5); after third
   parties exist, this same rename is a ecosystem-coordination event.
4. Generalize the multipart part name (`wasm` → `artifact`) in the same bump;
   keep `application/wasm` as the wasm content-type.
5. Metadata response: `wasm_url` → `artifact_url` (additive alias if old
   clients must survive; the CLI and server ship in lockstep today).

None of this touches signing mechanics, atomic commit, integrity gating,
auth/scopes, or the DB.

## Bottom line

The registry protocol is the **least** wasm-coupled subsystem in the runtime
decision. The trust chain (Ed25519 over `{name, version, artifact_sha256,
manifest_sha256, signed_at}`), the atomic publish commit, download integrity,
and the entire auth/scope/search surface are byte-string-agnostic and survive
every candidate unchanged. The wasm coupling is shallow and enumerable: one
hard validation gate (magic bytes), one hard schema requirement (`wasmPath` +
`grammarWasmPath`), and a handful of wire-visible names (`wasm_sha256`,
`wasm_url`, multipart `wasm` field, `application/wasm`). Generalizing costs a
small, one-time protocol bump — and the window to do it cheaply (only builtin
publishers exist) is open now. The only candidate-specific protocol hazard is
TypeScript: publishing source instead of a pinned bundle would silently break
R-4, because the signature would no longer bind executed bytes. Signing needs
**no per-artifact-type design at all** — SHA-256 binds opaque bytes equally
well for Lua text, Python zips, and JS bundles; the field rename is honesty,
not mechanics. LUA and PYTHON additionally push sandbox-policy semantics
(memory/exec/fs capability mapping) into interpretation layers the registry
merely stores — a manifest-schema concern, not a protocol break.

## Verdict

**Verdict:** MULTI
**Confidence:** 3
**One-line rationale:** The signed-registry protocol is artifact-agnostic except ~4 shallow couplings all fixable in one cheap v3 bump while only builtins publish — so the protocol seat removes any registry-based objection to adding a scripting tier, though it cannot by itself pick which tier wins.
