<!-- Ported from gsd-core v1.19.0 gsd-core/workflows/plan-phase/steps/reviews-prerequisite.md, MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD -->
## 2.5. Validate `--reviews` Prerequisite

**Skip if:** No `--reviews` flag.

**If `--reviews` AND `--gaps`:** Error — cannot combine `--reviews` with `--gaps`. These are conflicting modes.

**If `--reviews` AND `has_reviews` is false (no REVIEWS.md in phase dir):**

Error:
```
No review recorded for Phase {X}. Run reviews first:

/gsd:review --phase {X}

Then re-run /gsd:plan-phase {X} --reviews
```
Exit workflow.
