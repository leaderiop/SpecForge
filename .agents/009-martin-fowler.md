# 009 — Martin Fowler

**Cluster:** C2 — DSL & language design
**Roster role:** 'Domain-Specific Languages' author; evolutionary-design thinker
**SpecForge anchors:** .spec DSL evolution (`spec/research/RES-20-type-system-evolution.md`); format versioning (`crates/specforge-migrate`); extension-carried vocabulary (Kind/Field/Edge registries)

## Why this engineer
Fowler wrote the standard text on DSLs: internal vs external grammar, the semantic model as the real product, and the standing warning that DSLs accrete accidental complexity faster than value. SpecForge embodies his thesis — a minimal external grammar whose meaning lives in an extensible semantic model (the typed graph + registries) rather than in compiler special cases. His evolutionary-design stance matches SpecForge's roadmap, and the DSL book's versioning/migration guidance is precisely what specforge-migrate must operationalize as the .spec format ages.

## References for SpecForge
**Key works**
- [Domain-Specific Languages](https://martinfowler.com/dslwip/) — with Rebecca Parsons; Addison-Wesley, 2010. The canonical pattern catalog; its semantic-model separation is SpecForge's graph-before-grammar architecture.
- [Language Workbenches: The Killer-App for Domain Specific Languages?](https://martinfowler.com/articles/languageWorkbench.html) — martinfowler.com, 2005. Predicted IDE-driven DSL tooling; SpecForge's LSP-era realization of that argument.
- [DomainSpecificLanguage (bliki)](https://martinfowler.com/bliki/DomainSpecificLanguage.html) — martinfowler.com, 2005. The one-page test of when a DSL earns its keep — the filter for every proposed grammar addition.
- **Refactoring** — Addison-Wesley, 2nd ed, 2018. Behavior-preserving change discipline — the model for format migrations that must not alter graph semantics.

## Study first
1. DSL book Part 1 — semantic model pattern, grammar vs model split
2. Language Workbenches essay — DSL tooling evolution past and present
3. Versioning/migration guidance — the specforge-migrate contract
