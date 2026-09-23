# 037 — Henry Andrews

**Cluster:** C6 — Schemas, validation & serialization (Graph Protocol)
**Roster role:** JSON Schema 2019-09 spec lead; vocabulary & modularity design
**SpecForge anchors:** `schema/*.json` (draft 2020-12); `SchemaVersion` / `negotiate_version` / `SchemaMigrationChange` in `crates/specforge-emitter` (schema versioning of every export)

## Why this engineer
Andrews co-edited draft-07 and then led 2019-09, the release that decomposed JSON Schema into explicit vocabularies (validation, applicator, meta-data, format, content) — turning a bag of keywords into composable dialects that tools can negotiate over. SpecForge has the same problem in miniature: the Graph Protocol schema is embedded in every export, consumers adopt capabilities at different paces, and `specforge export --schema-version` exists precisely because Andrews' negotiation model is correct. His `unevaluated*` applicator work also defines how composite documents stay validatable without double-validation bugs — relevant to subgraph-scoped exports.

## References for SpecForge
**Key works**
- [draft-handrews-json-schema-validation-02](https://datatracker.ietf.org/doc/html/draft-handrews-json-schema-validation-02) — IETF Internet-Draft, 2019 (editor). The vocabulary split that made "Modern JSON Schema" — the intellectual basis for versioned schema dialects.
- [A Media Type for Describing JSON Documents (draft-07 core)](https://json-schema.org/draft-07/draft-handrews-json-schema-01) — IETF, 2018. Co-edited with Austin Wright; the base dialect SpecForge schemas ultimately derive from.
- [What is Modern JSON Schema? (interview with Henry Andrews)](https://modern-json-schema.com) — 2022. His own framing of why vocabulary-based modularity matters to consumers.
- [handrews](https://github.com/handrews) — GitHub. Drafts, issue scholarship, and schema-tooling work.

## Study first
1. The 2019-09 vocabulary decomposition and what a "dialect" promises its consumer
2. `unevaluatedProperties`/`unevaluatedItems` semantics — applicators that reason about the whole document
3. Capability negotiation between producer and consumer — the pattern behind `negotiate_version`
