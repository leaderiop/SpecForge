<!-- Ported from gsd-core v1.19.0 gsd-core/workflows/quick-batch.md, MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD -->
@~/.claude/gsd-core/references/response-language-directive.md
<purpose>
Batch several `/gsd:quick`-shaped tasks together (#3676, epic #3344, ADR-1239
"Quick-batch binding"). ONE coordinator (this workflow) owns every shared
write — `BATCH.json`, STATE.md, each executor's dispatch — and never
delegates them to a leaf. Leaves (planner/researcher/checker/executor/
verifier) return structured results only; they never invoke `/gsd:quick`,
never touch `BATCH.json`, and never write STATE.md/ROADMAP.md themselves
(single-writer invariant).

Dispatch decisions (effective concurrency, deterministic merge order, spawn
backpressure, failure/verification routing) are computed by the pure
`quick-batch-dispatch.cts` module (via the `quick-batch` CLI verbs) — this
workflow never re-derives that logic inline.
</purpose>

<required_reading>
Read all files referenced by the invoking prompt's execution_context before starting.
</required_reading>

<available_agent_types>
Valid GSD subagent types (use exact names — do not fall back to 'general-purpose'):
- gsd-phase-researcher — Researches technical approaches for an item
- gsd-planner — Creates a plan for one item (`quick-batch` mode)
- gsd-plan-checker — Reviews one item's plan before execution
- gsd-executor — Executes one item's plan, commits, creates SUMMARY.md
- gsd-verifier — Verifies one item's goal achievement
</available_agent_types>

<process>
**Step 1: Parse arguments, resolve mode**

```bash
# the commands below are specforge commands: there is no launcher to define
RESPONSE_LANGUAGE="{{setting:response_language|}}"   # response_language (the settings)
```

**If `response_language` is set:** all user-facing questions/prompts/explanations MUST be presented in `{response_language}`. Technical terms, code, file paths, and subagent prompts stay in English.

The host has parsed and checked the arguments (an undeclared option such as `--discuss` or `--full` is refused before this text renders): read them from these lines.

```bash
JOBS="{{arg:jobs|auto}}"
VALIDATE_MODE="{{arg:validate|false}}"
RESEARCH_MODE="{{arg:research|false}}"
RESUME_BATCH_ID="{{arg:resume|}}"
TASK_FILE="{{arg:file|}}"
TASKS="{{arg:tasks|}}"
case "$JOBS" in
  auto|[1-9]|[1-9][0-9]) ;;
  *) echo "--jobs is auto or a number: $JOBS" >&2; exit 1 ;;
esac

```

`$JOBS` is `auto` or a number, `$VALIDATE_MODE` and `$RESEARCH_MODE` are `true` or `false`, and `$RESUME_BATCH_ID` is the batch to resume (empty unless `--resume <id>`).

The task list is `$TASK_FILE` (`--file <path>`), else `$TASKS` (the words after the options).

```bash




VALIDATE_PARAM=""; if [ "$VALIDATE_MODE" = true ]; then VALIDATE_PARAM="--validate"; fi
RESEARCH_PARAM=""; if [ "$RESEARCH_MODE" = true ]; then RESEARCH_PARAM="--research"; fi

INIT=""   # no init bundle: the settings block and the argument block are all this run needs

AGENT_SKILLS_PLANNER=""   # the host install carries the skills of gsd-planner
AGENT_SKILLS_EXECUTOR=""   # the host install carries the skills of gsd-executor
AGENT_SKILLS_CHECKER=""   # the host install carries the skills of gsd-plan-checker
AGENT_SKILLS_VERIFIER=""   # the host install carries the skills of gsd-verifier
AGENT_SKILLS_RESEARCHER=""   # the host install carries the skills of gsd-phase-researcher
```

Parse `$INIT` for: `planner_model`, `executor_model`, `checker_model`, `verifier_model`, `researcher_model`, `commit_docs`, `roadmap_exists`, `planning_exists`.

<!-- #2517 model-omit-on-inherit -->

> **Model omission (#2517).** Every `Agent()` dispatch below (planner, researcher, plan-checker, executor, verifier) MUST omit the `model` parameter entirely when the value it would carry (`planner_model`, `checker_model`, `executor_model`, `verifier_model`, `researcher_model`) is `"inherit"` or empty. An empty value 404s on runtimes without native tier aliases — the default on non-Claude runtimes, where the installer writes `resolve_model_ids:"omit"`. Omitting it inherits the orchestrator's model. See @gsd-core/references/model-profile-resolution.md.

```bash


# @gsd/core has no worktree isolation: workflow.use_worktrees is ignored
RUNTIME="{{setting:runtime|}}"   # runtime (the settings)
```

**If `roadmap_exists` is false:** Error — quick-batch requires an active project with ROADMAP.md. Run `/gsd:new-project` first.

If the project uses git submodules, parse `SUBMODULE_PATHS` from `.gitmodules` exactly as `/gsd:quick` does (a fail-loud commit-time guard, applied per item at commit time — see `gsd-core/workflows/quick.md` Step 2 for the identical block, reused verbatim below):

```bash
if [ -f .gitmodules ]; then
  SUBMODULE_PATHS=$(specforge git submodules --pick '.submodules[].path' 2>/dev/null)
else
  SUBMODULE_PATHS=""
fi
```

**Resolve capacity now (#3676 design row 3-4).** `--jobs auto`/omitted uses this
value alone; `--jobs N` is capped by it (`min(taskCount, N, capacity)` — the
`quick-batch effective-concurrency` verb, called per-wave below, does the
arithmetic; this is only the raw resolve):
```bash
CAPACITY=1   # dispatch-capacity: one at a time, no background dispatch is part of @gsd/core
```

**No isolation to resolve.** `@gsd/core` runs no worktree isolation: every executor runs in sequential mode on the working tree (no isolation gate, no harness flag, no orphan sweep).














Display banner:
```
### GSD ► QUICK BATCH
◆ jobs=${JOBS} validate=${VALIDATE_MODE} research=${RESEARCH_MODE}${RESUME_BATCH_ID:+ resume=${RESUME_BATCH_ID}}
```

---

**Step 2: Resume or create**

If `$RESUME_BATCH_ID` is set: read and execute `gsd-core/workflows/quick-batch/steps/resume-mode.md`.
It loads the batch via `quick-batch
resume`, refuses closed on an unknown batch id or a diverged base revision,
and sets `$BATCH_ID`/`$BATCH_MANIFEST_JSON` for the steps below. Task-list
parsing and `quick-batch create` are skipped entirely.

Otherwise: read and execute `gsd-core/workflows/quick-batch/steps/batch-init.md`.
It parses the task list (inline or `--file`) and creates the
batch via `quick-batch create`, setting the same `$BATCH_ID`/
`$BATCH_MANIFEST_JSON` pair.

Either path converges on the same post-condition — continue to Step 3.

---

<!-- gsd:section id="research-phase" when="flag:--research" -->
If `section_manifest` is `null` or `"research-phase"` is in its `included` list: read and execute `gsd-core/workflows/quick-batch/steps/research-phase.md`. Otherwise skip — do not read the file.
<!-- /gsd:section -->

---

**Step 4: Per-DAG-layer planning**

Read and execute `gsd-core/workflows/quick-batch/steps/planner-wave.md`. It
dispatches a planner per eligible item (one `Agent()` per message, full task
catalog in every prompt), persists parsed `depends_on`/`files_modified` via
`quick-batch update` after each layer, and — when `$VALIDATE_MODE` — runs the
per-item plan-checker loop (`gsd-core/workflows/quick-batch/steps/plan-checker-loop.md`)
before advancing to the next layer.

---

**Step 6: Executor dispatch (sequential mode)**

Read and execute `gsd-core/workflows/quick-batch/steps/worktree-dispatch.md`.
Executors run one item at a time on the working tree (sequential mode).



---

**Step 7: Deterministic merge**

Nothing to merge: every executor committed on the working tree in Step 6, so
`gsd-core/workflows/quick-batch/steps/merge-wave.md` is not reached.


---

<!-- gsd:section id="verification-wave" when="flag:--validate" -->
If `section_manifest` is `null` or `"verification-wave"` is in its `included` list: read and execute `gsd-core/workflows/quick-batch/steps/verification-wave.md`. Otherwise skip — do not read the file.
<!-- /gsd:section -->

---

**Step 9: Completion**

Read and execute `gsd-core/workflows/quick-batch/steps/completion.md`. Calls
`completeQuickItem` (via `quick-batch complete`) only for a genuinely
complete item, updates STATE.md, and prints the final batch report.

</process>

<success_criteria>
- [ ] `--discuss`/`--full` rejected with a usage error before any dispatch
- [ ] A malformed `--jobs` value rejected before any dispatch
- [ ] `--resume <batch-id>` skips task-list parsing, dispatches only eligible items
- [ ] Task list parsed (inline or `--file`, ≥2 items) and batch created otherwise
- [ ] Planner dispatched per eligible item per DAG layer, full task catalog in prompt, `depends_on`/`files_modified` requested ALWAYS
- [ ] (--research) Researcher dispatched per item before planning
- [ ] (--validate) Plan-checker loop runs per item after planning (≤2 iterations)
- [ ] Executors dispatched one item at a time on the working tree (sequential mode)
- [ ] A mutating wave runs at concurrency 1; a research-only wave is unaffected

- [ ] (--validate) Verifier dispatched per item post-merge; `human_needed` never completes the item, `gaps_found` fails it without rollback or retry

- [ ] `completeQuickItem` called only for genuinely complete items; STATE.md updated; artifacts committed
</success_criteria>
