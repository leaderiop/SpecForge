# 016 — Chase Wilson (Kixiron)

**Cluster:** C3 — Parsing & grammar infrastructure
**Roster role:** lasso string-interner creator; compiler-infrastructure tooling in Rust
**SpecForge anchors:** crates/specforge-common/src/interner.rs — global ThreadedRodeo; Sym(Spur) with O(1) Copy/Eq/Hash and `&'static str` resolve

## Why this engineer
SpecForge interns every identifier, kind, and field name as `Sym(Spur)` in a lazily initialized `ThreadedRodeo` — interner.rs promises Copy/Eq/Hash in O(1) and resolution back to `&'static str`, and graph nodes, spans, and diagnostics all lean on it. That makes lasso, Wilson's library, a verified direct dependency on the compiler's hottest path: parse.rs and the graph builders hammer get_or_intern while the watch daemon and CLI run multi-threaded. Knowing ThreadedRodeo's sharded locking, Spur's numeric stability, and interner memory growth — topics Wilson documents and benchmarks himself — is prerequisite to tuning large-corpus runs. His wider portfolio (rust-langdev, an RVSDG optimizer, memory measurement) reads like a checklist for the registry and IR layers.

## References for SpecForge
**Key works**
- [lasso](https://github.com/Kixiron/lasso) — GitHub, Kixiron/lasso, 2020–present. The dependency itself: Rodeo vs ThreadedRodeo trade-offs, custom key and slicer types.
- [lasso on docs.rs](https://docs.rs/lasso) — API reference. Spur, get_or_intern/resolve contracts, capacity and memory behavior.
- [rust-langdev](https://github.com/Kixiron/rust-langdev) — GitHub. His curated "language development libraries for Rust" map — context for interner choice among peers.
- [size-of](https://github.com/Kixiron/size-of) — GitHub. Total-memory measurement tool — directly applicable to auditing the interner + petgraph footprint.
- [ddshow](https://github.com/Kixiron/ddshow) — GitHub. Visualization for timely/differential dataflow programs — a pattern for tracing specforge-test report runs.

## Study first
1. ThreadedRodeo's lock granularity: which paths serialize, which shard — why CLI threads don't fight over the global interner
2. Spur stability vs process lifetime: what Sym equality actually promises across repeated interning
3. Interner memory growth and reserve APIs: budgeting the 219-file self-spec corpus
