# 040 — Kenton Varda

**Cluster:** C6 — Schemas, validation & serialization (Graph Protocol)
**Roster role:** Protocol Buffers tech lead; Cap'n Proto creator; zero-copy schema evolution
**SpecForge anchors:** schema evolution (`SchemaVersion`, `negotiate_version`, `SchemaMigrationChange` in `crates/specforge-emitter`); token-efficient binary exports (`schema/specforge-binary-report.schema.json`); compact wasm payloads (`crates/specforge-extism`, postcard via extism)

## Why this engineer
Varda led Protocol Buffers at Google, then built Cap'n Proto on the insight that a wire format should be pointer arithmetic over an in-place layout — no parse/serialize step, cheap decode, and evolution by field-number discipline that has kept a decade-old format compatible. SpecForge inherits both of his problems: emitted schemas must evolve without breaking downstream consumers (that is exactly what `schema_version` negotiation and `SchemaMigrationChange` encode), and the report/binary paths — binary test reports, Wasm-boundary payloads — need compact encodings where copy and byte cost dominate. His career is the case study in designing formats for 10-year consumers.

## References for SpecForge
**Key works**
- [Cap'n Proto](https://capnproto.org) — capnproto.org, 2013–present (creator). Zero-copy layout, "infinity times faster than Protocol Buffers": the reference for cheap-decode binary reports.
- [capnproto/capnproto](https://github.com/capnproto/capnproto) — GitHub. Canonical implementation; schema-compiler-driven codegen for multiple languages.
- "Cap'n Web: A new RPC system for browsers and web servers" — Cloudflare blog, September 2025. Same author re-targeting the protocol design for the web stack — evidence the format family still evolves.
- "Protocol Buffers with Kenton Varda" — Software Engineering Daily, December 2017. First-hand proto3 rationale: why `required` died and field-number evolution discipline.

## Study first
1. Field-number evolution rules (protobuf/Cap'n Proto) — the strictest form of `schema_version` policy
2. Cap'n Proto's zero-copy message layout vs. postcard-style varint streams for `specforge-binary-report`
3. Cap'n Web: adapting binary-era protocol design to token/JSON-shaped worlds
