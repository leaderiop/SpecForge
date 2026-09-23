# Research: signing approaches for wasm package registries

> Ticket: leaderiop/SpecForge#9 (map #8, registry trust) · Researched 2026-09-24 · Research only — no code changes.
> Ground truth: `.scratch/wayfinder/registry-trust/inventory.md` (raw `.wasm` + `ManifestV2` JSON multipart; sha256 computed server-side at publish and re-verified client-side at install; **no signature anywhere** in schema, wire format, or code).

## TL;DR recommendation

**For v1 (one registry, end-to-end signed installs, no TUF): publisher-held Ed25519 signatures via `ed25519-dalek`, over a canonical message covering `{name, version, wasm_sha256, manifest_sha256, signed_at}`. The registry stores and serves signatures and public keys but is not a trust anchor; the client verifies fail-closed at install and pins keys in `specforge.lock`.**

Do **not** adopt for v1: `sigstore` crate (self-described experimental, very heavy deps), Rekor/transparency logging (defer — `sigstore-rekor` is the light path later), TUF/tough, or warg (no longer developed; see below). Registry-held signing (npm layer-1 / VS Code model) is cheap defense-in-depth but fails the stated E2E goal: it only proves *downloaded == what the registry accepted*, which is exactly the sha256 status quo.

## 1. Ecosystem survey

| Ecosystem | What the signature covers | Key holder | Transparency log | Client enforcement | Status |
|---|---|---|---|---|---|
| cargo / crates.io | Nothing (sha256 served by registry over TLS) | n/a | none | none — cargo never verifies signatures | No signing; direction is trusted publishing (OIDC), not signatures |
| npm | L1: `name@version:tarball-integrity` (registry ECDSA); L2: provenance+publish attestations binding tarball hash to source repo + CI run | L1: registry key in HSM; L2: ephemeral Fulcio certs from CI OIDC | L2: Rekor | opt-in (`npm audit signatures`) | Shipped; PGP signatures deprecated 2023-04-25 |
| deno / JSR | Provenance attestation binding package to source repo + build (SLSA/Sigstore) | Ephemeral keyless (Sigstore), from GitHub Actions OIDC | Rekor | automatic on install from provenance-enabled publishers | Default for CI publishes; token-less trusted publishing |
| VS Code Marketplace | Full VSIX payload: "downloaded == uploaded to the Marketplace" | Marketplace-held signing key (registry signs at upload) | none | on by default; `extensions.verifySignature` toggle; hard-fail error possible | Shipped; *publisher* signing still not shipped (under discussion since 2022) |
| warg (Bytecode Alliance) | Every release log entry: version, content hash, publisher key, grants/revocations — verifiable full package history | Per-publisher Ed25519/ECDSA-P256 keys + registry operator key | Built-in: immutable verifiable logs (TUF-flavored) | full history verification by client and third-party monitors | **No longer actively developed**; ecosystem moved to OCI + cosign |

### 1.1 cargo / crates.io — no signing at all

crates.io has no package signing; integrity is the sha256 the registry itself computes and serves, over TLS — the same trust model SpecForge has today (inventory §5: "the registry is the only integrity anchor"). A 2023 pre-RFC to add Sigstore signing on publish/verify stalled and was never merged: <https://internals.rust-lang.org/t/pre-rfc-using-sigstore-for-signing-and-verifying-crates/18115>. The accepted direction is **trusted publishing** — RFC 3691, short-lived OIDC credentials instead of long-lived tokens (<https://rust-lang.github.io/rfcs/3691-trusted-publishing-cratesio.html>) — which reduces credential theft but adds no artifact signatures.

**Lesson:** even the largest Rust registry ships without E2E signing. If SpecForge wants it (it does — "E2E signed installs" is the goal), it is greenfield; there is no cargo precedent to copy.

### 1.2 npm — two independent layers

- **Layer 1, registry signatures:** the registry ECDSA-signs `${package.name}@${package.version}:${package.dist.integrity}`; public keys published at `/-/npm/v1/keys`, key held in an HSM per stated best practice; third-party registries are encouraged to implement the same convention. This protects against tampering in transit or by mirrors — *not* against a malicious publisher (the registry signs whatever arrives). PGP signatures were deprecated 2023-04-25 in favor of this scheme: <https://docs.npmjs.com/about-registry-signatures>.
- **Layer 2, provenance:** DSSE attestations (provenance + publish) generated at publish time from GitHub Actions/GitLab OIDC via Sigstore's Fulcio (short-lived certs) and logged in Rekor; verification via `npm audit signatures`: <https://docs.npmjs.com/generating-provenance-statements>. This is the layer that binds artifact → source repo → CI run, i.e. real publisher commitment.

**Lesson:** the two-layer split is the right lens. npm's registry signature ≈ SpecForge's existing sha256 check (transport integrity); E2E publisher binding comes only from the provenance layer. npm does E2E with heavyweight infrastructure (Fulcio + Rekor + OIDC CI).

### 1.3 deno / JSR — provenance by default

JSR generates SLSA/Sigstore provenance automatically when publishing from GitHub Actions, writes Rekor transparency-log entries, and supports token-less trusted publishing (<https://jsr.io/docs/provenance-and-trust>, <https://deno.com/blog/how-we-built-jsr>). The newest registry skipped registry-held signing entirely and went straight to provenance + transparency log — viable because publishing is CI-centric and the registry operates the Sigstore integration for everyone.

**Lesson:** provenance-first is elegant but presumes Sigstore infrastructure and CI-only publishing. SpecForge v1 has neither; adopting it means operating/depending on public Sigstore services and shipping a large verification stack to every client.

### 1.4 VS Code Marketplace — registry-held signing, shipped

"The Visual Studio Marketplace signs all extensions when they are published. VS Code verifies this signature when you install an extension to check the integrity and the source of the extension package" — verification failure can block install (`Cannot install extension because Visual Studio Code cannot verify the extension signature`), with a `extensions.verifySignature` kill switch and error taxonomy (`NotSigned`, `CertificateRevoked`, `EntryIsTampered`, …): <https://code.visualstudio.com/docs/configure/extensions/extension-marketplace>. The design intent from the VS Code team: "all extensions on upload will get signed by the Marketplace. Upon download … VS Code will verify this repository signature. Using this approach we can be certain that the package that VS Code downloaded is indeed the one that got uploaded" — publisher-side signing was explicitly still an open topic (<https://github.com/microsoft/vscode-discussions/discussions/137>). On the Visual Studio (Windows) side, publishers sign VSIX with real CA-issued certificates via Sign CLI; self-issued certificates are rejected by the Marketplace.

**Lesson:** registry-held signing is production-grade and catches mirror/tamper/corruption — but even Microsoft hasn't shipped publisher signing; marketplace publisher identity is handled by a separate verified-publisher system, not by signatures. Registry-held signing alone does not satisfy SpecForge's E2E requirement.

### 1.5 dylibso / Bytecode Alliance warg — right design, wrong fate

Warg is the most ambitious design in the survey and the closest thing to "TUF for wasm": per-package **immutable, verifiable logs** — "Package releases are published to immutable logs signed by their maintainers. Clients, third-party monitors, and importing registries can cryptographically verify a registry's state and history of state changes to detect a compromised or malicious registry" (<https://github.com/bytecodealliance/registry/blob/main/docs/README.md>). Every release entry is signed by a per-publisher key (ECDSA P-256, held in the OS keystore), the operator key signs log metadata, and grant/revoke of publish/yank permissions are themselves logged entries. This is the only surveyed design that defeats a fully compromised registry *and* replay/rollback of version lists.

But: the repo carries a warning banner — "**This repository is no longer being actively developed by Bytecode Alliance members.** Work on an OCI-based registry system continues in bytecodealliance/wasm-pkg-tools" (<https://github.com/bytecodealliance/registry>). The live wasm signing pattern is cosign (keyless OIDC) over OCI artifacts, as practiced by wasmCloud (<https://wasmcloud.com>, "How to sign WebAssembly components with Cosign (OIDC)", Sep 2025).

**Lesson:** don't build SpecForge v1 on warg or its protocol. The verifiable-log idea is worth borrowing only if/when multi-registry federation and rollback hardening become goals.

## 2. What each approach actually covers (analysis)

- **Registry-held signature** (npm L1, VS Code): proves *downloaded == what the registry accepted*. Does not cover a malicious/compromised publisher, a compromised registry (it can re-sign anything), or version-list rollback.
- **Publisher-held signature** (warg, minisign model — SpecForge's stated goal): binds artifact to a publisher key the registry cannot forge; survives registry compromise. Gaps are key lifecycle (loss, rotation, revocation) and replay of old *validly signed* versions (no log). Replay is acceptable for v1 — the lockfile pin (`wasm_hash`, inventory §1) plus semver policy mitigate it; note it as a consciously accepted gap.
- **Keyless provenance + transparency log** (npm L2, JSR): binds artifact to source repo + CI run; strongest supply-chain signal; requires Fulcio/Rekor infrastructure, OIDC-based CI publishing, and a heavyweight client verification stack.

## 3. Rust crate landscape

| Crate | Version (updated) | Downloads | Dep weight | RustSec history | Maintenance notes |
|---|---|---|---|---|---|
| `ed25519-dalek` | 3.0.0 (2026-07-06) | 217.5M | Small, pure Rust: curve25519-dalek, ed25519, sha2, subtle, rand_core, zeroize (+feature-gated keccak, strobe-rs, serde) | RUSTSEC-2022-0093 — Keypair-API signing oracle, fixed by the v2 API removal (patched ≥2); family: curve25519-dalek RUSTSEC-2024-0344 — timing leak in `Scalar29/52::sub`, patched ≥4.1.3 | Active; de-facto standard Ed25519 in Rust |
| `minisign-verify` | 0.2.5 (2026-03-03) | 14.5M | **Zero dependencies** — vendored blake2b/sha512/ed25519/curve25519; verify-only | none | Maintained by the minisign author; "simple, auditable code"; signing requires the external `minisign` tool |
| `sigstore` | 0.14.0 (2026-05-22) | ~1.0M | Heavy: aws-lc-rs (C build), tokio, reqwest, rustls-webpki, x509-cert, openidconnect, oci-client, tough (TUF), chrono, scrypt, p256, … | none filed against the crate | README: "experimental … will not be considered stable until the 1.0 release"; repo active (pushed 2026-09-21) but 75 open issues and no 1.0 since 2021 |
| `rekor` (standalone) | 0.0.0 (2021-08-19) | ~1.3k | n/a | n/a | Dead placeholder crate |
| `sigstore-rekor` | 0.13.0 (2026-09-23) | ~1.3M | Light-ish: 9 normal deps (serde stack, reqwest, sigstore-types/merkle/cache, thiserror, jiff) | none | The maintained Rekor client for Rust, if transparency logging is added later |

Sources: crates.io API (versions/dates/downloads/dependency lists, 2026-09-24); RustSec advisory-db (<https://github.com/rustsec/advisory-db>, `crates/ed25519-dalek/RUSTSEC-2022-0093.md`, `crates/curve25519-dalek/RUSTSEC-2024-0344.md`); sigstore-rs README + Cargo.toml (<https://github.com/sigstore/sigstore-rs>); minisign-verify README + Cargo.toml (<https://github.com/jedisct1/rust-minisign-verify>).

Caveat on audits: I could not locate a formal third-party audit of recent `ed25519-dalek` or `minisign-verify` releases; the maturity signal here is RustSec history, adoption volume, and longevity [INFERENCE]. For contrast, `ring` — the most battle-tested crypto crate — collected three 2025 advisories (RUSTSEC-2025-0007/-0009/-0010): every crypto dependency carries audit-gate churn.

## 4. Dependency-risk notes (cargo-audit context)

`.cargo/audit.toml` already carries 21 ignored advisories and treats dependency weight as a first-class cost. Measured against that gate:

- **`sigstore` (default features)** would add aws-lc-rs (C code, cmake/nasm build friction), the tokio+reqwest async HTTP stack, openidconnect, oci-client, and tough — a large new audit surface, for Fulcio/Rekor/OCI infrastructure v1 deliberately does not have. It also self-declares unstable until 1.0, meaning API churn risk on top.
- **`ed25519-dalek`** adds a handful of pure-Rust crates, no C toolchain, no async runtime. The two family advisories (2022, 2024) are historical and patched; pin current versions.
- **`minisign-verify`** adds zero crates. If v1 wanted format interop with the standalone minisign tool (CI-driven signing), client-side verification cost is literally zero new dependencies; the tradeoff is that signing happens in an external CLI rather than in-process.

## 5. Recommendation for v1

**Publisher-held Ed25519 signatures, `ed25519-dalek` in-process on both sides.**

1. **Scheme & crates:** Ed25519. `ed25519-dalek` 3.x for keygen (`specforge key generate`), signing at publish, and verification at install. No CA, no OIDC, no transparency log in v1.
2. **Signed message:** canonical serialization of `name || version || wasm_sha256 || manifest_sha256 || signed_at`, signed with the publisher key. This covers *both* wire artifacts (the wasm and the ManifestV2 JSON — note the manifest has no hash field today, inventory §3, so the signature is the only place both hashes are bound together). Signing the manifest hash also means a publisher vouches for the sandbox policy it declares — but content review is still on the consumer; signatures prove identity, not benignity.
3. **Wire & storage:** add a `signature` (or `attestation`) part to the publish multipart; new server columns `signature`, `pubkey`, `key_id` on `packages`; serve them in `PackageMetadataResponse`. Keep `ManifestV2` unchanged (the current format has no signature field; mutating it breaks older clients — the signature rides beside the manifest instead). Server-side unsigned-publish policy: allow with a warning during a migration window, then per-registry opt-in enforcement.
4. **Client enforcement:** at install, after `verify_registry_integrity`/E032 sha256 check, verify the signature against the publisher key; fail closed for registry installs, with an explicit `--allow-unsigned` escape hatch for dev registries. Record `pubkey_id` + `signature` in `specforge.lock` alongside `wasm_hash`. Key distribution v1: pin-on-first-install (TOFU, SSH known_hosts style) with a loud warning, plus allowlisted publisher key fingerprints in client config for enforced trust.
5. **Explicitly deferred** (name them in the decision so they don't resurface as scope creep): Rekor/transparency log (path: `sigstore-rekor`, light enough to adopt later), keyless Fulcio/OIDC provenance (npm L2 / JSR model), TUF/tough, warg-style verifiable logs, registry-held counter-signatures (npm L1 / VS Code model — cheap defense-in-depth to add later, does not satisfy E2E on its own), key revocation UX (v1: document rotation-by-republish; a signed revocation list can come later).
6. **What v1 signing does not fix** (cross-ref inventory §5 — separate tickets): namespace ownership, token expiry/revocation, server hardening, version-list rollback/replay, manifest validation on publish. Signing one of nine gaps must not be sold as closing the trust model.
7. **Alternative considered and parked:** minisign-format signatures — publishers sign with the standalone `minisign` CLI in CI, clients verify with the zero-dependency `minisign-verify` crate. Best-in-class dependency story and mature format, but it forces an external binary into `specforge publish` and a second key format. Revisit if CI-driven publishing becomes the primary flow; the wire format above (signature bytes beside the manifest) is agnostic to which scheme fills it.
