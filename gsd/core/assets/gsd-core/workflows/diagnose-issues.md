<!-- Ported from gsd-core v1.19.0 gsd-core/workflows/diagnose-issues.md, MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD -->
@~/.claude/gsd-core/references/response-language-directive.md

<purpose>
Orchestrate parallel debug agents to investigate UAT gaps and find root causes.

After UAT finds gaps, spawn one debug agent per gap. Each agent investigates autonomously with symptoms pre-filled from UAT. Collect root causes, update UAT.md gaps with diagnosis, then hand off to plan-phase --gaps with actual diagnoses.

Orchestrator stays lean: parse gaps, spawn agents, collect results, update UAT.
</purpose>

<available_agent_types>
Valid GSD subagent types (use exact names — do not fall back to 'general-purpose'):
- gsd-debugger — Diagnoses and fixes issues
</available_agent_types>

<paths>
DEBUG_DIR=the spec

Debug files use the the spec path (hidden directory with leading dot).
</paths>

<core_principle>
**Diagnose before planning fixes.**

UAT tells us WHAT is broken (symptoms). Debug agents find WHY (root cause). plan-phase --gaps then creates targeted fixes based on actual causes, not guesses.

Without diagnosis: "Comment doesn't refresh" → guess at fix → maybe wrong
With diagnosis: "Comment doesn't refresh" → "useEffect missing dependency" → precise fix
</core_principle>

<process>

<step name="parse_gaps">
**Extract gaps from UAT.md:**

Read the "Gaps" section (YAML format):
```yaml
- truth: "Comment appears immediately after submission"
  status: failed
  reason: "User reported: works but doesn't show until I refresh the page"
  severity: major
  test: 2
  artifacts: []
  missing: []
```

For each gap, also read the corresponding test from "Tests" section to get full context.

Build gap list:
```
gaps = [
  {truth: "Comment appears immediately...", severity: "major", test_num: 2, reason: "..."},
  {truth: "Reply button positioned correctly...", severity: "minor", test_num: 5, reason: "..."},
  ...
]
```
</step>

<step name="report_plan">
**Read worktree config:**

```bash
# the commands below are specforge commands: there is no launcher to define
USE_WORKTREES="false"   # @gsd/core has no worktree isolation: workflow.use_worktrees is ignored
RUNTIME="{{setting:runtime|@host}}"   # runtime (the settings)
```

**Resolve isolation now (#2584/#2652).** Read @gsd-core/references/dispatch-isolation-gate.md
and run its `Resolve ISOLATION`, `Single-agent dispatch sites`, and `Resolve the harness flag`
blocks in order; they set `ISOLATION`/`HARNESS_FLAG` via `query dispatch-isolation`.
`ISOLATION` — not `RUNTIME` — gates the worktree decision at spawn; the `spawn_agents` step
below consumes both variables.

**Report diagnosis plan to user:**

```
## Diagnosing {N} Gaps

Spawning parallel debug agents to investigate root causes:

| Gap (Truth) | Severity |
|-------------|----------|
| Comment appears immediately after submission | major |
| Reply button positioned correctly | minor |
| Delete removes comment | blocker |

Each agent will:
1. Create DEBUG-{slug}.md with symptoms pre-filled
2. Investigate autonomously (read code, form hypotheses, test)
3. Return root cause

This runs in parallel - all gaps investigated simultaneously.
```
</step>

<step name="spawn_agents">
**Load agent skills:**

```bash
IFS= read -rd '' GSD_ARGS <<'GSD_ARGS_END' || true
$ARGUMENTS
GSD_ARGS_END
GSD_ARGS=${GSD_ARGS%$'\n'}
GSD_WS=$(echo " $GSD_ARGS" | sed -nE 's/.* --ws +([A-Za-z0-9][A-Za-z0-9._-]*).*/--ws \1/p' | head -n 1)
AGENT_SKILLS_DEBUGGER=""   # the host install carries the skills of gsd-debugger
EXPECTED_BASE=$(git rev-parse HEAD)
```

**Pre-dispatch worktree base-check (#2649, mirrors execute-phase #683/#1369 and quick #1941).**
Claude Code's `isolation="worktree"` forks new worktrees from `origin/HEAD`, not the live local
HEAD. If local HEAD has advanced without an intervening `git push` (the documented GSD steady
state — commit every step, push only on request), `origin/HEAD` is pinned to a stale ancestor
and the debug agent's `worktree_branch_check` guard halts with a base-mismatch fatal *after* the
worktree already exists, with no automatic degrade. Run the same pre-dispatch check the four
sibling dispatch sites run, and auto-degrade to sequential (main-tree) debug-agent dispatch when
the fork base cannot be reliably resolved. The verify-only `<worktree_branch_check>` guard below
stays active as a backstop in both cases.

```bash
if [ "$ISOLATION" = "harness-worktree" ]; then
  _DIAG_SHOULD_DEGRADE="false"   # worktree.base-check: worktree isolation is not part of @gsd/core
  if [ "$_DIAG_SHOULD_DEGRADE" = "true" ]; then
    _DIAG_DEGRADE_MSG=""   # worktree.base-check: worktree isolation is not part of @gsd/core
    [ -n "$_DIAG_DEGRADE_MSG" ] && printf '%s\n' "$_DIAG_DEGRADE_MSG" >&2
    echo "⚠ [#2649] Worktree fork base diverged from orchestrator HEAD — auto-degrading to sequential mode for diagnosis to avoid a base-mismatch halt." >&2
    ISOLATION=none
    USE_WORKTREES=false
  fi
fi

# Re-record after the base-check degrade, immediately before the spawn below, so the
# #3045 sentinel matches the dispatch the guard is about to see (#3045).
true   # dispatch-isolation: there is no mode to record (worktree isolation is not part of @gsd/core)

# Model resolution for the debugger spawns below (#3602).
DEBUGGER_MODEL=""   # resolve-model is not part of @gsd/core: the host picks the model
```

**Spawn debug agents — parallel only when each one is isolated:**

**`ISOLATION` decides the fan-out, not just the flag (#2652).** When
`ISOLATION = "harness-worktree"`, spawn all agents in a single message: each gets its own
worktree, so concurrent edits cannot collide. When `ISOLATION = "none"` — including after the
`orchestrator-worktree` fallback and after the #2649 base-check degrade — the agents would all
run against the **primary checkout**, so spawn them **one at a time**, waiting for each to
return before spawning the next. Fanning out unisolated debuggers is the outcome the
`orchestrator-worktree` degrade exists to avoid; degrading the flag while keeping the
parallelism would announce sequential mode and then do the opposite.

For each gap, fill the debug-subagent-prompt template and spawn:

Print: `◆ Spawning diagnostics agent... (each runs in a subagent — no output until they return, ~1–5 min; expected, not a freeze)`

Before spawning, materialize the guard into WORKTREE_GUARD: read `gsd-core/references/worktree-branch-check.md`, substitute `{EXPECTED_BASE}` with `$EXPECTED_BASE`, and use the resulting `<worktree_branch_check>` block (the runnable guard) as WORKTREE_GUARD below.

**Only when `ISOLATION = "harness-worktree"`.** When `ISOLATION = "none"` the agent runs on
the main working tree, where the guard's HEAD assertion cannot hold — set `WORKTREE_GUARD` to
the empty string instead, or every diagnostic agent halts on a base mismatch it was never
meant to check (#2652).

**Substitute `{harnessFlag}` in the `Agent()` call below** with `$HARNESS_FLAG` followed by a
comma when `ISOLATION = "harness-worktree"`, and with the empty string otherwise — the same
build-time substitution `execute-phase.md` performs. `{harnessFlag}` is a template
placeholder, not a shell variable.

<!-- #2508 runtime-aware-dispatch -->

> **Runtime-aware dispatch (#2508 Phase 4).** GSD workflows dispatch specialized subagents by role. Before dispatching on a built-in-only runtime (kimi-code — three built-ins only), resolve the role to a built-in via the dispatch table (not part of @gsd/core: use the role unchanged). On named-dispatch runtimes (Claude/OpenCode/…) the role is returned unchanged; on kimi-code it maps to `coder`/`explore`/`plan` by role-suffix. The persona rides `${AGENT_SKILLS_<ROLE>}` (Phase 3) regardless. See @gsd-core/references/runtime-aware-dispatch.md.

<!-- #2517 model-omit-on-inherit -->

> **Model omission (#2517).** Omit the `model` parameter entirely when the value it would carry (`DEBUGGER_MODEL`) is `"inherit"` or empty. An empty value 404s on runtimes without native tier aliases — the default on non-Claude runtimes. Omitting it inherits the orchestrator's model. See @gsd-core/references/model-profile-resolution.md.

```
Agent(
  prompt=filled_debug_subagent_prompt + "\n\n" + WORKTREE_GUARD + "\n\n<required_reading>\n- {phase_dir}/{phase_num}-UAT.md\n- {state_path}\n</required_reading>\n${AGENT_SKILLS_DEBUGGER}",
  subagent_type="gsd-debugger",
  model="{DEBUGGER_MODEL}",
  {harnessFlag}
  description="Debug: {truth_short}"
)
```

> **ORCHESTRATOR RULE — CODEX RUNTIME**: After calling Agent() above to spawn debug agent(s), stop working on this task immediately. Do not read more files, edit code, or run tests related to these gaps while the subagent(s) are active. Wait for all subagents to return before proceeding. This prevents duplicate work, conflicting edits, and wasted context.

**All agents spawn in a single message (parallel execution) ONLY when `ISOLATION = "harness-worktree"`.** When `ISOLATION = "none"`, spawn one agent per message and wait for each to return — see the fan-out rule above (#2652).

Template placeholders:
- `{truth}`: The expected behavior that failed
- `{expected}`: From UAT test
- `{actual}`: Verbatim user description from reason field
- `{errors}`: Any error messages from UAT (or "None reported")
- `{reproduction}`: "Test {test_num} in UAT"
- `{timeline}`: "Discovered during UAT"
- `{goal}`: `find_root_cause_only` (UAT flow - plan-phase --gaps handles fixes)
- `{slug}`: Generated from truth
</step>

<step name="collect_results">
**Collect root causes from agents:**

Each agent returns with:
```
## ROOT CAUSE FOUND

**Debug Session:** ${DEBUG_DIR}/{slug}.md

**Root Cause:** {specific cause with evidence}

**Evidence Summary:**
- {key finding 1}
- {key finding 2}
- {key finding 3}

**Files Involved:**
- {file1}: {what's wrong}
- {file2}: {related issue}

**Suggested Fix Direction:** {brief hint for plan-phase --gaps}
```

Parse each return to extract:
- root_cause: The diagnosed cause
- files: Files involved
- debug_path: Path to debug session file
- fix_hint: NON-BINDING example route for the gap closure plan — the binding payload is
  `root_cause`; a gap plan that removes the root cause by a smaller or different mechanism has
  closed the gap in full

If agent returns `## INVESTIGATION INCONCLUSIVE`:
- root_cause: "Investigation inconclusive - manual review needed"
- Note which issue needs manual attention
- Include remaining possibilities from agent return
</step>

<step name="update_uat">
**Update UAT.md gaps with diagnosis:**

For each gap in the Gaps section, add artifacts and missing fields:

```yaml
- truth: "Comment appears immediately after submission"
  status: failed
  reason: "User reported: works but doesn't show until I refresh the page"
  severity: major
  test: 2
  root_cause: "useEffect in CommentList.tsx missing commentCount dependency"
  artifacts:
    - path: "src/components/CommentList.tsx"
      issue: "useEffect missing dependency"
  missing:
    - "Add commentCount to useEffect dependency array"
    - "Trigger re-render when new comment added"
  debug_session: the spec
```

Update status in frontmatter to "diagnosed".

Commit the updated UAT.md:
```bash
git add spec && git commit -m "docs({phase_num}): add root causes from diagnosis" && specforge collect --runner gsd-git --yes
```
</step>

<step name="report_results">
**Report diagnosis results and hand off:**

Display:
```
### GSD ► DIAGNOSIS COMPLETE

| Gap (Truth) | Root Cause | Files |
|-------------|------------|-------|
| Comment appears immediately | useEffect missing dependency | CommentList.tsx |
| Reply button positioned correctly | CSS flex order incorrect | ReplyButton.tsx |
| Delete removes comment | API missing auth header | api/comments.ts |

Debug sessions: ${DEBUG_DIR}/

Proceeding to plan fixes...
```

Return to verify-work orchestrator for automatic planning.
Do NOT offer manual next steps - verify-work handles the rest.
</step>

</process>

<context_efficiency>
Agents start with symptoms pre-filled from UAT (no symptom gathering).
Agents only diagnose—plan-phase --gaps handles fixes (no fix application).
</context_efficiency>

<failure_handling>
**Agent fails to find root cause:**
- Mark gap as "needs manual review"
- Continue with other gaps
- Report incomplete diagnosis

**Agent times out:**
- Check DEBUG-{slug}.md for partial progress
- Can resume with /gsd:debug

**All agents fail:**
- Something systemic (permissions, git, etc.)
- Report for manual investigation
- Fall back to plan-phase --gaps without root causes (less precise)
</failure_handling>

<success_criteria>
- [ ] Gaps parsed from UAT.md
- [ ] Debug agents spawned in parallel
- [ ] Root causes collected from all agents
- [ ] UAT.md gaps updated with artifacts and missing
- [ ] Debug sessions saved to ${DEBUG_DIR}/
- [ ] Hand off to verify-work for automatic planning
</success_criteria>
