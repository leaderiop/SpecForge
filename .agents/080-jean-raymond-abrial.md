# 080 — Jean-Raymond Abrial

**Cluster:** C10 — Formal methods (@specforge/formal + analyze passes)
**Roster role:** Z & B-Method / Event-B; refinement-driven specification
**SpecForge anchors:** RES-25 (B-Method row), refinement entities (RefinesTo/RefinementChainLink, E041), capability→feature→behavior→test chain

## Why this engineer
Abrial co-invented Z, then B and Event-B — the methods RES-25 names as SpecForge's refinement lens, mapping the `capability → feature → behavior → test` chain onto B's abstract-model-then-refine discipline. SpecForge's refinement entities (RefinesTo/RefinementChainLink edges, E041 chain-cycle check) are Event-B's refinement DAG with graph validation standing in for proof obligations. His insistence that refinement steps stay small and each carry its own obligation is the design rationale for layering_verify and strict-level formal warnings.

## References for SpecForge
**Key works**
- [Modeling in Event-B: System and Software Engineering](https://www.cambridge.org/9780521895569) — Cambridge University Press, 2010. The definitive Event-B text: machines, events, invariants, and stepwise refinement — the blueprint behind the refinement entity set.
- **The B-Book: Assigning Programs to Meanings** — Cambridge University Press, 1996. B's formal core, incl. substitution calculus and proof obligation generation — prior art for turning condition checks into machine-checkable obligations.
- [Event-B and the Rodin Platform](https://www.event-b.org) — canonical Event-B site. Rodin's incremental obligation discharge shows how an IDE (specforge-lsp) and a checker share one model.
- **A Scientific Biography of a Formal Methods Pioneer** (J.P. Bowen, IEEE Annals) — 2026. Historical survey situating Z→B→Event-B; useful for RES-25-style method comparisons.

## Study first
1. Modeling in Event-B ch. 1–4; map events/invariants onto event/invariant entities
2. Refinement proof obligations — what W069–W071 refinement validation could grow into
3. Rodin workflow — incremental revalidation to mirror specforge-watch
