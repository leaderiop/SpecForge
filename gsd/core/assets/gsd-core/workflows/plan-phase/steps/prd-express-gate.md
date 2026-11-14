<!-- Ported from gsd-core v1.19.0 gsd-core/workflows/plan-phase/steps/prd-express-gate.md, MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD -->
## 3.5. Handle PRD Express Path

**Skip if:** No `--prd` flag in arguments.

**If `--prd <filepath>` provided:**

Follow `gsd-core/workflows/plan-phase/steps/prd-express-path.md`. That step takes the PRD (`$PRD_FILE`) and turns it into the phase's context: each requirement, story or criterion becomes a locked decision, whatever the PRD leaves open falls under "Claude's Discretion", and the canonical refs come from the roadmap and the specs the PRD cites; it then commits, sets `context_content` and skips step 4, as there is no context left to load. The rest of the workflow proceeds normally with the PRD-derived context.

