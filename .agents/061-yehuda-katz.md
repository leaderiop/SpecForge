# 061 — Yehuda Katz

**Cluster:** C8 — Registries, packaging & supply-chain trust
**Roster role:** Bundler creator; Cargo dependency-resolution design
**SpecForge anchors:** extension peer-dependency resolution (peer_dependencies in extensions/{software,product,governance,formal}/src/handshake.json), lockfile resolution and load-order checks (crates/specforge-wasm/src/lock_file.rs), docs/extension-inventory.md dependency graph

## Why this engineer
Katz and Carl Lerche invented deterministic lockfile-driven resolution twice — Bundler for Ruby, then Cargo for Rust — and it is SpecForge's model exactly: extensions declare peer_dependencies (@specforge/governance and @specforge/formal require @specforge/software; @specforge/software requires @specforge/product), a lockfile pins the resolved set, and load order must follow the resulting DAG. When SpecForge's resolution grows from today's simple semver-like peer checks in lock_file.rs into real version selection, conflict reporting, and diamond handling, Bundler/Cargo are the designs to copy.

## References for SpecForge
**Key works**
- [Bundler](https://github.com/rubygems/bundler) — GitHub, rubygems/bundler, 2009 (now merged into rubygems/rubygems). The origin of "one resolved dependency graph, frozen in a lockfile, identical for everyone."
- [Cargo: Dependency Resolution](https://doc.rust-lang.org/cargo/reference/resolver.html) — The Cargo Book. The matured form of his design: version unification, lockfile priority, SemVer-compatible backtracking — the target semantics for SpecForge extension resolution.
- [wycats](https://github.com/wycats) — GitHub. Handlebars, Ember.js, Tilde — the engineering career of making convention-heavy tooling feel inevitable.
- [carlhuda](https://github.com/carlhuda) — GitHub. The Katz/Lerche pairing account under which Bundler and early Cargo (2012) were built — the primary-source repo trail of the resolver design.

## Study first
1. Bundler's resolution philosophy: resolve once, lock, reproduce everywhere
2. Cargo Book resolver chapter: unification, backtracking, lockfile precedence
3. SpecForge's peer-dependency DAG (extension-inventory.md) vs what Bundler would do with optional peers and conflicts
