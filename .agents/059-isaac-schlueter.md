# 059 — Isaac Schlueter

**Cluster:** C8 — Registries, packaging & supply-chain trust
**Roster role:** npm creator — registry UX and lockfile model
**SpecForge anchors:** registry UX (specforge add/remove/publish specifiers in crates/specforge-cli/src/add.rs, publish.rs HttpRegistryClient), lockfile model (crates/specforge-wasm/src/lock_file.rs)

## Why this engineer
Schlueter invented the package-registry UX SpecForge's CLI imitates: `name@version` install specifiers, a registry that serves metadata and tarballs separately, and a lockfile that pins the resolved world. SpecForge's lock_file.rs — read/write/refresh entries with pinned versions, integrity hashes, and peer-dependency checks — is npm's package-lock.json pattern applied to extension .wasm artifacts; node-semver remains the reference implementation for the range grammar lock_file.rs's semver-like comparisons approximate.

## References for SpecForge
**Key works**
- [npm](https://github.com/npm/cli) — GitHub, npm/cli, 2009. The original registry-plus-CLI design: install specifiers, scopes, semantic version ranges — the template for specforge add @scope/name@1.0.0.
- [node-semver](https://github.com/npm/node-semver) — GitHub, npm/node-semver. The canonical range engine (caret/tilde/comparator semantics) SpecForge's peer-dependency checks should grow toward.
- **package-lock.json (npm 5)** — npm blog, 2017. The deterministic-install argument for committing lockfiles: exactly why SpecForge refreshes and stores LockFileEntry hashes per project.
- [Interview with Isaac Z. Schlueter, CEO of npm](https://increment.com) — Increment magazine. Firsthand reasoning about registry growth, trust, and keeping the client thin.
- [isaacs](https://github.com/isaacs) — GitHub. glob, rimraf, node-tar, tap — the supporting library discipline behind npm's client.

## Study first
1. package-lock.json design: what is pinned (version, resolved URL, integrity) vs what is resolved
2. node-semver range grammar vs lock_file.rs's current tuple comparison
3. npm registry endpoint split: metadata (mutable) vs tarball (immutable, content-addressable)
