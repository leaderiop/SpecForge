# 090 — Kent Beck

**Cluster:** C11 — Testing & BDD (verify statements, collect, report protocol)
**Roster role:** xUnit & TDD creator
**SpecForge anchors:** `verify` statements in the .spec DSL (unit/integration/property/e2e kinds, auto/manual modes); integrations/rust/specforge-test harness (guard, registry, report)

## Why this engineer
Beck invented the practice of pinning intended behavior with small automated tests: SUnit for Smalltalk, then JUnit with Erich Gamma, seeding the entire xUnit family every modern runner still follows. SpecForge's `verify` statements are exactly his test-first discipline lifted into the spec graph — each behavior declares obligations before any test exists. The specforge-test harness turns those declarations into an executable loop, which is Beck's red/green cycle applied to specifications instead of code.

## References for SpecForge
**Key works**
- **Test-Driven Development: By Example** — Addison-Wesley, 2002. The canonical red/green/refactor workflow the verify-then-collect pipeline should preserve.
- **Simple Smalltalk Testing: With Patterns** — Journal of Object-Oriented Programming, 1994. Origin of TestCase/fixture/suite vocabulary — the shape every report entry still echoes.
- **Extreme Programming Explained** — Addison-Wesley, 1999. Tests as specification and communication, the philosophical basis for machine-checkable obligations.
- **Tidy First?** — O'Reilly, 2023. Separating structural from behavioral change; keeps the 219-file self-spec corpus refactorable under test.
- [junit-team/junit5](https://github.com/junit-team/junit5) — GitHub. Direct descendant of Beck/Gamma's JUnit; its lifecycle is what collect adapters must observe.
- [xunit/xunit](https://github.com/xunit/xunit) — GitHub. The xUnit.net branch proving the pattern ports across languages — the portability argument behind a language-agnostic report schema.

## Study first
1. Test-Driven Development: By Example — the whole method in one thin book
2. Simple Smalltalk Testing: With Patterns — where the report schema's vocabulary began
3. JUnit 5 extension/lifecycle model — what a collect adapter actually sees
