# 033 — Peter Chen

**Cluster:** C5 — Graph engine & algorithms
**Roster role:** the Entity-Relationship model (1976)
**SpecForge anchors:** crates/specforge-emitter/src/model/ ERD renderers — mermaid.rs (`render_mermaid` → `erDiagram`), dbml.rs, dot.rs, markdown.rs, json.rs; `ModelIntermediate` IR (model/mod.rs) built by ModelIntermediate_from_schema (model/build.rs); cardinality.rs `infer_cardinality` (1:1, 1:N, N:1, N:M)

## Why this engineer
Chen's 1976 model is the direct ancestor of SpecForge's `model` command: entities with attributes and typed relationships, cardinality annotations, and the claim that one conceptual schema can be rendered into many notations. `ModelIntermediate` is exactly Chen's ER schema as a Rust struct — entity kinds as entity sets, edge-derived relationships with inferred 1:1/N:M cardinalities — and the five renderers (markdown/mermaid/dot/json/dbml) demonstrate his core thesis that notation is a view, not the model. Chen's taste for unifying views of data also matches SpecForge's ERD-as-context-for-agents goal.

## References for SpecForge
**Key works**
- [The Entity-Relationship Model — Toward a Unified View of Data](https://dl.acm.org/doi/10.1145/320434.320440) — ACM Transactions on Database Systems 1(1):9–36, 1976. The founding paper: entity sets, relationships, cardinality, and diagrammatic notation — the spec ModelIntermediate implements.
- **Entity-Relationship Modeling: Historical Events, Future Trends, and Lessons Learned** — in Software Pioneers: Contributions to Software Engineering (Broy & Denert, eds.), Springer, 2002. Chen's own retrospective on why ER survived: minimal concepts, tool-renderable notation.
- **English Sentence Structure and Entity-Relationship Diagrams** — Information Sciences 29, 1983. Mapping natural-language statements onto ER constructs — the pattern for turning .spec prose contracts into entity/relationship diagrams.

## Study first
1. The 1976 paper, §1–4: entity sets, relationships, cardinality vs ModelIntermediate
2. Notation independence: one schema → many renderers (mermaid/dbml/dot)
3. The 1983 English-mapping paper for prose→ER extraction ideas
