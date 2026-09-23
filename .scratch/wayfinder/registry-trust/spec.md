Part of #8 (wayfinder map: registry trust). Collapses decisions #9–#20.

## Problem Statement

A developer who runs `specforge add foo` today downloads and executes a wasm blob whose only integrity guarantee is the registry itself: the hash is computed by the same server that serves the bytes, so a compromised or malicious registry can swap artifacts at will. Anyone can publish any name (publisher identity is a self-chosen token label), the server validates nothing about what it stores, credentials sit in a plaintext file, tokens never expire, and an installed-but-tampered extension is never re-checked against the lockfile. The third-party extension ecosystem — the product's stated growth path — is gated on none of this being true.

## Solution

Packages become **publisher-signed** (Ed25519) and clients **verify signatures end-to-end**, trusting pins and config — not the registry — via TOFU key pinning with fail-closed defaults. Publishing rights become **scope-owned** (first-claim registration, scope-bound tokens, registry-assigned publisher ids). Credentials move to the **OS keyring**; tokens **expire**. The server **rejects invalid publishes** (schema, path, wasm magic, network-on sandbox policies, unsigned packages), **rate-limits**, and moves token administration behind an **admin-scoped API**. Installed extensions are **re-verified against the lockfile on every load**. The wasm-runtime sandbox escape (RUSTSEC-2026-0269) is handled by the documented wait-for-upstream path (map decision, not this spec's code).

## User Stories

1. As an extension publisher, I want `specforge publish` to sign my package with my local key, so that consumers can verify it came from me.
2. As an extension publisher, I want my key generated and stored locally on first publish, so that key management needs no extra tooling.
3. As an extension publisher, I want publish to fail loudly if my manifest and wasm disagree, so that I never ship a broken artifact.
4. As an extension consumer, I want `specforge add` to refuse unsigned packages, so that I never execute unverified code by accident.
5. As an extension consumer, I want the publisher's key pinned on first install, so that later updates are guaranteed to come from the same publisher.
6. As an extension consumer, I want a loud, actionable error when a key changes, so that a substituted publisher cannot slide in silently.
7. As an extension consumer, I want `--allow-unsigned`, so that I can deliberately take the risk for local development fixtures.
8. As an extension consumer, I want `specforge update` to verify signatures before replacing an installed extension, so that updates are as trustworthy as installs.
9. As a security-conscious user, I want a `trusted_keys` allowlist in config, so that I can pre-seed keys instead of relying on TOFU.
10. As a user running specforge in CI, I want `--yes` to accept key changes non-interactively, so that automation does not hang on prompts.
11. As a user who audits their supply chain, I want `specforge.lock` to record the key id beside the hash, so that my pin is auditable.
12. As a registry operator, I want publish requests validated (manifest schema, name/version vs path, wasm magic bytes), so that garbage never enters the store.
13. As a registry operator, I want to reject packages whose sandbox policy enables network, so that v1 packages cannot phone home.
14. As a registry operator, I want unsigned packages rejected once signing is live, so that the trust floor rises for everyone.
15. As a registry operator, I want per-token and per-IP rate limits, so that abuse cannot take the service down.
16. As a registry operator, I want token create/list/revoke behind an admin-scoped bearer API, so that administration does not require filesystem access to the server's database.
17. As a registry user, I want my token to expire by default (90 days), so that a leaked token does not live forever.
18. As a registry user, I want my publish token in the OS keyring, so that my credentials are not a plaintext file on disk.
19. As a headless CI user, I want the env-token flow unchanged, so that my pipelines keep working.
20. As the first publisher of `@myscope/utils`, I want the scope registered to me on first publish, so that nobody can impersonate my namespace later.
21. As a consumer, I want `publisher` to be a registry-assigned identity, so that displayed provenance means something.
22. As a user, I want trust displayed as "signed by key id X" rather than Verified/Community badges, so that trust claims are verifiable facts, not labels.
23. As a user whose laptop was compromised, I want to rotate my signing key and have clients interactively re-pin, so that recovery has a defined path.
24. As a user, I want config-level key revocation (pin-deny), so that I can refuse a compromised key without server infrastructure.
25. As a user running an installed extension, I want it re-verified against the lockfile hash on every load, so that on-disk tampering is detected.
26. As a registry operator, I want yanked versions hidden from new resolves but working when pinned, so that pinning stays reproducible.
27. As a security reviewer, I want the documented downgrade window (registry-controlled version lists) stated plainly, so that residual risk is honest rather than hidden.

## Implementation Decisions

- **Signing scheme**: Ed25519 (ed25519-dalek) over the canonical payload `{name, version, wasm_sha256, manifest_sha256, signed_at}`. ManifestV2 bytes unchanged — the signature covers the manifest's sha256.
- **Key representation**: publisher-held local keypair, generated on first publish; key id = short hash of the public key.
- **Wire format**: signature travels as a `signature` multipart field on publish; stored in `packages.signature` + `packages.key_id`; served in package metadata; recorded in `specforge.lock` entries.
- **Trust anchor**: TOFU — first install pins the key id (lockfile + user config); later installs must match. Fail-closed: unsigned → refuse (`--allow-unsigned` override); key mismatch → refuse, re-pin only through the explicit key-change flow. Config `trusted_keys` allowlist for pre-seeded trust. No Verified publisher class in v1; TrustLevel heuristics are deleted; trust display is "signed by key id X".
- **Key lifecycle**: rotation = sign with new key → clients see key-id mismatch on update → interactive re-pin (old X → new Y, [y/N]; `--yes` for CI). Revocation is config-level (pin-deny) in v1; no server-side revocation; transparency log deferred post-v1.
- **Namespace ownership**: first-claim scope registration; publish rights require token scope ⊇ package scope; `publisher` becomes a registry-assigned account id. A `None`-scope token no longer publishes arbitrary names.
- **Credentials**: OS keyring via the `keyring` crate, 0600 file fallback where no keychain exists; plaintext stores re-saved on next login; env-token CI flow unchanged.
- **Token expiry**: new server tokens carry `expires_at` (default 90 days, explicit `--no-expiry` escape); expired tokens fail validation.
- **Server publish contract**: reject manifest schema violations, body name/version ≠ URL path, missing wasm magic bytes, `sandbox_policy.network ≠ "off"`, and unsigned packages. Storage layout revisited to remove the flat name-collision hazard.
- **Server hardening**: per-token and per-IP rate limits; token admin operations behind an admin-scoped bearer API.
- **Rollback stance**: installed wasm re-hashed and compared to the lockfile on every load (fail with remediation hint on mismatch). Yank hides from new resolves, keeps pinned installs working. The registry-controlled downgrade window is documented, not engineered (signed version lists deferred).
- **Runtime sandbox (RUSTSEC-2026-0269)**: no code in this effort — wait for the extism release carrying wasmtime ≥46.0.3; git-dep bridge is the pre-approved fallback if remote installs must ship first; remote installs stay gated meanwhile.

## Testing Decisions

- **Seam 1 (primary)**: the registry HTTP boundary — integration tests run the registry server and exercise publish → metadata → download → verify → install end-to-end, asserting observable behavior (status codes, metadata fields, install success/refusal). Prior art: existing registry client/server integration tests.
- **Seam 2**: the extension lifecycle (install/load) against a temp filesystem — verification failures, lockfile re-verification, yank/pin behavior. Prior art: existing install tests.
- **Seam 3**: the credential store abstraction — keyring backend behind the existing store trait, file-fallback and expiry behavior via the trait; the OS keychain itself is not mocked beyond the trait boundary.
- Good tests assert external behavior only: a refused unsigned install, a served signature, an expired token rejected — never internal call graphs.

## Out of Scope

- Operating a production registry (infra, domains, ops).
- Full TUF/sigstore metadata frameworks, signed version lists, transparency logs.
- Git-source installs and decentralized distribution.
- Server-side key revocation infrastructure.
- SDK adoption follow-through and native runtime changes.

## Further Notes

- All nine decisions were resolved through wayfinder map #8 (research-informed, user-approved); the map is the decision record — this spec is the build contract.
- The downgrade window and TOFU residual risks must be stated in user-facing docs shipped with this work.
