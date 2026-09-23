# 098 — Anthony Fu

**Cluster:** C11 — Testing & BDD (verify statements, collect, report protocol)
**Roster role:** Vitest creator; Vite/Vue/Nuxt core team
**SpecForge anchors:** primary report adapter target — @specforge/vitest custom Reporter implementing the vitest/node Reporter API to emit specforge-report.json; consumed by specforge collect

## Why this engineer
Fu created Vitest, the default JS/TS runner for the ecosystem SpecForge's tooling targets, and gave it a first-class Reporter API: onInit through onTestRunEnd, with typed TestModule/TestCollection/TestResult objects walking the whole run. The primary SpecForge TS adapter is simply a reporter — subscribe to the lifecycle, map test names to spec entity IDs, write the report. Vitest being Vite-native also makes it the runner a VS Code user already has, minimizing friction in the collect story.

## References for SpecForge
**Key works**
- [vitest-dev/vitest](https://github.com/vitest-dev/vitest) — GitHub. The runner the primary report adapter targets.
- [Reporters (advanced API)](https://vitest.dev/api/advanced/reporters.html) — Vitest official docs. The full Reporter lifecycle (onInit, onTestRunStart/End, per-case hooks) the @specforge/vitest adapter implements.
- [Why Vitest](https://antfu.me/posts/why-vitest) — antfu.me, 2021. Design rationale: reuse Vite's pipeline, structured results over stdout scraping — why a reporter (not log parsing) is the right adapter seam.
- [antfu.me](https://antfu.me) — Personal site. Current role and full project index for the Vitest/Vite ecosystem he stewards.

## Study first
1. Reporters advanced API — onTestRunEnd module/case traversal
2. vitest/node exports: TestModule, TestCollection, TestCase result shapes
3. Why Vitest — the Vite-native design decisions the adapter rides on
