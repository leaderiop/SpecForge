<!-- Ported from gsd-core v1.19.0 gsd-core/workflows/execute-phase/detail/elaboration.md, MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD -->
# execute-phase.md — deferred elaboration

Read in full when `workflow.compact_content` is `false` (the default) — see
`gsd-core/references/compact-content-gate.md` for the check and resolution rule this
spine defers to. Each `§` below is the full text the spine condenses at the point it
names.

(safe_resume_gate, checkpoint_handling, and auto_copy_learnings are stated verbatim in the
spine itself — pre-existing structural drift guards in this repo's test suite pin their exact
wording and bash there, so nothing about them is deferred to this file.)

## § 1 — check_interactive_mode

**Parse `--interactive` flag from $ARGUMENTS.**

**If `--interactive` flag present:** Switch to interactive execution mode.

Interactive mode executes plans sequentially **inline** (no subagent spawning) with user
checkpoints between tasks. The user can review, modify, or redirect work at any point.

**Interactive execution flow:**

1. Load plan inventory as normal (discover_and_group_plans)
2. For each plan (sequentially, ignoring wave grouping):

   a. **Present the plan to the user:**
      ```
      ## Plan {plan_id}: {plan_name}

      Objective: {from plan file}
      Tasks: {task_count}

      Options:
      - Execute (proceed with all tasks)
      - Review first (show task breakdown before starting)
      - Skip (move to next plan)
      - Stop (end execution, save progress)
      ```

   b. **If "Review first":** Read and display the full plan file. Ask again: Execute, Modify, Skip.

   c. **If "Execute":** Read and follow `~/.claude/gsd-core/workflows/execute-plan.md` **inline**
      (do NOT spawn a subagent). Execute tasks one at a time.

   d. **After each task:** Pause briefly. If the user intervenes (types anything), stop and address
      their feedback before continuing. Otherwise proceed to next task.

   e. **After plan complete:** Show results, commit, write the plan's summary entity (execute-plan's create_summary),
      then present next plan.

3. After all plans: proceed to verification (same as normal mode).

(The spine's own condensed text already states the handle_branching hand-off; not repeated here.)

## § 2 — cross_ai_delegation

**Optional step 2.5 — Delegate plans to an external AI runtime.**

This step runs after plan discovery and before normal wave execution. It identifies plans
that should be delegated to an external AI command and executes them via stdin-based prompt
delivery. Plans handled here are removed from the execute_waves plan list so the normal
executor skips them.

**Activation logic:**

1. No flag forces or disables this step: the command declares no `--cross-ai` or `--no-cross-ai`.
2. Only `ready` plans are eligible (#4628 — every dependency has completion evidence). A not-ready plan is never delegated: it would dispatch over an incomplete predecessor, the exact failure #4628 reports.
3. Check each plan's frontmatter for `cross_ai: true` AND verify config
   `workflow.cross_ai_execution` is `true`. Plans matching both conditions are marked for cross-AI.

```bash
# the commands below are specforge commands: there is no launcher to define
CROSS_AI_ENABLED="{{setting:workflow.cross_ai_execution|false}}"   # workflow.cross_ai_execution (the settings)
CROSS_AI_CMD="{{setting:workflow.cross_ai_command|}}"   # workflow.cross_ai_command (the settings)
CROSS_AI_TIMEOUT="{{setting:workflow.cross_ai_timeout|300}}"   # workflow.cross_ai_timeout (the settings)
```

**If no plans are marked for cross-AI:** Skip to execute_waves.

**If plans are marked but `cross_ai_command` is empty:** Error — tell user to set
`workflow.cross_ai_command` via `specforge settings set @gsd/core workflow.cross_ai_command "<command>"`.

**For each cross-AI plan (sequentially):**

1. **Construct the task prompt** from the plan file:
   - Read the plan's objective and tasks: `specforge query "$PLAN_ID" --depth 1 --format context`
   - Append the project's context (name, description, tech stack) from the plan's context pack:
     `specforge context gsd/executor "$PLAN_ID"`
   - Format as a self-contained execution prompt

2. **Check for dirty working tree before execution:**
   ```bash
   if [ "$(specforge git diff --pick .count 2>/dev/null)" != 0 ]; then
     echo "WARNING: dirty working tree detected — the external AI command may produce uncommitted changes that conflict with existing modifications"
   fi
   ```

3. **Run the external command** from the project root, writing the prompt to stdin.
   Never shell-interpolate the prompt — always pipe via stdin to prevent injection:
   ```bash
   echo "$TASK_PROMPT" | specforge run --deadline "${CROSS_AI_TIMEOUT}" -- ${CROSS_AI_CMD} > "$CANDIDATE_SUMMARY" 2>"$ERROR_LOG"
   EXIT_CODE=$?
   ```

4. **Evaluate the result:**

   **Success (exit 0 + valid summary):**
   - Read `$CANDIDATE_SUMMARY` and validate it contains meaningful content
     (not empty, and it names a substantive one-liner and what the plan delivered)
   - Record it as the plan's `summary` entity with the summary template's typed edits, then close the plan with
     `specforge gsd complete-plan`: the plan's status and the roadmap's progress are computed from it
   - Mark plan as handled — skip it in execute_waves

   **Failure (non-zero exit or invalid summary):**
   - Display the error output and exit code
   - Warn: "The external command may have left uncommitted changes or partial edits
     in the working tree. Review `git status` and `git diff` before proceeding."
   - Offer three choices:
     - **retry** — run the same plan through cross-AI again
     - **skip** — fall back to normal executor for this plan (re-add to execute_waves list)
     - **abort** — stop execution entirely, preserve state for resume

5. **After all cross-AI plans processed:** Remove successfully handled plans from the
   incomplete plan list so execute_waves skips them. Any skipped-to-fallback plans remain
   in the list for normal executor processing.
