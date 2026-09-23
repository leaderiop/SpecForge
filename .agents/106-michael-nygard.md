# 106 — Michael Nygard

**Cluster:** C12 — Requirements engineering, ubiquitous language & docs-as-code
**Roster role:** Architecture Decision Records; production-stability patterns
**SpecForge anchors:** `decision` entities in `spec/governance/decisions.spec` (status: accepted, date, context, decision, consequences) + per-extension decision dirs under `spec/extensions/`; `failure-modes.spec` severity/occurrence/detection discipline

## Why this engineer
Nygard's 2011 post defined the ADR: a short, numbered, version-controlled document with Title, Context (value-neutral forces), Decision, Status, Consequences — so future developers see *why*, and reversed decisions are marked superseded, never deleted. SpecForge's `decision` entity is this format promoted into the typed graph: status is a validated lifecycle field, consequences are structured lists, and extensions keep their own ADRs — exactly Nygard's "small, modular documents kept up to date." His Release It! stability vocabulary (fail fast, circuit breakers, bulkheads) is also the natural language for `failure-modes.spec` mitigation narratives.

## References for SpecForge
**Key works**
- [Documenting Architecture Decisions](https://cognitect.com/blog/2011/11/15/documenting-architecture-decisions) — blog post, November 15, 2011. The format definition SpecForge's decision entity implements nearly field-for-field (proposed/accepted/superseded lifecycle included).
- **Release It!: Design and Deploy Production-Ready Software**, 2nd edition — Pragmatic Bookshelf, 2018. Stability and capacity patterns; the vocabulary for failure_mode cause/effect/mitigation entries.
- [adr.github.io](https://adr.github.io/) — community hub. The ecosystem of ADR tooling and templates — precedent for rendering decisions to markdown/mermaid docs from the graph.

## Study first
1. The 2011 post — compare each prescribed section to the `decision` block's fields
2. Status lifecycle handling: superseded references, numbers never reused — validator candidates
3. Release It! pattern catalog as a source of failure_mode templates for extensions
