# 062 — D. Richard Hipp

**Cluster:** C8 — Registries, packaging & supply-chain trust
**Roster role:** SQLite creator — embedded storage engine
**SpecForge anchors:** registry-server SQLite storage (crates/specforge-registry-server/src/db.rs: rusqlite, tokens/package metadata; storage.rs artifact files)

## Why this engineer
SpecForge's registry-server made the Hipp bet: one SQLite file, embedded via rusqlite behind a mutex-guarded connection, holding packages and auth tokens with zero administration (db.rs). SQLite is the most deployed database engine on earth precisely because of the properties a small registry needs — single-file durability, no daemon, a file format stable across decades and readable by future tooling. His discipline (a file format treated as a permanent public contract, and the most obsessively tested codebase in the industry) is the standard specforge-registry-server's storage layer should be held to.

## References for SpecForge
**Key works**
- [SQLite](https://www.sqlite.org) — sqlite.org, 2000–present. The canonical single-file, public-domain embedded database; the storage engine already under specforge-registry-server.
- **SQLite: Past, Present, and Future** — Proc. VLDB Endow. 15(12), 2022, pp. 3535–3547 (Gaffney, Prammer, Brasfield, Hipp, Kennedy, Patel). Why SQLite's design choices (B-tree per table, single writer) hold up at modern scale — the honest performance envelope of db.rs.
- [How SQLite Is Tested](https://www.sqlite.org/testing.html) — sqlite.org. 100% branch/MC-DC coverage, fuzzing, fault injection — the gold standard for validating a storage layer that registry data depends on.
- [Fossil](https://www.fossil-scm.org) — fossil-scm.org, 2007. His SCM built directly on SQLite — a worked example of using SQLite as the backbone of stateful infrastructure, as registry-server does.
- [SQLite Database File Format](https://www.sqlite.org/fileformat2.html) — sqlite.org. The documentation rigor of a format designed to outlive its authors.

## Study first
1. sqlite.org/fileformat2.html — what "stable file format" means in practice
2. How SQLite Is Tested — coverage/fault-injection methodology transferable to db.rs/storage.rs
3. VLDB 2022 paper — single-writer trade-offs that will cap registry-server write throughput
