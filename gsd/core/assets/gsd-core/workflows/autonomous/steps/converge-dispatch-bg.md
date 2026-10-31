<!-- Ported from gsd-core v1.19.0 gsd-core/workflows/autonomous/steps/converge-dispatch-bg.md, MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD -->
## Converge Dispatch (Background)

If `PLAN_STRATEGY=converge`, print: `◆ Spawning background plan-convergence loop for phase ${PHASE_NUM}... (runs in a subagent — no output until it returns, ~1–5 min; expected, not a freeze)`

```
Agent(
  description="Plan convergence phase ${PHASE_NUM}: ${PHASE_NAME}",
  run_in_background=true,
  prompt="Run plan convergence for phase ${PHASE_NUM}: Skill(skill=\"gsd-plan-review-convergence\", args=\"${PHASE_NUM} ${CONVERGENCE_ARGS}\")"
)
```
