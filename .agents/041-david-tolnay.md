# 041 — David Tolnay

**Cluster:** C6 — Schemas, validation & serialization (Graph Protocol)
**Roster role:** serde lead maintainer; thiserror/anyhow; Rust library ergonomics
**SpecForge anchors:** serde derive models across all crates (`GraphProtocolSchema` in `crates/specforge-emitter/src/schema.rs`, Kind/Field/Edge registries, report types in `integrations/rust/specforge-test`); `thiserror`/`anyhow` as direct deps of `extensions/{software,product,governance,formal}`; `serde_json` workspace-wide

## Why this engineer
Every JSON artifact SpecForge produces — Graph Protocol exports, registry manifests, test reports — exists because a struct somewhere derives `Serialize`/`Deserialize`, and serde is Tolnay's crate (lead maintainer since 2016). His error-model pair is equally load-bearing: the four builtin Wasm extension crates use thiserror for typed library errors and anyhow for application-level context. Beyond code, his crate-design discipline — minimal APIs, zero-cost derives, published soundness essays — is the quality bar for keeping SpecForge's serialization layer boring, fast, and correct rather than clever.

## References for SpecForge
**Key works**
- [serde-rs/serde](https://github.com/serde-rs/serde) — GitHub, 2014–present. The derive/visitor architecture behind every SpecForge model; study the data model before touching emitter formats.
- [Serde data model reference](https://serde.rs/data-model.html) — serde.rs. The 29-type contract that decides how `GraphProtocolSchema` maps to and from JSON.
- [dtolnay/thiserror](https://github.com/dtolnay/thiserror) — GitHub. Typed error derives — the pattern for extension-crate error enums.
- [dtolnay/anyhow](https://github.com/dtolnay/anyhow) — GitHub. Contextual application errors — the CLI/pipeline error path.
- [Essays by David Tolnay](https://docs.rs/dtolnay) — docs.rs. Soundness and API-design essays; the taste document for Rust library work.

## Study first
1. The serde data model: what `serialize_map` vs `serialize_struct` means for stable emitted field names
2. `#[serde(rename_all, skip_serializing_if)]` discipline for wire-stable Graph Protocol JSON
3. thiserror-vs-anyhow split — where library errors end and application errors begin
