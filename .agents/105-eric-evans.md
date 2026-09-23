# 105 — Eric Evans

**Cluster:** C12 — Requirements engineering, ubiquitous language & docs-as-code
**Roster role:** Domain-Driven Design; ubiquitous language
**SpecForge anchors:** `term` entities in `spec/glossary.spec` + `spec/product/terms.spec` (definition, aliases, see_also); zero-domain-knowledge core with vocabulary contributed by extensions in `extensions/{software,product,governance,formal}`

## Why this engineer
Evans' thesis — the domain model and its language must live in one bounded, rigorously used vocabulary, refined continuously with domain experts — is SpecForge's founding mechanism: the glossary is a first-class singleton entity in the graph, terms are linkable (`see_also`, alias resolution), and every extension must introduce its vocabulary through declared terms rather than tribal prose. His bounded-context idea explains the architecture: the core knows no domain words; each extension (software, product, governance, formal) is a bounded context whose language joins the graph at handshake.

## References for SpecForge
**Key works**
- **Domain-Driven Design: Tackling Complexity in the Heart of Software** — Addison-Wesley, 2003. The blue book: ubiquitous language, bounded contexts, model-driven design — the rationale for glossary-as-entity and extension vocabularies.
- [DDD Reference](https://www.domainlanguage.com/ddd/reference/) — Domain Language, free PDF, 2015. One-page definitions of every pattern; the strict-sense vocabulary SpecForge glossary terms should match when overlapping.
- **Introducing Domain-Driven Design workshops and talks** — Domain Language (domainlanguage.com), 2003-present. Evans' talks on refining language with experts model the glossary-authoring loop SpecForge wants in `specforge infer` and reviews.

## Study first
1. Ubiquitous Language + Bounded Context chapters — then audit cross-extension term collisions
2. The DDD Reference pattern map vs `term` entity fields (definition/aliases/see_also)
3. Supple design chapters — keep the DSL vocabulary as disciplined as the model vocabulary
