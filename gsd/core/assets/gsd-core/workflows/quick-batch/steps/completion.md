<!-- Ported from gsd-core v1.19.0 gsd-core/workflows/quick-batch/steps/completion.md, MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD -->
**Step 9: Completion**

For every item that merged successfully in Step 7 AND (NOT `$VALIDATE_MODE`,
OR Step 8 routed it to `complete`): its quick plan and its summary are the
record. The item is `complete` once its plan's summary is (the plan's computed
status), and the batch's `status` is `complete` once every item is: nothing
else is written, and re-running this step changes nothing.

Items NOT reaching this call — `human_needed`, `failed` (planner/checker/
merge/verification failure), or still `blocked`/`pending` (a dependency
failed, row 32) — are left exactly as their respective routing step set
them. No STATE row, no `complete` status.

**Final commit.** Stage every artifact produced this run (PLAN.md, SUMMARY.md,
`--research` RESEARCH.md, `--validate` VERIFICATION.md, per item, plus
the spec changes) and commit:
`$BATCH_ARTIFACT_FILES` is a bash ARRAY (not a plain string — a plain
space-joined string re-splits unpredictably under `set -f`/globbing and
diverges between bash and zsh, the #4109 word-splitting bug class):
```bash
COMMIT_DOCS="true"   # commit_docs: every spec change is committed with the work
if [ "$COMMIT_DOCS" != "false" ]; then
  ADDS=(); for f in "${BATCH_ARTIFACT_FILES[@]}"; do [ -e "$f" ] || [ -n "$(specforge git show "HEAD:$f" --pick .sha 2>/dev/null)" ] && ADDS+=(--add "$f"); done
  specforge git commit "${ADDS[@]}" -m "docs(quick-batch-${BATCH_ID}): ${ITEM_COUNT} item(s)" && specforge collect --runner gsd-git --yes
fi
```

**Final report.** Re-load the batch and its items' statuses, and summarize
by status:

```bash
QB_STATUS=$(specforge query "$BATCH_ID" --depth 0 --kinds quick_batch --pick '.nodes[0].fields.status') || exit 1   # open or complete
QB_ITEM_STATUSES=$(specforge query "$BATCH_ID" --depth 1 --kinds plan --pick '.nodes[kind=plan]' --fields id,fields.status) || exit 1   # one quick id and plan status row per item
```

```
---
GSD > QUICK BATCH COMPLETE

Batch ${BATCH_ID}: ${ITEM_COUNT} item(s)
  Complete: ${complete_count}
  Failed: ${failed_count}${failed_count > 0 ? ' (' + failed_reasons + ')' : ''}
  Needs review: ${human_needed_count}
  Blocked: ${blocked_count}

${failed_count + human_needed_count > 0 ? 'Resume after resolving: /gsd:quick-batch --resume ' + BATCH_ID : ''}
---
```

If EVERY item is `complete`, this is a clean finish — no further action
needed. If any item is `failed`/`human_needed`/`blocked`, the batch stays
resumable: fix the underlying issue (or accept the failure), then re-run
`/gsd:quick-batch --resume ${BATCH_ID}` — `resumeBatch`'s own propagation
(unmodified from Phase 3) re-evaluates eligibility from the current state, no
special quick-batch-side recovery logic needed.
