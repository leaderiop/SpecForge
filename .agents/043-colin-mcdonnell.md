# 043 — Colin McDonnell

**Cluster:** C6 — Schemas, validation & serialization (Graph Protocol)
**Roster role:** Zod creator; TypeScript schema validation & developer experience
**SpecForge anchors:** TS-side schema consumption of Graph Protocol exports; `integrations/vscode/schemas/specforge.schema.json` (config schema wired into editor validation); complement to `specforge-lsp` for non-.spec TS surfaces

## Why this engineer
McDonnell built Zod, the default TypeScript runtime-validation library, on the schema-first/parse-don't-validate pattern: declare a schema, validate untrusted data at the boundary, infer the static type from the same declaration. That is exactly how TS consumers should ingest everything SpecForge emits — exported Graph Protocol JSON, `specforge-report.json`, and the specforge config file whose schema ships in `integrations/vscode/schemas`. Zod 4's JSON Schema conversion and its smaller, tree-shakable core show what a modern validation surface owes its host language's tooling — relevant wherever SpecForge meets the VSCode extension and JS/TS ecosystem.

## References for SpecForge
**Key works**
- [colinhacks/zod](https://github.com/colinhacks/zod) — GitHub, 2020–present. The canonical TS validation library; study its API ergonomics and inference-from-schema design.
- [zod.dev](https://zod.dev) — Official docs. `z.toJSONSchema()` (Zod 4) for round-tripping Graph Protocol JSON ↔ Zod schemas in consumers.
- "Joining Clerk as an OSS Fellow to work on Zod 4" — zod.dev blog, June 2024. How major validation releases get funded and scoped.
- "Colin McDonnell Talks About The Design Choices Behind Zod" — Total TypeScript, 2023. Creator's rationale for schema-first validation in TS.
- [colinhacks.com](https://colinhacks.com) — Personal site and related projects.

## Study first
1. `z.toJSONSchema()` — converting runtime schemas to draft 2020-12 and back
2. Parse-don't-validate: typed boundaries around emitted artifacts in TS consumers
3. What editor UX (autocomplete from `integrations/vscode/schemas`) does for config adoption
