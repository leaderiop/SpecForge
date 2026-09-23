# 036 — Austin Wright

**Cluster:** C6 — Schemas, validation & serialization (Graph Protocol)
**Roster role:** Original-generation JSON Schema author; revival-era spec editor (drafts 05–07)
**SpecForge anchors:** `schema/specforge.schema.json`, `schema/specforge-report.schema.json`, `schema/specforge-binary-report.schema.json` (all declare draft 2020-12); `specforge schema --publish` (`crates/specforge-cli/src/export.rs` → `specforge-emitter::publish_json_schema`)

## Why this engineer
Wright picked up the stalled JSON Schema effort and rebuilt it into a real specification process: he introduced the `application/schema+json` media type, re-launched the IETF draft lineage that still numbers releases today, and edited the drafts (05–07, with Henry Andrews) that made tooling vendors invest. SpecForge emits and publishes JSON Schema draft documents as its public contract, so the durability of the format — stable `$schema` addressing, `$id`-based identity — is directly Wright's legacy. His editorial discipline is the template for keeping `https://specforge.dev/schema/*.json` a trustworthy, citable contract rather than a moving target.

## References for SpecForge
**Key works**
- [draft-wright-json-schema-00](https://datatracker.ietf.org/doc/html/draft-wright-json-schema-00) — IETF Internet-Draft, 2017 (editor). Introduced `application/schema+json`; the document that re-founded the spec process SpecForge's published schemas ride on.
- [JSON Schema Specification Links (draft 05–07 lineage)](https://json-schema.org/specification-links) — json-schema.org. Canonical map of which draft Wright edited and how meta-schemas are addressed — mirrors SpecForge's `$schema`/`$id` practice.
- [json-schema-org/json-schema-spec](https://github.com/json-schema-org/json-schema-spec) — GitHub. The spec repository model: issues, drafts, editorial process — a governance pattern worth copying for the Graph Protocol.
- [awwright](https://github.com/awwright) — GitHub. Author profile and related work.

## Study first
1. draft-wright-json-schema-00: why a media type (not just a file format) defines an ecosystem contract
2. Draft 05→07 evolution notes: what a "cleanup" release changes vs. breaks
3. `$id` base-URI addressing — the mechanism behind `https://specforge.dev/schema/*.json`
