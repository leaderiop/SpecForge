<!-- Ported from gsd-core v1.19.0 gsd-core/workflows/plan-phase/steps/research-only-early-exit.md, MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD -->
### Research-Only Early Exit (`--research-phase`)

**Skip if:** `RESEARCH_ONLY` is `false` (the default).

**If `RESEARCH_ONLY=true`:** the user invoked `/gsd:plan-phase --research-phase <N>` for research-only mode. Do **not** continue to Section 5.5+ (validation strategy, planner, plan-checker, verification, gaps, bounce, post-planning-gaps). Print the research-complete summary and exit cleanly:

```text
✓ Research-only mode complete (#3042)

  Phase:       ${PHASE}
  RESEARCH.md: ${research_path}

Re-run /gsd:plan-phase ${PHASE} to plan the phase using this research,
or /gsd:plan-phase ${PHASE} --research to refresh research and plan.
```

This exits the workflow. The planner / plan-checker / verifier blocks below are skipped.
