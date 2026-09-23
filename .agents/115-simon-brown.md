# 115 — Simon Brown

**Cluster:** C13 — Diagrams & rendering
**Roster role:** C4 model creator; Structurizr; diagrams-as-code with a validated model
**SpecForge anchors:** model command (crates/specforge-cli/src/model.rs), renderer extension mechanism (ModelFormat dispatch in crates/specforge-emitter/src/model/mod.rs), ModelIntermediate view layer

## Why this engineer
Brown's core argument — don't draw pictures; build a model and *extract* views from it — is SpecForge's model command realized: one resolved, validated graph from which markdown/mermaid/dot/json/dbml views are rendered, so diagrams cannot drift from the spec they depict. His C4 hierarchy (context → containers → components → code) is the natural roadmap for SpecForge's renderer extension mechanism: zoom levels and filtered views over ModelIntermediate rather than hand-authored diagrams, with Structurizr's DSL `views` block as prior art for view-as-code.

## References for SpecForge
**Key works**
- [c4model.com](https://c4model.com) — C4 model reference: levels, notation, and the diagrams-as-code argument.
- **Software Architecture for Developers, vols 1–2** — Leanpub, 2013 (v1) / 2018–2020 (v2). The visualising-software-architecture chapters diagnose diagram-vs-model drift.
- [Structurizr](https://structurizr.com) — diagrams-as-code platform built on model-extracted views.
- [structurizr/java](https://github.com/structurizr/java) — GitHub. Core model + DSL implementation; view definitions separated from the model.
- [structurizr/lite](https://github.com/structurizr/lite) — GitHub. Minimal-footprint deployment of the same model/view pipeline.

## Study first

1. C4 levels as filtered views over one model
2. Structurizr DSL: model block vs views block separation
3. "Diagrams as code 1.0 vs 2.0": why text diagrams still drift, and extracting views from a live model
