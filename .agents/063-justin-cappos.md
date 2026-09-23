# 063 — Justin Cappos

**Cluster:** C8 — Registries, packaging & supply-chain trust
**Roster role:** TUF/in-toto creator — update integrity frameworks
**SpecForge anchors:** publish integrity (crates/specforge-wasm/src/integrity.rs: SHA-256 pinning, E032 hash-mismatch hard error, E028 missing binary, W027 skip warning), lockfile-pinned hashes (lock_file.rs), specforge update checks (crates/specforge-wasm/src/upgrade.rs)

## Why this engineer
SpecForge already does step zero of Cappos' program: every install pins a SHA-256 digest, and verification hard-fails on mismatch (E032), treats a missing binary as an error (E028), and only warns when the user explicitly bypasses (W027). TUF is what that becomes when an attacker shows up: role-separated signing keys, snapshot/timestamp metadata, and rollback protection so a compromised registry or key cannot silently serve stale or malicious artifacts — exactly the threat model specforge publish/upgrade will face once extensions ship from a public registry. in-toto extends the same chain-of-custody thinking to how artifacts get built.

## References for SpecForge
**Key works**
- [The Update Framework](https://theupdateframework.io) — CNCF graduated project, 2009–present. The design SpecForge's digest pinning should mature into: key rotation, expiry, rollback protection for registry updates.
- [theupdateframework](https://github.com/theupdateframework) — GitHub. Spec and python-tuf reference implementations — the metadata schema to borrow for a future SpecForge update channel.
- **Survivable Key Compromise in Software Update Systems** — ACM CCS, 2010 (Samuel, Mathewson, Cappos, Dingledine). The founding paper: why single-key, single-hash update chains fail and how role separation survives compromise.
- [in-toto](https://in-toto.io) — CNCF graduated project. Farm-to-table attestations for build pipelines — the model for proving builtin .wasm blobs match their source in extensions/.
- **in-toto: Providing farm-to-table guarantees for bits and bytes** — USENIX Security, 2019 (Torres-Arias et al.). The formal treatment of supply-chain layout attestations.

## Study first
1. TUF's threat model vs SpecForge's current single SHA-256 pin (integrity.rs)
2. TUF metadata roles: root/targets/snapshot/timestamp and what each key failure leaves intact
3. in-toto link/layout attestations for the builtin-wasm build path (specforge-extism/src/builtins.rs)
