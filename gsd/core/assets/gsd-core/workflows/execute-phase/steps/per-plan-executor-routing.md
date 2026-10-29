<!-- Ported from gsd-core v1.18.0 gsd-core/workflows/execute-phase/steps/per-plan-executor-routing.md, MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD -->
# Per-plan executor routing (#1689)

Run for each plan, immediately before its `Agent()` dispatch in step 3. Sets
`EXECUTOR_TYPE` so a plan can opt into a specialist executor instead of the
default `gsd-executor`. `plan_json` (the current plan's object from
`phase-plan-index`, same scope step 2.5 uses) is in scope.

## Contract

- Default: `EXECUTOR_TYPE="gsd-executor"` — byte-identical to pre-#1689 dispatch.
- A plan opts into a specialist by declaring `agent_hint: <name>` in its PLAN.md
  frontmatter. The field reaches the orchestrator as `plan_json.agent_hint`
  (parsed by `phase-plan-index`; `null` when unset).
- When routing is enabled AND the hint is non-empty AND the named agent resolves
  on the active runtime, `EXECUTOR_TYPE` becomes the hint. Otherwise it stays
  `gsd-executor`.
- The resolved `EXECUTOR_TYPE` is used as `subagent_type` in BOTH worktree and
  sequential dispatch (sequential reuses the worktree-mode `Agent()` template).

## Resolution

```bash
# the commands below are specforge commands: there is no launcher to define
# Default-on; opt out with: gsd config-set workflow.agent_hint_routing false
AGENT_HINT_ROUTING="{{setting:workflow.agent_hint_routing|true}}"   # workflow.agent_hint_routing (the settings)

EXECUTOR_TYPE="gsd-executor"
if [ "${AGENT_HINT_ROUTING:-true}" != "false" ]; then
  PLAN_HINT=$(jq -r '.agent_hint // empty' <<<"$plan_json" 2>/dev/null | tr -d '"')
  if [ -n "$PLAN_HINT" ]; then
    EXECUTOR_TYPE="gsd-executor"   # resolve-agent is not part of @gsd/core: the agent hint names a core agent or falls back to this
  fi
fi

# #1689 v1 routes only the Agent()-based dispatch. On the orchestrator-worktree
# backend (process-spawn; no subagent_type) a resolved hint cannot be honored
# yet — surface it so a set hint is never silently ignored.
if [ "${ISOLATION:-}" = "orchestrator-worktree" ] && [ -n "${PLAN_HINT:-}" ]; then
  echo "note: plan ${plan_id} agent_hint='${PLAN_HINT}' resolved, but orchestrator-worktree dispatch does not route subagent types in this release — using the default executor." >&2
fi
```

Resolving a plan's `agent_hint` (not part of @gsd/core) would consult the **active runtime's agent directory**
(both project-local and user-global, across runtime filename variants — `.md`,
`.agent.md`, `.toml`, the kimi `subagents/<name>.{yaml,md}` pair) and fails
closed to `gsd-executor` when the named agent does not resolve or on any error,
so a missing or misspelled hint never blocks dispatch.

## Scope

Routing applies to the `Agent()`-based dispatch (harness-worktree and sequential
modes). The `orchestrator-worktree` isolation backend spawns executors via a
separate process path that has no `subagent_type` and is not routed in this
release.

## Checkpoint gate rule (#3370)

Loaded with the routing resolution so the orchestrator reads it immediately
before composing each dispatch prompt, in every isolation mode.

On `checkpoint:human-verify` / `checkpoint:decision` tasks, `gate="blocking"`
(the default) is auto-approvable in auto-mode — that is the executor's own
`<checkpoint_protocol>` (`agents/gsd-executor.md`), and `checkpoints.md` (the
full gate table) is embedded in the dispatch `<execution_context>` verbatim.
Only `gate="blocking-human"` always surfaces to a human, regardless of
auto-mode. An unmet `<precondition>` checkpoint (executor step 0, `Blocked by:
Precondition not met` — unmet `user_setup` step, missing env var, absent
prior-phase artifact) reports `blocking-human` and therefore always surfaces
to a human, in every mode (#3210): the missing prerequisite is a fact only a
human can establish, not a verification step to rubber-stamp.

When composing the `Agent()` prompt, do NOT add text refusing or overriding
auto-approval for a `blocking` gate. Orchestrator-composed instructions that contradict the
executor's protocol win the executor's attention, stall autonomous runs at the
checkpoint, and defeat `_auto_chain_active`/auto-advance for the common case.
Executor-side gate semantics are already complete; compose nothing about gates
beyond what the template already embeds.
