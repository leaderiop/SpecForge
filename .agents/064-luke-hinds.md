# 064 — Luke Hinds

**Cluster:** C8 — Registries, packaging & supply-chain trust
**Roster role:** Sigstore co-founder — keyless artifact signing
**SpecForge anchors:** signing extension artifacts (crates/specforge-cli/src/publish.rs → registry-server handlers.rs/auth.rs: token auth + server-side SHA-256, no signatures yet), verification hook (crates/specforge-wasm/src/integrity.rs)

## Why this engineer
SpecForge's publish path today authenticates with registry tokens and pins SHA-256 digests server-side (handlers.rs), but published .wasm artifacts are unsigned — provenance rests entirely on the registry. That is the gap Hinds built Sigstore to close: keyless signing backed by a Fulcio certificate authority and a Rekor transparency log, so signatures cost an OIDC login instead of a key ceremony. Extending integrity.rs's existing verify hook from "digest matches" to "digest matches AND signature chains to the publisher" follows the adoption-first path he proved on containers and binaries.

## References for SpecForge
**Key works**
- [Sigstore](https://github.com/sigstore) — GitHub, sigstore, 2021–present. Cosign (signing), Fulcio (keyless CA), Rekor (transparency log) — the component map for signing SpecForge extension artifacts.
- [sigstore.dev](https://www.sigstore.dev) — Official site. Design docs for keyless signing: short-lived certificates bound to an identity, auditable in a public log.
- **Sigstore: Software Signing for Everybody** — ACM CCS, 2022 (Newman, Meyers, Torres-Arias). The project's paper: usability evidence that keyless signing moves real-world adoption — the constraint SpecForge's signing UX must respect.
- [lukehinds](https://github.com/lukehinds) — GitHub. Creator of Sigstore; former Red Hat open-source security lead; OpenSSF Technical Advisory Council.

## Study first
1. Cosign's sign/verify flow on a single blob — the minimal version of signing a published .wasm
2. Rekor as public witness: why transparency logs beat private key stores for registry trust
3. Map SpecForge's verify chain: integrity.rs digest check → future Fulcio cert → Rekor inclusion proof
