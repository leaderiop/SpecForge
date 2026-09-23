# 096 — Bruno Oliveira

**Cluster:** C11 — Testing & BDD (verify statements, collect, report protocol)
**Roster role:** pytest lead maintainer; plugin ecosystem steward
**SpecForge anchors:** @specforge/pytest adapter — production-grade hook implementation, plugin packaging via pytest11 entry point, pytester-based testing of the plugin itself

## Why this engineer
Oliveira (@nicoddemus) has been pytest's core maintainer since 2014 through its plugin-ecosystem boom, stewarding core hooks while authoring pytest-mock — a model third-party plugin. An @specforge/pytest adapter must survive pytest releases, coexist with unrelated plugins loaded by entry point, and avoid assertion-rewriting traps; these are exactly the failure modes his maintenance work polices. He is the reference for what separates a toy hook script from a plugin that ships.

## References for SpecForge
**Key works**
- [pytest-dev/pytest-mock](https://github.com/pytest-dev/pytest-mock) — GitHub. His flagship plugin: the packaging and layout template for @specforge/pytest (entry point, module split, changelog discipline).
- [Writing plugins](https://docs.pytest.org/en/stable/how-to/writing_plugins.html) — pytest official docs. Plugin discovery order, `pytest_plugins`, assertion rewriting — the sections where adapters break.
- **pytest Quick Start Guide** — Packt, 2018. His practical mental model of fixtures and markers — the annotation surface a SpecForge adapter reads to link tests to entities.
- [pytest-dev/pytest](https://github.com/pytest-dev/pytest) — GitHub. The core he maintains; hook guarantees the adapter can rely on across releases.

## Study first
1. pytest-mock's source layout and release process
2. Assertion rewriting + plugin registration caveats from the plugin guide
3. Testing plugins with pytester — how the adapter's own tests should run
