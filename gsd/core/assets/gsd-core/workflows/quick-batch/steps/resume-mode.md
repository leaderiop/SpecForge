<!-- Ported from gsd-core v1.19.0 gsd-core/workflows/quick-batch/steps/resume-mode.md, MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD -->
**Step 2a: Resume mode (only when `$RESUME_BATCH_ID` is set)**

Skip this step entirely if `$RESUME_BATCH_ID` is empty.

Resume re-derives eligibility via the batch's own `resumeBatch` propagation —
it is the single source of truth for which items are still runnable. Never
re-parse a task list or re-run `quick-batch create` on resume (row 9/16 of
the design's behavior table).

```bash
# the commands below are specforge commands: there is no launcher to define
QB_RESUME_JSON=$(specforge query "$RESUME_BATCH_ID" --depth 0 --kinds quick_batch --pick '.nodes[0].fields') && QB_RESUME_RC=0 || QB_RESUME_RC=$?   # an id no batch holds is refused
if [ "$QB_RESUME_RC" -eq 0 ]; then
  QB_BASE=$(specforge query "$RESUME_BATCH_ID" --depth 0 --pick '.nodes[0].fields.base')
  if [ -z "$QB_BASE" ] || [ "$(specforge git merge-base "$QB_BASE" HEAD --pick .sha 2>/dev/null)" != "$QB_BASE" ]; then
    echo "quick batch ${RESUME_BATCH_ID}: its base ${QB_BASE} is no longer an ancestor of HEAD: the history diverged" >&2
    QB_RESUME_RC=1
  fi
fi

```

**If `$QB_RESUME_RC` is non-zero:** the resume was refused closed — an unknown
batch id (row 18) or a diverged base revision (row 17, ADR-1239 "Base
divergence"). Print the CLI's error message verbatim and STOP. Do not dispatch
anything, do not create a new batch on the user's behalf.

**Otherwise:** `$QB_RESUME_JSON` holds the batch's fields: `items`, `base`,
`created`, its computed `status` and `eligible` (the quick ids whose plan has
not started, halted or blocked and whose dependencies are complete). There
are no `transitions` to apply: each item's status is its plan's, computed from
the graph every time it is read.

```bash
BATCH_ID="$RESUME_BATCH_ID"
QB_ITEMS=$(specforge query "$BATCH_ID" --depth 0 --pick '.nodes[0].fields.items[]') || exit 1   # the batch's quick ids, in its order
BATCH_MANIFEST_JSON=""
while IFS= read -r QB_ITEM; do
  [ -n "$QB_ITEM" ] || continue
  QB_ROW=$(specforge query "$QB_ITEM" --depth 0 --pick '.nodes[0]' --fields id,fields.slug,fields.objective) || exit 1
  BATCH_MANIFEST_JSON="${BATCH_MANIFEST_JSON:+$BATCH_MANIFEST_JSON
}$QB_ROW"
done <<< "$QB_ITEMS"   # the manifest's items in the batch's order, one quick_id, slug, description row each
```

Report to user:
```
Resuming batch ${BATCH_ID}: ${eligible.length} item(s) eligible now.
```

Continue to Step 3 in `quick-batch.md` — the DAG-layer loop in `planner-wave.md`
reads `$BATCH_MANIFEST_JSON`/`$BATCH_ID` exactly the same way whether this
batch was just created or just resumed; it re-derives per-item progress from
which artifacts already exist on disk (PLAN.md/SUMMARY.md/VERIFICATION.md),
never from a separate "resume" code path.
