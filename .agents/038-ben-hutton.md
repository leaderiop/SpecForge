# 038 — Ben Hutton

**Cluster:** C6 — Schemas, validation & serialization (Graph Protocol)
**Roster role:** JSON Schema 2020-12 spec lead; open-standard stewardship & validator compliance
**SpecForge anchors:** Graph Protocol as open standard (`$id: https://specforge.dev/schema/*.json`); `specforge schema` command (`--publish` standalone draft 2020-12); `crates/specforge-validator` (orphan/file-ref checks against emitted schema)

## Why this engineer
Hutton led the 2020-12 release, formalized bundling after years of `$ref` folklore, and then did what most spec editors never do: built the ecosystem apparatus — cross-implementation compliance testing (Bowtie), learning material (Learn JSON Schema), and community process — that turns a document into an adopted standard. SpecForge's moat thesis bets the Graph Protocol becomes an open standard third parties validate against and build UIs for. Hutton's playbook is the exact roadmap: publish stable schemas under fixed `$id`s (`specforge schema --publish` already emits draft 2020-12), then prove compliance and teach consumers.

## References for SpecForge
**Key works**
- [Draft 2020-12 meta-schema](https://json-schema.org/draft/2020-12/schema) — IETF/json-schema.org, 2020–2022 (editor, draft-bhutton-*). The dialect every SpecForge schema explicitly targets.
- [Learn JSON Schema](https://www.learnjsonschema.com) — site/book. Model for Graph Protocol learning material and per-dialect documentation.
- [Bowtie — JSON Schema CLI compliance harness](https://github.com/bowtie-json-schema/bowtie) — GitHub. Template for running specforge-validator against official test suites instead of ad-hoc fixtures.
- "JSON Schema Bundling Finally Formalised" — apisyouwonthate.com, 2021. Why `$id`/`$ref` bundling rules matter for multi-file exports.
- [Relequestual](https://github.com/Relequestual) — GitHub. Spec work, website, and community tooling.

## Study first
1. 2020-12 bundling + `$dynamicRef` — handling cross-file references in Graph Protocol subgraphs
2. Bowtie's harness design — pluggable adapters, normalized test results
3. What "specification lead as community builder" changed about adoption curves
