<!-- Ported from gsd-core v1.19.0 gsd-core/workflows/execute-phase/steps/completion-reconciliation.md, MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD -->
# Completion reconciliation (#4217, split A of #3754)

Read and follow this fragment from `execute-phase.md` step 4 whenever an executor's
completion is in question. It owns the whole reconciliation policy — both arms — so the
host wait step stays inside the ADR-857 Phase 6 byte ceiling (#1168).

**Reconcile FIRST, classify SECOND.** How the child's session ended is bookkeeping
about the transport; what it wrote to disk and to git is the evidence about the work.

## When this runs

1. **No terminal response** — a spawned agent does not return a normal terminal
   completion signal but appears to have finished its work (or may still be running).
2. **Abnormal end** — the child's session ended without a normal terminal completion
   response: interrupted, aborted, closed, killed, timed out, or ended `turn_aborted` —
   INCLUDING ends the orchestrator itself initiated. **An abnormally-ended child is
   not evidence of failure (#4217):** the orchestrator's own interrupt/close says
   nothing about whether the work completed; only the artifacts do.

This policy applies to EVERY runtime and every isolation path — harness `Agent()`
dispatches, orchestrator-worktree process spawns, and sequential dispatch alike. Never
block indefinitely waiting for a signal; verify via filesystem and git state.

## Probes (per plan in the wave)

```bash
# the commands below are specforge commands: there is no launcher to define
# For each plan in this wave, check if the executor finished:
SUMMARY_EXISTS=$(test -f "{phase_dir}/{plan_number}-{plan_padded}-SUMMARY.md" && echo "true" || echo "false")
# #5164: the plan's commits come from the evaluation-scope resolver — anchored on the SUBJECT,
# zero-pad tolerant (#4003, #4619, #4748), reachable from THIS branch only (the former
# any-branch lookup let a commit on another branch satisfy the probe); the 1-hour window stays.
# Exit 69 (UNAVAILABLE) is the resolver saying "could not look": an empty COMMITS_FOUND is then NOT "no commits".
COMMITS_SCOPE=$(specforge collect --runner gsd-git --yes >/dev/null 2>&1; specforge query "{phase_number}-{plan_padded}" --kinds commit --format json | jq -c '{commits: [.nodes[]|select(.kind=="commit")|{sha: .fields.hash}]}') && COMMITS_SCOPE_RC=0 || COMMITS_SCOPE_RC=$?
COMMITS_FOUND=""
if [ "$COMMITS_SCOPE_RC" -eq 0 ]; then COMMITS_FOUND=$(printf '%s' "$COMMITS_SCOPE" | jq -r '.commits[0].sha // empty' 2>/dev/null); fi
COMMITS_SINCE_DISPATCH=$(git log "${EXPECTED_BRANCH}" --since="${DISPATCH_TS}" --oneline | head -1)
```

## Verdicts

**If `COMMITS_SCOPE_RC` is non-zero** (`69` `UNAVAILABLE`: the commit scope could not be resolved, #5170): the commit probe has no answer. Do not read the empty `COMMITS_FOUND` as "no matching commits", do not route to the failure handler on it, and do not re-dispatch an executor: surface `⚠ Commit probe unavailable for {Plan ID} (evaluation-scope exit {COMMITS_SCOPE_RC})` and stop for the user.

**If SUMMARY.md exists AND matching commits are found:** the agent completed
successfully — treat the plan as complete WITHOUT requiring another terminal child
response, proceed to step 5, and do NOT re-dispatch a fresh executor for this plan:
the work is already committed, and a second executor would redo it on top of itself.
Log: `"✓ {Plan ID} completed (verified via spot-check — completion signal not received)"`.

**If SUMMARY.md does NOT exist after a reasonable wait:** the agent may still be
running or may have failed silently. Check `git log --oneline -5` for recent
activity. If commits are still appearing, wait longer. If no activity, report the
plan as failed and route to the failure handler in step 6.

Evidence is BOTH probes or neither: a SUMMARY without matching commits, and matching
commits without a SUMMARY, are each incomplete evidence — never auto-complete on one
of them. When an abnormal end reconciles to no completion evidence, it stays failed:
route to the failure handler exactly as a normal failure would, and let the
safe-resume gate handle any un-summarized commits on the next run.
