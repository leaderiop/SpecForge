<!-- Ported from gsd-core v1.19.0 gsd-core/workflows/validate-phase.md, MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD -->
<purpose>
Audit Nyquist validation gaps for a completed phase. Generate missing tests. Record the validation rows.
</purpose>

<required_reading>
@~/.claude/gsd-core/references/ui-brand.md
</required_reading>

<available_agent_types>
Valid GSD subagent types (use exact names — do not fall back to 'general-purpose'):
- gsd-nyquist-auditor — Validates verification coverage
</available_agent_types>

<process>

## 0. Initialize

```bash

# the commands below are specforge commands: there is no launcher to define
RESPONSE_LANGUAGE="{{setting:response_language|}}"   # response_language (the settings)

INIT=$(specforge query "$PHASE_ID" --depth 1 --format json)

AGENT_SKILLS_AUDITOR=""   # the host install carries the skills of gsd-nyquist-auditor
```

**If `response_language` is set:** All user-facing output of this workflow — narration between tool calls, status updates, progress notes, findings, questions, prompts, and explanations — MUST be presented in `{response_language}`. Technical terms, code, file paths, and subagent prompts stay in English — only user-facing output is translated.

Read from the answer: the node whose `id` is `$PHASE_ID` is the phase (its number, two digits, is the last segment of that id; `name` and `slug` are its fields).

```bash
AUDITOR_MODEL=""   # resolve-model is not part of @gsd/core: the host picks the model
VERIFY_POST_HOOKS_JSON=$(for c in ai codebase ops security ui; do specforge gsd "$c-hooks" verify:post --pick . 2>/dev/null || [ $? -eq 2 ] || echo "⚠ $c-hooks failed: its hooks are left out" >&2; done)   # one compact JSON answer per line, one per capability extension: its activeHooks / skippedHooks are those of every line, in order
```

Resolve active step hooks from `VERIFY_POST_HOOKS_JSON` where `kind == "step"` and `ref.skill == "validate-phase"`.

If no active validate-phase step hook exists: exit with "Nyquist validation is disabled. Enable via /gsd:settings."

Display banner: `GSD > VALIDATE PHASE {N}: {name}`

## 1. Detect Input State

```bash
VALIDATION_FILE=$(specforge query "$PHASE_ID" --depth 1 --kinds validation --pick '.nodes[kind=validation].id')
SUMMARY_FILES=$(specforge query "$PHASE_ID" --depth 2 --kinds summary --pick '.nodes[kind=summary].id')
```

- **State A** (`VALIDATION_FILE` non-empty): Audit existing
- **State B** (`VALIDATION_FILE` empty, `SUMMARY_FILES` non-empty): Reconstruct from artifacts
- **State C** (`SUMMARY_FILES` empty): Exit — "Phase {N} not executed. Run /gsd:execute-phase {N} first."

## 2. Discovery

### 2a. Read Phase Artifacts

Read all PLAN and SUMMARY files. Extract: task lists, requirement IDs, key-files changed, verify blocks.

### 2b. Build Requirement-to-Task Map

Per task: `{ task_id, plan_id, wave, requirement_ids, has_automated_command }`

### 2c. Detect Test Infrastructure

State A: Read the existing validation's `framework`, `quick_cmd` and `full_cmd`, and its rows: `specforge query "$PHASE_ID" --depth 2 --kinds validation,validation_row --format context`.
State B: Filesystem scan:

```bash
specforge files list --kind file --glob pytest.ini --glob 'jest.config.*' --glob 'vitest.config.*' --glob pyproject.toml --max 10 --pick '.entries[].path' 2>/dev/null
specforge files list --glob '*.test.*' --glob '*.spec.*' --glob 'test_*' --max 40 --pick '.entries[].path' 2>/dev/null
```

### 2d. Cross-Reference

Match each requirement to existing tests by filename, imports, test descriptions. Record: requirement → test_file → status.

## 3. Gap Analysis

Classify each requirement:

| Status | Criteria |
|--------|----------|
| COVERED | Test exists, targets behavior, runs green |
| PARTIAL | Test exists, failing or incomplete |
| MISSING | No test found |

Build: `{ task_id, requirement, gap_type, suggested_test_path, suggested_command }`

No gaps → skip to Step 6, set `nyquist_compliant: true`.

## 4. Present Gap Plan

**Text mode (`workflow.text_mode: true` in config or `--text` flag):** Set `TEXT_MODE=true` if `--text` is present in `$ARGUMENTS` OR `text_mode` from init JSON is `true`. When TEXT_MODE is active, replace every `AskUserQuestion` call with a plain-text numbered list and ask the user to type their choice number. This is required for non-Claude runtimes (OpenAI Codex, Antigravity, etc.) where `AskUserQuestion` is not available.
Call AskUserQuestion with gap table and options:
1. "Fix all gaps" → Step 5
2. "Skip — mark manual-only" → add to Manual-Only, Step 6
3. "Cancel" → exit

## 5. Spawn gsd-nyquist-auditor

Print: `◆ Spawning nyquist auditor... (runs in a subagent — no output until it returns, ~1–5 min; expected, not a freeze)`

<!-- #2508 runtime-aware-dispatch -->

> **Runtime-aware dispatch (#2508 Phase 4).** GSD workflows dispatch specialized subagents by role. Before dispatching on a built-in-only runtime (kimi-code — three built-ins only), resolve the role to a built-in via the dispatch table (not part of @gsd/core: use the role unchanged). On named-dispatch runtimes (Claude/OpenCode/…) the role is returned unchanged; on kimi-code it maps to `coder`/`explore`/`plan` by role-suffix. The persona rides `${AGENT_SKILLS_<ROLE>}` (Phase 3) regardless. See @gsd-core/references/runtime-aware-dispatch.md.

<!-- #2517 model-omit-on-inherit -->

> **Model omission (#2517).** Omit the `model` parameter entirely when the value it would carry (`AUDITOR_MODEL`) is `"inherit"` or empty. An empty value 404s on runtimes without native tier aliases — the default on non-Claude runtimes. Omitting it inherits the orchestrator's model. See @gsd-core/references/model-profile-resolution.md.

```
Agent(
  prompt="Read ~/.claude/agents/gsd-nyquist-auditor.md for instructions.\n\n" +
    "<required_reading>specforge context gsd/nyquist-auditor \"{PHASE_ID}\"</required_reading>" +
    "<gaps>{gap list}</gaps>" +
    "<test_infrastructure>{framework, config, commands}</test_infrastructure>" +
    "<constraints>Never modify impl files. Max 3 debug iterations. Escalate impl bugs.</constraints>" +
    "${AGENT_SKILLS_AUDITOR}",
  subagent_type="gsd-nyquist-auditor",
  model="{AUDITOR_MODEL}",
  description="Fill validation gaps for Phase {N}"
)
```

> **ORCHESTRATOR RULE — CODEX RUNTIME**: After calling Agent() above, stop working on this task immediately. Do not read more files, edit code, or run tests related to this task while the subagent is active. Wait for the subagent to return its result. This prevents duplicate work, conflicting edits, and wasted context. Only resume when the subagent result is available.

Handle return:
- `## GAPS FILLED` → record tests + map updates, Step 6
- `## PARTIAL` → record resolved, move escalated to manual-only, Step 6
- `## ESCALATE` → move all to manual-only, Step 6

## 6. Record the Validation Rows

Record the phase's `validation` when it has none (State B), with the detected test infrastructure, then one `validation_row` per task of the map (`task`, `test_type`, the requirement, the test file and its automated command); a row's `status` and the validation's `nyquist_compliant` are computed from the tests. The fence is the validation template's:

```bash
PHASE_ID="{the phase's id, e.g. v1.0/01}"
FRAMEWORK="{the test framework, e.g. vitest}"
IFS= read -rd '' QUICK_CMD <<'TEXT' || true; QUICK_CMD=${QUICK_CMD%$'\n'}
{the quick run command}
TEXT
IFS= read -rd '' FULL_CMD <<'TEXT' || true; FULL_CMD=${FULL_CMD%$'\n'}
{the full suite command}
TEXT
ROW_ID="{R and the row's number, e.g. R1}"
TASK="{the task's id, e.g. v1.0/01/01-01/T1}"
TEST_TYPE="{unit, integration, smoke, e2e or manual}"
REQUIREMENT="{the requirement's id, e.g. AUTH-01}"
TEST_FILE="{the row's test file, ROOT-relative}"
IFS= read -rd '' COMMAND <<'TEXT' || true; COMMAND=${COMMAND%$'\n'}
{the row's automated command}
TEXT
PHASE_FILE=$(specforge query "$PHASE_ID" --depth 0 --pick '.nodes[0].file') || exit 1
VALIDATION=$(specforge query "$PHASE_ID" --depth 1 --kinds validation --pick '.nodes[kind=validation].id' --first 1) || exit 1
if [ -z "$VALIDATION" ]; then
  specforge new validation "${PHASE_ID##*/}-VALIDATION" "Phase ${PHASE_ID##*/} validation strategy" --in "$PHASE_FILE" --phase "$PHASE_ID" --framework "$FRAMEWORK" --quick_cmd "$QUICK_CMD" --full_cmd "$FULL_CMD" || exit 1
  VALIDATION="$PHASE_ID/${PHASE_ID##*/}-VALIDATION"
fi
specforge new validation_row "$ROW_ID" "$TASK" --in "$PHASE_FILE" --validation "$VALIDATION" --task "$TASK" --test_type "$TEST_TYPE" --requirement "$REQUIREMENT" --file "$TEST_FILE" --command "$COMMAND" || exit 1
```

**State A (update):**
1. Add a `validation_row` for each newly covered task with the same fence; an escalated task stays without a row, as manual-only. The person approves the strategy with `specforge gsd approve-validation`, which sets its `status` to validated.
2. Append to the audit trail through the owning verb — it compares against the LAST `## Validation Audit` block and skips the write entirely when nothing material changed (#5105 R3; #4981: idempotence alone is insufficient for VALIDATION, whose content legitimately changes with the plan/SUMMARY set, so this call is made AFTER the Per-Task Map update above, not instead of it):

```bash
: "verification.append-audit: record the audit in the validation entity (specforge gsd record-validation)" \
  --heading "Validation Audit" \
  --rows "{\"Gaps found\": {N}, \"Resolved\": {M}, \"Escalated\": {K}}"
```

## 7. Commit

```bash
specforge git commit --add {each test file, one --add each} \
  -m "test(phase-${PHASE}): add Nyquist validation tests"

specforge git commit --add spec -m "docs(phase-${PHASE}): add/update validation strategy" && specforge collect --runner gsd-git --yes

```

## 8. Results + Routing

**Compliant:**
```
GSD > PHASE {N} IS NYQUIST-COMPLIANT
All requirements have automated verification.
▶ Next: /gsd:audit-milestone
```

**Partial:**
```
GSD > PHASE {N} VALIDATED (PARTIAL)
{M} automated, {K} manual-only.
▶ Retry: /gsd:validate-phase {N}
```

Display `/clear` reminder.

</process>

<success_criteria>
- [ ] Nyquist config checked (exit if disabled)
- [ ] Input state detected (A/B/C)
- [ ] State C exits cleanly
- [ ] PLAN/SUMMARY files read, requirement map built
- [ ] Test infrastructure detected
- [ ] Gaps classified (COVERED/PARTIAL/MISSING)
- [ ] User gate with gap table
- [ ] Auditor spawned with complete context
- [ ] All three return formats handled
- [ ] `validation` and its `validation_row` entities recorded
- [ ] Test files committed separately
- [ ] Results with routing presented
</success_criteria>
