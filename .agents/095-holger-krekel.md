# 095 — Holger Krekel

**Cluster:** C11 — Testing & BDD (verify statements, collect, report protocol)
**Roster role:** pytest creator
**SpecForge anchors:** @specforge/pytest adapter design — a pytest11 plugin observing runtest hooks and emitting specforge-report.json; report schema mirrors integrations/rust/specforge-test/src/report.rs

## Why this engineer
Krekel instigated pytest in 2003/04 and maintained it for a decade, building it on pluggy: a hook architecture where collection, execution, and reporting are separate, individually overridable seam points. The planned @specforge/pytest adapter lives exactly on those seams — hook implementations that record entity/test outcomes and serialize the shared report schema. Krekel's instinct that the format outlives the tool (pytest's JUnit XML output outlived pytest itself in CI pipelines) is the argument for keeping specforge-report.json the only contract the adapter honors.

## References for SpecForge
**Key works**
- [pytest-dev/pytest](https://github.com/pytest-dev/pytest) — GitHub. Core architecture: pluggy hooks, collection tree, reporting pipeline — every extension point the adapter uses.
- [Writing plugins](https://docs.pytest.org/en/stable/how-to/writing_plugins.html) — pytest official docs. pytest11 entry points, conftest plugins, hook registration — the concrete recipe for @specforge/pytest.
- **pytest History** — pytest official docs. Documents the py.test→pytest lineage and the design decisions since 2004, including the plugin-first philosophy.
- **FOSDEM 2026 talk** — fosdem.org. Krekel's own retrospective on instigating and maintaining pytest for a decade alongside PyPy.

## Study first
1. Writing plugins guide — entry points, hook discovery order, assertion rewriting
2. pluggy's hookspec/hookimpl model — how the adapter stays version-tolerant
3. pytest's JUnit XML output path — precedent for a runner emitting a portable report
