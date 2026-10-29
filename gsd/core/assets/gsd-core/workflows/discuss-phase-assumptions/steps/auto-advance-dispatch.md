<!-- Ported from gsd-core v1.18.0 gsd-core/workflows/discuss-phase-assumptions/steps/auto-advance-dispatch.md, MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD -->
**If `--auto` flag present OR `AUTO_MODE` is true:**

Display banner:
```text
### GSD ► AUTO-ADVANCING TO PLAN

Context captured (assumptions mode). Launching plan-phase...
```

Launch: `Skill(skill="gsd-plan-phase", args="${PHASE} --auto")`

Handle return: PHASE COMPLETE / PLANNING COMPLETE / INCONCLUSIVE / GAPS FOUND
(identical handling to discuss-phase.md auto_advance step)
