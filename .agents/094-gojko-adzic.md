# 094 — Gojko Adzic

**Cluster:** C11 — Testing & BDD (verify statements, collect, report protocol)
**Roster role:** Specification by Example; living documentation
**SpecForge anchors:** spec-to-test traceability — crates/specforge-cli/src/trace.rs (`specforge trace`), specforge coverage merging per-language reports, specforge-report.json mapping entity IDs to test outcomes

## Why this engineer
Adzic studied dozens of high-performing teams and distilled Specification by Example: derive executable specifications collaboratively, then keep them as living documentation. The load-bearing artifact is traceability from a business-facing spec statement to the tests proving it — precisely what `specforge trace` walks (entity → contract → verify → test results) and what the coverage matrix automates. Adzic's work defines what makes such a mapping trustworthy: key examples, not blanket test counts, and a report anyone can read.

## References for SpecForge
**Key works**
- **Specification by Example** — Manning, 2011. The canonical study of turning examples into executable specs — the blueprint verify statements + coverage gating implement.
- [Specification by Example, 10 years later](https://gojko.net/2020/03/17/sbe-10-years.html) — gojko.net, 2020. Retrospective on which practices endured; informs what the traceability matrix should promise.
- **Impact Mapping** — Neuri Consulting, 2012. Deliverables-to-outcomes reasoning — same chain as behaviors → features → milestones in the graph.
- **Bridging the Communication Gap** — Neuri Consulting, 2009. The precursor: fitting testing into requirements so specs never rot — the "living documentation" idea the report protocol serves.

## Study first
1. Specification by Example — living documentation and key-example chapters
2. The 10-years-later retrospective — what traceability survives contact with teams
3. Impact Mapping — linking graph entities to outcomes, not just tests
