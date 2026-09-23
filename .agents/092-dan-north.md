# 092 — Dan North

**Cluster:** C11 — Testing & BDD (verify statements, collect, report protocol)
**Roster role:** Inventor of BDD; JBehave creator
**SpecForge anchors:** `behavior` entities with `contract` statements and `verify` obligations in the .spec DSL; self-spec corpus behaviors.spec

## Why this engineer
North created BDD in 2003 by reframing TDD's vocabulary — "behavior" instead of "test", sentences instead of method names — so that a test suite doubles as a requirements conversation. SpecForge's graph makes this literal: `behavior` is a first-class entity carrying a `contract` field plus machine-checkable `verify` obligations. North's core claim, that the wording of a spec determines who can read it, is exactly what the graph encodes when coverage maps a behavior's contract onto passing tests.

## References for SpecForge
**Key works**
- [Introducing BDD](https://dannorth.net/introducing-bdd/) — dannorth.net, 2006. The founding article: the vocabulary shift from test to behavior — the entity-naming decision SpecForge baked into its graph.
- [What's in a Story?](https://dannorth.net/whats-in-a-story/) — dannorth.net, 2007. User-story template becoming Given/When/Then acceptance criteria — direct ancestor of pairing a contract with verify statements on one behavior.
- **BDD is like TDD if…** — dannorth.net, 2012. The mature restatement: BDD as analysis technique, not test tooling — the framing SpecForge's zero-domain-knowledge core depends on.
- [jbehave/jbehave](https://github.com/jbehave/jbehave) — GitHub. The first BDD framework; the earliest binding of prose specification to executable validation.

## Study first
1. Introducing BDD — the vocabulary shift and its motivation
2. What's in a Story? — Given/When/Then as acceptance criteria
3. JBehave's story→steps binding — first attempt at spec-to-test linkage
