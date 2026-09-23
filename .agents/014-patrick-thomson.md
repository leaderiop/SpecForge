# 014 — Patrick Thomson

**Cluster:** C3 — Parsing & grammar infrastructure
**Roster role:** stack-graphs creator at GitHub — precise, incremental name resolution
**SpecForge anchors:** crates/specforge-resolver/src/linker.rs (link_references → PendingEdge, E003 diagnostics); crates/specforge-lsp/src/navigation.rs (find_all_references)

## Why this engineer
link_references currently resolves `.spec` references by exact name matching across indexed files — flat, whole-string, no scope awareness. Stack graphs, which Thomson designed and led at GitHub, formalizes precisely this problem: declarative name-binding rules over a tree-sitter CST, resolved by path-finding that captures scope, imports, and capture rules, computed incrementally with zero per-repo configuration. As `.spec` grows scoped imports or kind-aware references, linker.rs will need machinery of exactly this shape — and the rules-not-code style matches SpecForge's declarative validation_engine. His work also models shipping research-grade name resolution as a product (GitHub Precise Code Navigation).

## References for SpecForge
**Key works**
- [Stack graphs: name resolution at scale](https://arxiv.org/abs/2211.01224) — Creager, Thomson, Hynes, Martin; EVCS (OASIcs), 2023. The formal treatment: name-binding path semantics, soundness, incrementalization guarantees.
- [github/stack-graphs](https://github.com/github/stack-graphs) — GitHub, 2020–present. Rust implementation plus tree-sitter query extraction — direct prior art for linker.rs.
- [Introducing stack graphs](https://github.blog/open-source/introducing-stack-graphs) — GitHub Blog, 2021. Accessible origin story: why heuristic code nav broke and graphs fixed it.
- [Incremental, zero-config code nav using stack graphs](https://www.thestrangeloop.com/2021/incremental-zero-config-code-nav-using-stack-graphs.html) — Strange Loop 2021 (talk). The rule model and incremental computation, narrated end to end.

## Study first
1. The rules model of exported/imported scopes: how cross-file references become graph paths — compare with PendingEdge in linker.rs
2. Partial paths and cycle safety: what guarantees resolution terminates — contrast with specforge-graph's own cycle detection
3. tree-sitter query extraction of binding rules: a zero-core-change path toward scope-aware E003 diagnostics in find_all_references
