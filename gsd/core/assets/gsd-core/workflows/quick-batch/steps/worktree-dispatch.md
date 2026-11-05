<!-- Ported from gsd-core v1.19.0 gsd-core/workflows/quick-batch/steps/worktree-dispatch.md, MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD -->
**Step 6: Executor dispatch (sequential mode)**

This is the batch's MUTATING wave: executors run one item at a time on the
working tree (`@gsd/core` has no worktree isolation, row 6),
unlike planning/research above.

No fork base to check: no worktree is created, so nothing degrades to sequential mode.














**Effective concurrency for this MUTATING wave** (sequential mode runs
one item at a time regardless of `--jobs`/capacity — row 6):
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
   (executor committed, `SUMMARY.md` written) and Step 9's completion leaves
   `BATCH.json` at `pending` with no STATE.md row yet (that row is written
   only in Step 9) — so on `--resume`, such an item still comes back
   `eligible` here. Before computing `spawn-plan`, determine which eligible
   items already have a summary `${quick_id}/${quick_id}-SUMMARY` (`specforge query "${quick_id}/${quick_id}-SUMMARY" --depth 0`, on the working tree, where its executor wrote it; same
   `item_dir` derivation via `generate-slug` every other step uses), then
   let the PURE `quick-batch filter-executed` verb
   (`filterAlreadyExecuted`, `src/quick-batch-dispatch.cts`) decide which
   ids are actually safe to spawn — never re-derive that split inline:
   ```bash
   EXECUTED_IDS_JSON="[]" # JSON array of quick_ids whose SUMMARY.md already exists on disk this round
QB_FILTER_JSON='{"spawnEligible":[],"alreadyExecuted":[]}'   # quick-batch filter-executed: the batch router is not part of @gsd/core; spawn the eligible ids that have no summary yet
   ```
   Parse `spawnEligible` (safe to spawn this round) and `alreadyExecuted`
   (diagnostic only — report these as "already executed, routing to completion"
   rather than dispatching them). Replace `$ELIGIBLE_IDS_JSON` with
   `spawnEligible` before continuing to backpressure below.

   NEVER re-dispatch any id `filter-executed`
   returns in `alreadyExecuted`. It is not lost: its executor committed on the
   working tree, and Step 9 completes any `pending` item whose summary exists,
   independent of this eligible/spawn list — dropping it here only prevents
   the duplicate dispatch, it does not remove it from the batch.



2. **Backpressure.** Not every eligible item necessarily spawns this round —
   cap fan-out at `$EXEC_CONCURRENCY` minus current in-flight count (row
   27/39):
   ```bash
QB_SPAWN_JSON='{"spawn":[],"pending":[]}'   # quick-batch spawn-plan: the batch router is not part of @gsd/core; spawn up to $EXEC_CONCURRENCY eligible items
   ```
   Parse `spawn` (dispatch these now) and `pending` (leave `pending` in
   `BATCH.json` — already the case, no write needed; NEVER mark these
   `failed`, NEVER increase fan-out to compensate).

3. **Dispatch executors, ONE AT A TIME per `spawn` item** (sequential mode on
   the working tree, each awaited before the next,
   `execute-phase.md`'s own discipline):

   For each item in `spawn`, in order:

   ```bash
SLUG=$(printf '%s' "$description" | tr '[:upper:]' '[:lower:]' | tr -cs 'a-z0-9' '-' | cut -c1-40 | sed 's/^-//; s/-$//')
   ITEM_DIR="${quick_id}"   # the quick plan's id: a quick task is entities, not a directory
   ```

   **Executor dispatch:** one `Agent()` per item, on the working tree,
   awaited before the next. Same prompt shape as `/gsd:quick`'s own
   executor dispatch (Step 6 of `quick.md`) — required_reading, agent skills,
   `<submodule_commit_guard>` using this project's `$SUBMODULE_PATHS`
   (identical block, verbatim) — with these differences:
   ```
   Agent(
     prompt="
   Execute quick-batch item ${quick_id}.

   <required_reading>
   - `specforge query ${quick_id} --depth 1 --format context` (Plan)

   - ./CLAUDE.md or ./.claude/CLAUDE.md (if exists)
   </required_reading>

   ${AGENT_SKILLS_EXECUTOR}

   <submodule_commit_guard>
   (same SUBMODULE_PATHS fail-loud guard as /gsd:quick — see gsd-core/workflows/quick.md Step 6)
   </submodule_commit_guard>

   <constraints>
   - Execute all tasks in the plan; commit each task atomically
   - Record the summary `${quick_id}/${quick_id}-SUMMARY` with `status complete`
   - NEVER invoke /gsd:quick or any other GSD command — you are a leaf, not a coordinator
   - NEVER write the batch notes
   - Do NOT update STATE.md or ROADMAP.md — the orchestrator owns those writes after every item in this dispatch round completes (ADR-1239 single-writer invariant)
   - Do NOT commit docs artifacts (SUMMARY.md, STATE.md, PLAN.md) — the orchestrator commits them at completion
   </constraints>
   ",
     subagent_type="gsd-executor",
     model="{executor_model}",

     description="Execute ${quick_id}: ${description}"
   )
   ```
   No worktree is created or recorded: the executor commits on the working tree, so Step 7 has nothing to merge.
































   > **ORCHESTRATOR RULE — CODEX RUNTIME**: after each `Agent()` call above, wait for it to return before dispatching the next item.

4. **After every item dispatched this round returns:** verify
   summary `${quick_id}/${quick_id}-SUMMARY` exists (`specforge query "${quick_id}/${quick_id}-SUMMARY" --depth 0`). If missing, the item stays
   `pending` for diagnosis rather than guessing
   completion — do not complete it this round.

Continue to Step 7 once every eligible item has been dispatched (across
however many rounds backpressure required) and returned.
