# 097 — Christoph Nakazawa

**Cluster:** C11 — Testing & BDD (verify statements, collect, report protocol)
**Roster role:** Jest creator; Yarn and Metro co-creator
**SpecForge anchors:** reporter-plugin pattern for @specforge/vitest — a custom reporter stacked via the runners' reporters config, consuming structured results and emitting specforge-report.json

## Why this engineer
Nakazawa built Jest at Facebook as a zero-config test platform whose reporter layer is a first-class plugin point: config lists reporters, each receiving the run's structured events (onRunStart, per-file results, onRunComplete) and formatting whatever it wants. That is precisely the shape of @specforge/vitest — a reporter stacked beside the default one that walks the results tree and serializes entity↔test mappings to specforge-report.json. Jest proved a runner can stay ignorant of downstream consumers if it emits clean structured results.

## References for SpecForge
**Key works**
- [jestjs/jest](https://github.com/jestjs/jest) — GitHub. The Reporter interface (onRunStart/onTestResult/onRunComplete) — reference design for event-driven report emission.
- [Configuring Jest](https://jestjs.io/docs/configuration) — jestjs.io, official docs. The `reporters` array: chain a default reporter with custom summarizers — how @specforge/vitest must stack non-destructively.
- **Challenge Driven Leadership** — book, 2019. His account of building the Facebook JS tools team — context for scaling a plugin ecosystem around a runner.

## Study first
1. Jest's Reporter interface and aggregation model
2. The `reporters` config semantics — stacking custom beside default
3. How Jest's structured test results feed third-party reporters (summary/junit)
