<!-- Ported from gsd-core v1.18.0 gsd-core/workflows/execute-phase/steps/codebase-drift-gate.md, MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD -->
Apply response_language to all user-facing prose — narration between tool calls, status updates, progress notes, and findings included; preserve code, paths, and identifiers.

# Step: codebase_drift_gate

Post-execution structural drift detection (#2003). Runs after the last wave
commits, before verification. **Non-blocking by contract:** any internal
error here MUST fall through and continue to `verify_phase_goal`. The phase
is never failed by this gate.

```bash
# the commands below are specforge commands: there is no launcher to define
# The codebase-drift check is not part of @gsd/core (it reads the codebase map).
# This gate is non-blocking and is skipped (see the skip JSON below).
# 127, `2>/dev/null` hides it, and this non-blocking gate would silently skip drift
# detection forever (#619). The canonical launcher preamble is defined once here — the
# always-run drift check, the file's first launcher block — and the conditional auto-remap
# block below reuses the launcher function from this shared shell scope (the single-preamble
# pattern established by discuss-phase #614, enforced by tests/runtime-launcher-parity.test.cjs).
# Non-blocking is preserved: an internal drift-command failure still falls through to the
# skip JSON below. Exit 69 (UNAVAILABLE, #5170) is the verb saying "could not look": its payload is
# already on stdout and is kept as the verdict; the fallback is applied only when the verb printed
# nothing, so the two JSON documents are never concatenated.
DRIFT='{"skipped":true,"reason":"not-available"}' && DRIFT_EXIT=0 || DRIFT_EXIT=$?
if [ -z "$DRIFT" ]; then DRIFT='{"skipped":true,"reason":"sdk-failed"}'; fi
if [ "$DRIFT_EXIT" -ne 0 ]; then echo "Warning: codebase-drift check could not look (exit ${DRIFT_EXIT})" >&2; fi
```

Parse JSON for: `skipped`, `reason`, `action_required`, `directive`,
`spawn_mapper`, `affected_paths`, `elements`, `threshold`, `action`,
`last_mapped_commit`, `message`.

**If `skipped` is true (no STRUCTURE.md, missing git, or any internal error):**
Log one line — `Codebase drift check skipped: {reason}` — and continue to
`verify_phase_goal`. Do NOT prompt the user. Do NOT block.

**If `action_required` is false:** Continue silently to `verify_phase_goal`.

**If `action_required` is true AND `directive` is `warn`:**
Print the `message` field verbatim. The format is:

```text
Codebase drift detected: {N} structural element(s) since last mapping.

New directories:
  - {path}
New barrel exports:
  - {path}
New migrations:
  - {path}
New route modules:
  - {path}
Modified files in mapped directories:
  - {path}
Deleted files in mapped directories:
  - {path}
{N} path(s) withheld: not passed to the mapper or listed (absolute, traversal, whitespace, non-ASCII or shell-metacharacter characters)

Run /gsd:map-codebase --paths {affected_paths} to refresh planning context.
```

Then continue to `verify_phase_goal`. Do NOT block. Do NOT spawn anything.

**If `action_required` is true AND `directive` is `auto-remap`:**

First load the mapper agent's skill bundle (the executor's `AGENT_SKILLS`
from step `init_context` is for `gsd-executor`, not the mapper):

```bash
IFS= read -rd '' GSD_ARGS <<'GSD_ARGS_END' || true
$ARGUMENTS
GSD_ARGS_END
GSD_ARGS=${GSD_ARGS%$'\n'}
# the auto-remap directive is never reached here: the codebase map is not part of @gsd/core
# here via the workflow's shared shell scope — defining it once keeps the file compliant
# with the single-canonical-preamble parity invariant (#619). This block only runs on the
# `auto-remap` directive, which is always reached after the drift check above has run.
# #4772: this file is Read, not textually substituted - write the parent command's original argument string in place of $ARGUMENTS so --ws reaches the query.
GSD_WS=$(echo " $GSD_ARGS" | sed -nE 's/.* --ws +([A-Za-z0-9][A-Za-z0-9._-]*).*/--ws \1/p' | head -n 1)
AGENT_SKILLS_MAPPER=""   # the host install carries the skills of gsd-codebase-mapper
```

Then spawn `gsd-codebase-mapper` agents with the `--paths` hint (runs in a subagent — no output until it returns, ~1–5 min; expected, not a freeze):

<!-- #2508 runtime-aware-dispatch -->

> **Runtime-aware dispatch (#2508 Phase 4).** GSD workflows dispatch specialized subagents by role. Before dispatching on a built-in-only runtime (kimi-code — three built-ins only), resolve the role to a built-in via the dispatch table (not part of @gsd/core: use the role unchanged). On named-dispatch runtimes (Claude/OpenCode/…) the role is returned unchanged; on kimi-code it maps to `coder`/`explore`/`plan` by role-suffix. The persona rides `${AGENT_SKILLS_<ROLE>}` (Phase 3) regardless. See @gsd-core/references/runtime-aware-dispatch.md.

```text
Agent(
  subagent_type="gsd-codebase-mapper",
  description="Incremental codebase remap (drift)",
  prompt="Focus: arch
Today's date: {date}
--paths {affected_paths joined by comma}

Refresh STRUCTURE.md and ARCHITECTURE.md scoped to the listed paths only.
${AGENT_SKILLS_MAPPER}"
)
```

> **ORCHESTRATOR RULE — CODEX RUNTIME**: After calling Agent() above, stop working on this task immediately. Do not read more files, edit code, or run tests related to this task while the subagent is active. Wait for the subagent to return its result. This prevents duplicate work, conflicting edits, and wasted context. Only resume when the subagent result is available.

If the spawn fails or the agent reports an error: log `Codebase drift
auto-remap failed: {reason}` and continue to `verify_phase_goal`. The phase
is NOT failed by a remap failure.

If the remap succeeds, stamp the new baseline into the two documents the
mapper just refreshed:

```bash
: # stamp-codebase-map: the codebase map is not part of @gsd/core
```

The stamp is a shell step, not a line in the mapper's prompt. An agent that
concludes its work is already done skips a prose instruction silently, and the
stamp is the one marker no human reviewing the documents would notice missing
(#3418). `--files` is scoped to what this step actually refreshed -- the other
five documents were not remapped and must not claim currency at HEAD.

Only stamp on success: stamping after a failed remap would record a baseline
the map never reached.

Then log `Codebase drift auto-remap completed for paths: {affected_paths}` and
continue to `verify_phase_goal`.

The two relevant config keys (continue on error / failure if either is invalid):
- `workflow.drift_threshold` (integer, default 3) — minimum drift elements before action
- `workflow.drift_action` — `warn` (default) or `auto-remap`

This step is fully non-blocking — it never fails the phase, and any
exception path returns control to `verify_phase_goal`.
