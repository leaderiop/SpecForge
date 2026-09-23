# 053 — Matt Butcher

**Cluster:** C7 — Wasm plugin runtimes & the extension bet
**Roster role:** Fermyon Spin creator — Wasm app ergonomics & warm scheduling
**SpecForge anchors:** crates/specforge-wasm/src/engine_pool.rs (WarmEngineConfig, LRU warm pool shared via Arc for MCP handlers), docs/extension-sdk.md (SDK macros → protocol categories), crates/specforge-wasm/src/lock_file.rs

## Why this engineer
Butcher co-created Helm, co-founded Deis, and then built Spin: a micro-framework where Wasm components react to triggers (HTTP, timers, events) so writing server-side Wasm feels like writing a function. Spin's founding argument — "life after cold starts", warm instances and sub-millisecond startup — is what SpecForge's EnginePool implements for long-lived MCP/watch handlers while the CLI pays per-invocation compile cost. His ergonomics-first manifests (spin.toml) and developer-facing conventions are the benchmark for SpecForge's manifest v2 + SDK macro surface.

## References for SpecForge
**Key works**
- [spinframework/spin](https://github.com/spinframework/spin) — GitHub, 2022. Trigger + component model and manifest ergonomics; the closest product analogue to SpecForge's extension/app framing.
- [Helm](https://helm.sh) — Kubernetes/Helm, 2016. Packaging declarative artifacts with releases and locks — the precedent behind SpecForge's registry specifier and lock file discipline.
- Kubernetes: Up and Running — O'Reilly, 2017 (with Hightower & Burns). The canonicalization playbook: turn an orchestration concept into the default developer vocabulary.
- Ep. #125: Life After Cold Starts — Heavybit podcast, 2023. The warm-start/wasm-startup argument that motivates engine pooling.

## Study first
1. Spin's trigger/component split vs SpecForge's describe categories and host exports
2. Warm-instance scheduling arguments → validate EnginePool's LRU + memory caps
3. Helm's release/lock discipline vs lock_file.rs and extension versioning
