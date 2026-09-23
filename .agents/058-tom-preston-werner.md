# 058 — Tom Preston-Werner

**Cluster:** C8 — Registries, packaging & supply-chain trust
**Roster role:** Semantic Versioning spec author; version-compatibility policy
**SpecForge anchors:** handshake version compatibility (crates/specforge-wasm/src/protocol/host.rs, validate_protocol_version; PROTOCOL_VERSION in protocol/mod.rs), extension semver ranges (crates/specforge-wasm/src/lock_file.rs peer_dependencies, upgrade.rs)

## Why this engineer
SpecForge's extension protocol already runs on his compatibility rule: the host parses protocol versions with `semver::Version` and treats same-major as compatible, rejecting mismatched majors at handshake (host.rs). Peer dependency constraints in the lockfile and upgrade checks compare major.minor.patch tuples exactly per his spec. SemVer is what makes `@scope/name@1.0.0` specifiers and protocol v1.0.0 mean anything at all — when peer-dep conflicts or protocol drift need richer range semantics, his spec is the vocabulary.

## References for SpecForge
**Key works**
- [Semantic Versioning 2.0.0](https://semver.org) — semver.org, June 2013. The spec behind SpecForge's protocol_version compatibility rule and extension version fields; the MAJOR.MINOR.PATCH contract for API-facing changes.
- [semver/semver](https://github.com/semver/semver) — GitHub, canonical spec source. Where clarifications and errata to the compatibility rules live.
- [TOML](https://github.com/toml-lang/toml) — GitHub, toml-lang/toml, 2013. His config-format minimalism — a design benchmark for the JSON extension manifest and sidecar files.
- [Jekyll](https://github.com/jekyll/jekyll) — GitHub, jekyll/jekyll, 2008. Proof of the "small core, convention over configuration" philosophy SpecForge applies to its zero-domain-knowledge core.

## Study first
1. SemVer 2.0.0 spec, esp. §4-§8 (versioning in public APIs, ranges left undefined)
2. npm/node-semver range grammar — the de facto standard for the ranges his spec deliberately leaves open
3. SpecForge's own rule: same major = compatible (protocol/mod.rs, host.rs)
