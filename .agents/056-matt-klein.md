# 056 — Matt Klein

**Cluster:** C7 — Wasm plugin runtimes & the extension bet
**Roster role:** Envoy creator — filter/extension architecture at scale
**SpecForge anchors:** docs/extension-protocol.md (11 describe categories), crates/specforge-wasm/src/protocol/mod.rs (SUPPORTED_CATEGORIES, contribution_flags gating), crates/specforge-registry (Kind/Field/EdgeRegistry population)

## Why this engineer
Klein built Envoy around a fixed, versioned core with a small enumerated set of extension points (network/HTTP filters): extensions are statically linked but configured only at runtime, so the core ships and evolves without churn and every extension plugs into a reviewed surface. SpecForge's extension bet is the same pattern in compiler-land: 11 enumerated describe categories, per-category contribution flags deciding which `__describe` calls happen, and declarative registries that populate from descriptors. Klein's essays on operating this architecture at scale — compat policy, API review, hot restarts — are the governance manual for growing the 11 categories without breaking installed extensions.

## References for SpecForge
**Key works**
- [envoyproxy/envoy](https://github.com/envoyproxy/envoy) — GitHub, 2016. Filter-chain architecture and extension registration at fleet scale — the pattern behind contribution_flags + SUPPORTED_CATEGORIES.
- [Extending Envoy](https://www.envoyproxy.io/docs/envoy/latest/extending/extending) — Envoy docs. Codified rules for adding extension points and the compat bar they must clear — a mirror for evolving the 11 categories.
- [mattklein123.dev](https://mattklein123.dev/) — engineering blog, 2016+. Essays on Envoy's design and operation ("universal data plane", hot restart) — operational posture for specforge-watch and protocol changes.
- Q&A with Matt Klein on Creating Envoy at Lyft — InfoQ, 2017. The origin decisions: one core, extension surface over forking.

## Study first
1. Envoy filter chains & extension registration vs describe categories + flags
2. Envoy's extension-point governance (review, deprecation, compat) for the 11-category surface
3. Hot-restart/drain lessons → incremental invalidation in specforge-watch
