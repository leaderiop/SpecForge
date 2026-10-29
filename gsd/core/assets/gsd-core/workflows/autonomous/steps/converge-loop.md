<!-- Ported from gsd-core v1.18.0 gsd-core/workflows/autonomous/steps/converge-loop.md, MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD -->
## Converge Loop

If `PLAN_STRATEGY=converge`, run the convergence loop:

```
Skill(skill="gsd-plan-review-convergence", args="${PHASE_NUM} ${CONVERGENCE_ARGS}")
```
