# 114 — Arnaud Roques

**Cluster:** C13 — Diagrams & rendering
**Roster role:** PlantUML creator; text-to-UML with swappable layout backends
**SpecForge anchors:** future renderer extensions (ModelFormat dispatch in crates/specforge-emitter/src/model/mod.rs), extension-declared rendering vocabulary

## Why this engineer
Roques is the counterfactual worth studying when SpecForge adds renderers: PlantUML kept one stable input language for 15+ years while swapping and adding rendering backends — originally delegating layout to Graphviz dot, then absorbing a pure-Java port (Smetana) and ELK so diagrams render with zero native dependencies. That is exactly the SpecForge scenario where `ModelFormat::Mermaid|Dot|Dbml|...` grows new variants without touching the resolved model. PlantUML also proves the ecosystem value of a text protocol (text → PNG/SVG) that editors and docs embed.

## References for SpecForge
**Key works**
- [PlantUML](https://plantuml.com) — official site, 2009–. Language reference and rendering server protocol.
- [plantuml/plantuml](https://github.com/plantuml/plantuml) — GitHub, Java implementation. Study the layout-backend seam (Graphviz vs Smetana vs ELK).
- [A coffee with Arnaud Roques (creator of PlantUML)](https://modeling-languages.com/interview-plantuml/) — modeling-languages.com, 2016. His account of design priorities: text-first, minimal syntax, layout delegated.
- **PlantUML Community Choice Project of the Month interview** — SourceForge, July 2019. Project history and evolution of backend support.

## Study first
1. Layout-backend seam: dot delegation vs Smetana/ELK fallbacks
2. Keeping input syntax stable while renderers multiply
3. Text→image server protocol (encoded text, no state) as a distribution model
