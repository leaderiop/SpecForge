# Registry/Trust Surface Inventory — SpecForge

> Ground truth for wayfinder map #2 (registry trust). Compiled 2026-09-24 from source.

## 1. Client (`crates/specforge-registry`)

**Module map** (`src/client/`): `mod.rs` (re-exports) · `registry_ops.rs` (pure orchestration) · `resolver.rs` (semver range resolution) · `auth.rs` (credential resolution/retry) · `credentials.rs` (on-disk store) · `http_client.rs` (`HttpRegistryClient`) · `registry_client.rs` (trait, errors, `RetryPolicy`) · `registry_config.rs` (`RegistryConfig`, `AuthMethod`, `TrustLevel`). No `publish.rs` in the client — publish orchestration lives in `registry_ops.rs` + `http_client.rs`; the CLI entry is `crates/specforge-cli/src/publish.rs`. Non-network top-level modules: `manifest/`, `registries/`, `compilation/` (registry-as-data for entity kinds).

**Network/auth/verify functions:**
- Trait `RegistryClient` — `registry_client.rs:104` (`fetch`, `search`, `publish`, `authenticate`). `RegistryResponse { name, version, wasm_url, sha256 }` at `:8`.
- `HttpRegistryClient` — `http_client.rs:52`: `resolve_token` (:81), `fetch_versions` (:102, GET `/v1/packages/{name}`), `download_wasm` (:147, raw GET of `wasm_url`), `fetch` (:184), `search` (:237), `publish` (:282, PUT multipart), `authenticate` (:346, POST `/v1/auth/verify`); helpers `parse_specifier` (:389), `parse_retry_after_ms` (:449).
- `registry_ops.rs`: `hex_sha256` (:17), `resolve_from_registry` (:27), `search_registries` (:62), `publish_to_registry` (:102), `verify_registry_integrity` (:138, code R-OPS-002), `assign_trust_level` (:164).
- `resolver.rs`: `resolve_version` (:12, highest semver match).
- `auth.rs`: `resolve_credential` (:9), `sanitize_token` (:52), `validate_credentials` (:63), `logout_registry` (:80), `authenticate_with_retry` (:91, one retry on 401).
- `credentials.rs`: `CredentialStore`/`CredentialEntry` (:9,:16 — `Token{token, expires_at}` | `EnvVar{token_env}`), `get_credential` (:28), `set_token` (:41), `credentials_path` (:56), `read_credentials` (:60), `write_credentials` (:85).

**Publish end-to-end** (`crates/specforge-cli/src/publish.rs:10` `run`): read `manifest.json` → `ManifestV2`, read `wasm` file (`manifest.wasm_path`), pick registry by scope (`find_registry_for_specifier`), credential from `SPECFORGE_REGISTRY_TOKEN` env override else stored credential (`:164` `select_credential`), then `publish_to_registry` (`registry_ops.rs:102`). There it computes `let _sha256 = hex_sha256(package)` (**:110 — computed then discarded; never sent**), does a fetch-based duplicate check unless `force`, then `HttpRegistryClient::publish` PUTs a multipart form (`manifest` = ManifestV2 JSON + `wasm` = raw bytes, `application/wasm`) with optional `Authorization: Bearer` (`http_client.rs:298-312`). **No client-side signature, no hash commitment.**

**Install/download path — EXISTS** (`crates/specforge-cli/src/add.rs:58` `install_from_registry`): `resolve_version` → `resolve_from_registry` (metadata incl. `sha256`) → `download_wasm` → `verify_registry_integrity` (:119) → `specforge_wasm::install_extension` (`crates/specforge-wasm/src/install.rs:19`): re-verifies SHA256 (E032, :29-42), atomic temp-dir rename (:44-89), optional AOT cache, writes `specforge.lock` entry `{name, version, source: "registry", wasm_hash}` (:99-111). Same flow in `update.rs:73-97`. `parse_extension_specifier` (`specforge-wasm/src/discovery.rs:32`) accepts Local / Registry / Git; **Git rejected at CLI level** (`add.rs:42-54`, E-ADD-001) — registry installs are live today.

**Credentials on disk:** plaintext JSON at `~/.specforge/credentials.json` (`credentials.rs:56-58`; `dirs_home` falls back to `USERPROFILE` then `.`). No keyring, no permission hardening in `write_credentials` (:85). `expires_at` always `None` (`set_token`, :41-49).

**Token formats:** client `AuthMethod::{TokenEnvVar, TokenFile, Bearer}` (`registry_config.rs:22`); server-issued tokens are `sfr_` + 32 random bytes hex (`server/auth.rs:45`).

## 2. Server (`crates/specforge-registry-server`)

- **Auth** (`src/auth.rs`): opaque bearer tokens; only SHA256 hash stored (`hash_token` :39); `validate_bearer` (:21) hash-lookup; `token_has_scope` (:27) npm-style scope-prefix match (`None` scope = all packages). No expiry anywhere.
- **Routes** (`src/handlers.rs:16-33`): `GET /v1/packages/{name}` (versions), `GET /v1/packages/{name}/{version}` (metadata: sha256, size, publisher, `wasm_url`), `PUT` same path (publish, `DefaultBodyLimit` 64 MiB), `DELETE` same (yank), `GET .../download` (**no auth**), `GET /v1/search`, `POST /v1/auth/verify`, `GET /health`.
- **Auth on handlers:** publish (:248-300) and yank (:476-525) require bearer + scope; download (:177), metadata (:124), search (:218), versions (:96) are anonymous.
- **Storage** (`src/storage.rs`): `LocalStorage` — flat files `{data_dir}/packages/{name with '/'→'_'}/{version}.wasm` (:13-19).
- **SQLite schema** (`src/db.rs:43-76`): `packages(id, name, version, sha256, size_bytes, description, keywords, publisher, published_at, yanked, UNIQUE(name,version))`; `tokens(id, token_hash UNIQUE, scope, label, created_at, revoked)`. **No signature, key, attestation, or provenance columns.** `publisher` = the token's self-chosen label (`handlers.rs:428`).
- **Publish-side verification — none beyond auth/dedup** (`publish_package` :239-467): computes sha256 itself (:357-366), parses manifest JSON only to extract `description`/`keywords` (:369-389). It never validates the manifest as `ManifestV2`, never checks body name/version against the URL path, never checks wasm magic bytes, and stores whatever bytes arrived.
- **Rate limiting:** none server-side (grep over `src/` finds only the `limit` search param, default 50). Client-side 429 handling only (`http_client.rs:415-449`). Server default bind `127.0.0.1:4873` (`main.rs:27-32`); token create/list/revoke are CLI subcommands operating directly on the SQLite file (`main.rs:119-164`) — no admin API, no auth for token ops.

## 3. Wire format

- **Package = raw `.wasm` bytes + sidecar `ManifestV2` JSON** — multipart fields `wasm`/`manifest` on PUT; not a tarball. `ManifestV2` (`specforge-registry/src/manifest/types.rs:8-55`): name, version, manifest_version, wasm_path, contributes, entity_kinds, sandbox_policy, peer_dependencies, … **No hash field and no signature field anywhere in the manifest.**
- **sha256 appears** in: server-computed at publish (`handlers.rs:357-366`), stored in `packages.sha256`, returned in metadata (`PackageMetadataResponse`, `handlers.rs:44-54`), verified client-side at install (`add.rs:119`, `install.rs:29-42`) and recorded in `specforge.lock`.
- **Signatures exist nowhere** in schema, wire format, or code.

## 4. Already-recorded decisions

- `.scratch/wayfinder/sdk-greet-prototype/README.md:17` — "ticket #5 decision: local paths only in v1" for installs (code has since moved: registry install is implemented — see §1).
- `.cargo/audit.toml:29-34` — **RUSTSEC-2026-0269** (HIGH 8.8, wasmtime filesystem sandbox escape via trailing-slash paths; fix only in wasmtime ≥46.0.3; extism 1.30 pins wasmtime 43.0.2 — no patched 43.x exists; upstream fix merged (extism PR #912) but unreleased) ignored with: "no remote extension loading; local first-party fixtures only — MUST be cleared before remote installs ship. Blocking dependency of the registry-trust effort." Backlog tracked in issue #7.
- `docs/extension-protocol.md:491-517` — Sandbox Policy: defaults network/fs off; **the manifest-declared policy is what the host enforces**.
- `README.md:96-101` — public CLI surface: `add`, `search`, `publish` against registries.
- `TrustLevel::{Local,Git,Community,Verified}` (`registry_config.rs:28`) with heuristic `assign_trust_level` (`registry_ops.rs:162-174`: substring "verified" ⇒ Verified).
- No `CONTEXT.md`, no `docs/adr/` — no recorded ADRs on signing/TUF/trust model.

## 5. Trust gaps (concrete, for safe third-party installs)

- **No signing at all**: no signature in `ManifestV2`, no sig/key column in `packages`/`tokens`, client never signs or verifies. A compromised/malicious registry can swap bytes and update `sha256` — the registry is the only integrity anchor.
- **No end-to-end publisher commitment**: `publish_to_registry` computes sha256 and discards it (`registry_ops.rs:110`); hash is computed post-upload server-side, so publisher↔artifact binding is unverifiable.
- **No transport-scheme enforcement**: `RegistryConfig.url` is an unchecked string (`http_client.rs:77`); `http://` registries give silent MITM; no TOFU pinning either.
- **Publish accepts anything**: server never validates manifest (name/version vs URL path, schema), wasm magic bytes, or caps `sandbox_policy` — an installed extension authors its own sandbox policy, which the host then enforces (`extension-protocol.md:493`).
- **No namespace ownership**: `publisher` is a self-declared token label; any scoped token (or `None`-scope token) can publish any name; no scope reservation, no org verification behind `TrustLevel::Verified`.
- **No expiry/rotation**: tokens never expire (`tokens` table has no expiry; client `expires_at` always `None`); revocation requires filesystem access to the server's SQLite via CLI.
- **No server hardening**: zero rate limiting, anonymous metadata/download/search, no admin auth for token ops, flat-filename storage (`name.replace('/', "_")` — collision hazards).
- **Plaintext credential store**: `~/.specforge/credentials.json` default perms, no OS keyring.
- **Runtime exposure**: RUSTSEC-2026-0269 sandbox escape in the pinned wasmtime line (via extism) is an explicit blocker for remote installs per `.cargo/audit.toml`.
- **No update framework**: no replay/rollback protection — `resolve_version` trusts whatever version list the server returns; a server can downgrade clients within `^`/`~` ranges.
- **Lockfile pin is write-only today**: `specforge.lock` records `wasm_hash` at install, but no re-verification of installed binaries against it on subsequent runs (`lifecycle.rs:12` computes hash for cache-keying, not attestation). [INFERENCE on the re-verify point; lockfile recording is cited.]
