<!-- Ported from gsd-core v1.19.0 gsd-core/workflows/quick-batch/steps/worktree-dispatch.md, MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD -->
**Step 6: Worktree create + executor dispatch**

This is the batch's MUTATING wave — worktree create/executor dispatch/merge
are the operations `isolation == "none"` caps to concurrency 1 (row 6),
unlike planning/research above.

**Auto-degrade on stale fork base (row 38, mirrors `/gsd:quick`'s own #1941
guard):**
```bash
# the commands below are specforge commands: there is no launcher to define
if [ "$ISOLATION" = "harness-worktree" ] && [ "${USE_WORKTREES:-true}" != "false" ]; then
  _QB_SHOULD_DEGRADE="false"   # worktree.base-check: worktree isolation is not part of @gsd/core
  if [ "$_QB_SHOULD_DEGRADE" = "true" ]; then
    echo "⚠ [#1941] Worktree fork base diverged — auto-degrading quick-batch to sequential mode." >&2
    USE_WORKTREES=false
    ISOLATION=none
  fi
fi
true   # dispatch-isolation: there is no mode to record (worktree isolation is not part of @gsd/core)
```

**Effective concurrency for this MUTATING wave** (`mutating` forces
`isolation == none` to 1 regardless of `--jobs`/capacity — row 6):
```bash
EXEC_CONCURRENCY=1   # quick-batch effective-concurrency: the batch router is not part of @gsd/core; one at a time

```

**Dispatch rounds.** Repeat until no item is eligible-and-not-yet-dispatched
(bounded by `$ITEM_COUNT` rounds):

1. Re-derive eligibility (also reconciles crash-window/blocked-propagation —
   safe to call repeatedly, idempotent when nothing changed):
   ```bash
QB_ELIG_JSON='{"eligible":[]}'   # quick-batch resume: the batch router is not part of @gsd/core; the eligible items are those of your notes whose dependencies are complete
   ```
   Parse `eligible` (quick ids ready to execute — every dependency already
   `complete`); `$BATCH_MANIFEST_JSON` keeps its `quick_id<TAB>description` lines.

   **Crash-window guard (mirrors `planner-wave.md`'s PLAN.md-existence
   check one layer earlier — #3677):** `quick-batch resume`'s `eligible` is
   purely status/dependency-derived; it does NOT know an item already
   finished executing. A coordinator crash between this step returning
   (executor committed, `SUMMARY.md` written) and Step 7's merge leaves
   `BATCH.json` at `pending` with no STATE.md row yet (that row is written
   only in Step 9) — so on `--resume`, such an item still comes back
   `eligible` here. Before computing `spawn-plan`, determine which eligible
   items already have a summary `${quick_id}/${quick_id}-SUMMARY` (`specforge query "${quick_id}/${quick_id}-SUMMARY" --depth 0 --path "${WT_PATH:-.}"`, in the item's recorded worktree `$WT_PATH` when it has one, where its executor wrote it; same
   `item_dir` derivation via `generate-slug` every other step uses), then
   let the PURE `quick-batch filter-executed` verb
   (`filterAlreadyExecuted`, `src/quick-batch-dispatch.cts`) decide which
   ids are actually safe to spawn — never re-derive that split inline:
   ```bash
   EXECUTED_IDS_JSON="[]" # JSON array of quick_ids whose SUMMARY.md already exists on disk this round
QB_FILTER_JSON='{"spawnEligible":[],"alreadyExecuted":[]}'   # quick-batch filter-executed: the batch router is not part of @gsd/core; spawn the eligible ids that have no summary yet
   ```
   Parse `spawnEligible` (safe to spawn this round) and `alreadyExecuted`
   (diagnostic only — report these as "already executed, routing to merge"
   rather than dispatching them). Replace `$ELIGIBLE_IDS_JSON` with
   `spawnEligible` before continuing to backpressure below.

   NEVER re-dispatch it into a second worktree for any id `filter-executed`
   returns in `alreadyExecuted`. It is not lost: Step 7's own mergeable-wave
   criterion
   (`gsd-core/workflows/quick-batch/steps/merge-wave.md` Step 1) already
   picks up any `pending` item with an on-disk `SUMMARY.md` that isn't yet
   merged, independent of this eligible/spawn list — dropping it here only
   prevents the duplicate dispatch, it does not remove it from the batch.

2. **Backpressure.** Not every eligible item necessarily spawns this round —
   cap fan-out at `$EXEC_CONCURRENCY` minus current in-flight count (row
   27/39):
   ```bash
QB_SPAWN_JSON='{"spawn":[],"pending":[]}'   # quick-batch spawn-plan: the batch router is not part of @gsd/core; spawn up to $EXEC_CONCURRENCY eligible items
   ```
   Parse `spawn` (dispatch these now) and `pending` (leave `pending` in
   `BATCH.json` — already the case, no write needed; NEVER mark these
   `failed`, NEVER increase fan-out to compensate).

3. **Create worktrees + dispatch executors, ONE AT A TIME per `spawn` item**
   (`git worktree add` races on `.git/config.lock` — never simultaneous,
   `execute-phase.md`'s own discipline):

   For each item in `spawn`, in order:

   ```bash
SLUG=$(printf '%s' "$description" | tr '[:upper:]' '[:lower:]' | tr -cs 'a-z0-9' '-' | cut -c1-40 | sed 's/^-//; s/-$//')
   ITEM_DIR="${quick_dir}/${quick_id}-${SLUG}"
   ```

   **`isolation == "harness-worktree"`:** one `Agent()` per message,
   `run_in_background: true`. Same prompt shape as `/gsd:quick`'s own
   executor dispatch (Step 6 of `quick.md`) — required_reading, agent skills,
   `<submodule_commit_guard>` using this project's `$SUBMODULE_PATHS`
   (identical block, verbatim) — with these differences:
   ```
   Agent(
     prompt="
   Execute quick-batch item ${quick_id}.

   <required_reading>
   - `specforge query ${quick_id} --depth 1 --format context` (Plan)
   - ${STATE_PATH} (Project state — READ ONLY, do not write it)
   - ./CLAUDE.md or ./.claude/CLAUDE.md (if exists)
   </required_reading>

   ${AGENT_SKILLS_EXECUTOR}

   <submodule_commit_guard>
   (same SUBMODULE_PATHS fail-loud guard as /gsd:quick — see gsd-core/workflows/quick.md Step 6)
   </submodule_commit_guard>

   <constraints>
   - Execute all tasks in the plan; commit each task atomically
   - Create summary at: ${ITEM_DIR}/${quick_id}-SUMMARY.md with `status: complete` in frontmatter
   - NEVER invoke /gsd:quick or any other GSD command — you are a leaf, not a coordinator
   - NEVER write the batch notes
   - Do NOT update STATE.md or ROADMAP.md — the orchestrator owns those writes after every item in this dispatch round completes (ADR-1239 single-writer invariant)
   - Do NOT commit docs artifacts (SUMMARY.md, STATE.md, PLAN.md) — the orchestrator commits them at completion
   </constraints>
   ",
     subagent_type="gsd-executor",
     model="{executor_model}",
     {harnessFlag}
     description="Execute ${quick_id}: ${description}"
   )
   ```
   Record `worktree_path`, `branch`, `agent_id`, `expected_base` and `allowed_bases`
   from the executor's return into `$QUICK_BATCH_WORKTREE_MANIFEST` (a tab-separated
   file, empty before the first round, one `worktree<TAB>path<TAB>branch<TAB>…` line
   each — the format `/gsd:quick`'s own `QUICK_WORKTREE_MANIFEST` uses).

   **`isolation == "orchestrator-worktree"`:** GSD creates the worktree
   (`worktree create (not part of @gsd/core; --manifest "$QUICK_BATCH_WORKTREE_MANIFEST" --agent-id ... --path ... --branch ... --base ... --files "$PLAN_FILES" --deletions "$PLAN_DELETIONS"`) then
   process-spawns the executor via `dispatch-isolation --json --cwd-target
   --prompt`, exactly as `gsd-core/workflows/execute-phase/steps/executor-isolation-dispatch.md`'s
   "orchestrator-worktree" section
   describes — reuse that mechanism verbatim, substituting this item's
   `${quick_id}`/`${ITEM_DIR}`/`${quick_id}-PLAN.md` for its
   `{plan_number}`/`{phase_dir}`/`{plan_file}` placeholders.

   **`isolation == "none"`:** no worktree. Dispatch the executor inline on the
   primary checkout (same prompt, minus the worktree-only framing), one item
   at a time — `EXEC_CONCURRENCY` is already forced to 1 in this mode.

   **Durable worktree-recovery persistence (#3677 — `harness-worktree`/
   `orchestrator-worktree` only, skip for `none`):** immediately after
   recording `{agent_id, worktree_path, branch, expected_base}` into the
   EPHEMERAL `$QUICK_BATCH_WORKTREE_MANIFEST` above, ALSO persist the same
   triple durably onto this item in `BATCH.json`:
   ```bash
: "quick-batch update: the batch router is not part of @gsd/core; keep the worktree, branch and base in your notes"
   ```
   The ephemeral manifest is a per-process `mktemp` file (same format
   `/gsd:quick`'s own `QUICK_WORKTREE_MANIFEST` uses) — it does NOT survive
   a coordinator crash/restart. A RESUMED coordinator's Step 7
   (`merge-wave.md`) reads this durable BATCH.json triple as its fallback
   for any item the crash-window guard above correctly did NOT re-dispatch
   in the current process.

   > **ORCHESTRATOR RULE — CODEX RUNTIME**: after each `Agent()` call above, wait for it to return before starting the next worktree create.

4. **After every item dispatched this round returns:** verify
   summary `${quick_id}/${quick_id}-SUMMARY` exists in its worktree, which Step 7 has not merged yet (`specforge query "${quick_id}/${quick_id}-SUMMARY" --depth 0 --path "${WT_PATH:-.}"`, `$WT_PATH` the item's recorded `worktree_path`). If missing, the item stays
   `pending`/its worktree preserved for diagnosis rather than guessing
   completion — do not proceed to merge for it this round.

Continue to Step 7 once every eligible item has been dispatched (across
however many rounds backpressure required) and returned.
