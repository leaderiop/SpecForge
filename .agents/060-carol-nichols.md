# 060 — Carol Nichols

**Cluster:** C8 — Registries, packaging & supply-chain trust
**Roster role:** crates.io co-builder — registry storage/index design at scale
**SpecForge anchors:** specforge-registry-server storage/index design (crates/specforge-registry-server/src/storage.rs LocalStorage, db.rs, handlers.rs)

## Why this engineer
Nichols helped carry crates.io from a side project into the default infrastructure of an entire language ecosystem — including the decisive architectural split between an immutable artifact store and a small, fast, version-index that clients can query without downloading artifacts. SpecForge's registry-server stands at the same fork in the road: LocalStorage already writes immutable `{package}/{version}.wasm` blobs (storage.rs) beside a SQLite metadata DB (db.rs) with server-side SHA-256 (handlers.rs); her index/storage separation and yanking semantics are the blueprint for evolving it past its current shape.

## References for SpecForge
**Key works**
- [crates.io](https://github.com/rust-lang/crates.io) — GitHub, rust-lang/crates.io. Production registry codebase: index endpoints, artifact storage, ownership/yank policy — the closest production analogue to specforge-registry-server.
- [The Rust Programming Language](https://doc.rust-lang.org/book/) — No Starch Press / doc.rust-lang.org, 2015–present. Co-authored with Steve Klabnik; her chapter-level care for beginners models how specforge CLI help text should read.
- [Cargo and Crates.io with Carol (Nichols \|\| Goulding)](https://integer32.com) — The Manifest podcast (manifest.fm), 2017. Firsthand account of maintaining both resolver and registry — the client/server split SpecForge now has between specforge-wasm and registry-server.
- [rust-lang/book](https://github.com/rust-lang/book) — GitHub. The book's repo — an example of docs maintained as first-class project infrastructure.

## Study first
1. crates.io index-vs-storage split: why the index stayed small and clients never fetch artifacts to resolve
2. yank semantics: remove from index, never from disk
3. crates.io's migration history (git index → sparse index) as a lesson in evolving registry protocols without breaking clients
