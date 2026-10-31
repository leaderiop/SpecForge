<!-- Ported from gsd-core v1.18.0 gsd-core/workflows/secure-phase.md, MIT, Copyright (c) 2026 Open GSD; see gsd/security/LICENSE-GSD -->
<purpose>
Verify threat mitigations for a completed phase. Confirm the plans' threat dispositions are resolved. Update the phase's SECURITY.md, the security audit.
</purpose>

<required_reading>
@~/.claude/gsd-core/references/ui-brand.md
</required_reading>

<available_agent_types>
Valid GSD subagent types (use exact names — do not fall back to 'general-purpose'):
- gsd-security-auditor — Verifies threat mitigation coverage
</available_agent_types>

<process>

## 0. Initialize

```bash
IFS= read -rd '' GSD_ARGS <<'GSD_ARGS_END' || true
$ARGUMENTS
GSD_ARGS_END
GSD_ARGS=${GSD_ARGS%$'\n'}
# the commands below are specforge commands: there is no launcher to define
RESPONSE_LANGUAGE="{{setting:response_language|}}"   # response_language (the settings)
PHASE_ARG=$(echo "$GSD_ARGS" | sed -E 's/--ws[[:space:]]+[A-Za-z0-9._-]+//g' | xargs)
GSD_WS=$(echo " $GSD_ARGS" | sed -nE 's/.* --ws +([A-Za-z0-9][A-Za-z0-9._-]*).*/--ws \1/p' | head -n 1)
INIT=$(specforge query "$PHASE_ID" --depth 1 --format json)
if [[ "$INIT" == @file:* ]]; then INIT=$(cat "${INIT#@file:}"); fi
AGENT_SKILLS_AUDITOR=""   # the host install carries the skills of gsd-security-auditor
```

**If `response_language` is set:** All user-facing output of this workflow — narration between tool calls, status updates, progress notes, findings, questions, prompts, and explanations — MUST be presented in `{response_language}`. Technical terms, code, file paths, and subagent prompts stay in English — only user-facing output is translated.

Parse: `phase_dir`, `phase_number`, `phase_name`, `phase_slug`, `padded_phase`.

```bash
AUDITOR_MODEL=""   # resolve-model is not part of @gsd/security: the host picks the model
VERIFY_POST_HOOKS_JSON=$(for c in ai codebase ops security ui; do specforge gsd "$c-hooks" verify:post --format json 2>/dev/null; done | jq -sc '{activeHooks: (map(.activeHooks) | add // []), skippedHooks: (map(.skippedHooks) | add // [])}')
SECURITY_ASVS="{{setting:workflow.security_asvs_level|1}}"   # workflow.security_asvs_level
SECURITY_BLOCK_ON="{{setting:workflow.security_block_on|high}}"   # workflow.security_block_on
```

Resolve active step hooks from `VERIFY_POST_HOOKS_JSON` where `kind == "step"` and `ref.skill == "secure-phase"`.

If no active secure-phase step hook exists: exit with "Security enforcement disabled. Enable via /gsd:settings."

Display banner: `GSD > SECURE PHASE {N}: {name}`

## 1. Detect Input State

```bash
AUDIT=$(specforge query "$PHASE_ID" --kinds security_audit --format json | jq -r '.nodes[]|select(.kind=="security_audit")|.id')
PLAN_FILES=$(specforge query "$PHASE_ID" --kinds plan --format json | jq -r '.nodes[]|select(.kind=="plan")|.id')
SUMMARY_FILES=$(specforge query "$PHASE_ID" --depth 2 --kinds summary --format json | jq -r '.nodes[]|select(.kind=="summary")|.id')
```

- **State A** (`AUDIT` non-empty): Audit existing
- **State B** (`AUDIT` empty, `PLAN_FILES` and `SUMMARY_FILES` non-empty): Run from artifacts
- **State C** (`SUMMARY_FILES` empty): Exit — "Phase {N} not executed. Run /gsd:execute-phase {N} first."

## 2. Discovery

### 2a. Read Phase Artifacts

Read the plans' threat model: `specforge query "$PHASE_ID" --depth 2 --kinds trust_boundary,threat --format json` — trust boundaries (`trust_boundary`), STRIDE register (`threat`: `category`, `component`, `severity`, `disposition`, `mitigation`, `files`; the `threat_id` is the entity's name, `T-03-01`).

### 2b. Read Summary Threat Flags

Read the summaries — extract their `threat_flags`.

### 2c. Build Threat Register

Per threat: `{ threat_id, category, component, severity, disposition, mitigation_pattern, files_to_check }`

Also set `register_authored_at_plan_time: true` if **at least one** `threat` of the phase's plans was authored at plan time (it has no `retroactive: true`); `false` if no plan holds a `threat` authored at plan time (legacy phase authored before formal threat modelling was standard).

## 3. Threat Classification

Classify each threat:

| Status | Criteria |
|--------|----------|
| CLOSED | mitigation found OR accepted risk documented in SECURITY.md OR transfer documented |
| OPEN | none of the above |

Build: `{ threat_id, category, component, severity, disposition, status, evidence }`

**Short-circuit rule:**
- If `threats_open: 0 AND register_authored_at_plan_time: true AND asvs_level == 1` → skip to Step 6 directly. No open threats at or above the block threshold remain (threats_open: 0); below-threshold open threats may remain and are non-blocking. L1 grep-depth is sufficient; no deeper verification required.
- If `threats_open: 0 AND register_authored_at_plan_time: true AND asvs_level >= 2` → **do NOT skip**. The preliminary threat classification is grep-level (L1 depth) and is insufficient for L2/L3. Proceed to Step 5 (spawn the auditor) so that L2 boundary-placement checks and L3 end-to-end trace checks are performed. Skipping the auditor here would defeat ASVS level scaling for "clean" phases.
- If `threats_open: 0 AND register_authored_at_plan_time: false` → **do NOT skip**. Empty-by-no-planning must not rubber-stamp a clean SECURITY.md. Proceed to Step 5 in **retroactive-STRIDE mode** — the auditor builds a register from implementation files first, then verifies mitigations.
- If `threats_open > 0` → proceed to Step 4 (present threat plan to user).

## 4. Present Threat Plan

**Text mode (`workflow.text_mode: true` in config or `--text` flag):** Set `TEXT_MODE=true` if `--text` is present in `$ARGUMENTS` OR `text_mode` from init JSON is `true`. When TEXT_MODE is active, replace every `AskUserQuestion` call with a plain-text numbered list and ask the user to type their choice number. This is required for non-Claude runtimes (OpenAI Codex, Antigravity, etc.) where `AskUserQuestion` is not available.
Call AskUserQuestion with threat table and options:
1. "Verify all open threats" → Step 5
2. "Accept all open — document in accepted risks log" → ask the person to run `accept_risk` for each (accepted risks log); an accepted threat leaves `threats_open` by itself, so do not record it closed. Step 6
3. "Cancel" → exit

## 5. Spawn gsd-security-auditor

**Auditor constraint — varies by register origin:**

- `register_authored_at_plan_time: true` — **Verify mitigations exist** — do not scan for new threats. The register is complete; verify each threat's mitigation is present in the implementation.
- `register_authored_at_plan_time: false` (retroactive-STRIDE mode) — **Retroactive-STRIDE: build a STRIDE register from implementation files first, then verify mitigations.** The phase was authored before formal threat modelling; the auditor must construct the register from scratch before verifying.

Substitute `{SECURITY_ASVS}` with the value of `$SECURITY_ASVS` and `{SECURITY_BLOCK_ON}` with the value of `$SECURITY_BLOCK_ON` resolved in Step 0 from the settings.

Print: `◆ Spawning security auditor... (runs in a subagent — no output until it returns, ~1–5 min; expected, not a freeze)`

<!-- #2508 runtime-aware-dispatch -->

> **Runtime-aware dispatch (#2508 Phase 4).** GSD workflows dispatch specialized subagents by role. Before dispatching on a built-in-only runtime (kimi-code — three built-ins only), resolve the role to a built-in via the dispatch table (not part of @gsd/security: use the role unchanged). On named-dispatch runtimes (Claude/OpenCode/…) the role is returned unchanged; on kimi-code it maps to `coder`/`explore`/`plan` by role-suffix. The persona rides `${AGENT_SKILLS_<ROLE>}` (Phase 3) regardless. See @gsd-core/references/runtime-aware-dispatch.md.

<!-- #2517 model-omit-on-inherit -->

> **Model omission (#2517).** Omit the `model` parameter entirely when the value it would carry (`AUDITOR_MODEL`) is `"inherit"` or empty. An empty value 404s on runtimes without native tier aliases — the default on non-Claude runtimes. Omitting it inherits the orchestrator's model. See @gsd-core/references/model-profile-resolution.md.

```
Agent(
  prompt="Read ~/.claude/agents/gsd-security-auditor.md for instructions.\n\n" +
    "<required_reading>{PLAN, SUMMARY, impl files, SECURITY.md}</required_reading>" +
    "<threat_register>{threat register}</threat_register>" +
    "<config>asvs_level: {SECURITY_ASVS}, block_on: {SECURITY_BLOCK_ON}</config>" +
    "<constraints>Never modify implementation files. Verify mitigations exist — do not scan for new threats. Escalate implementation gaps. Return a structured verdict only — do NOT write SECURITY.md (the orchestrator owns the file write).</constraints>" +
    "${AGENT_SKILLS_AUDITOR}",
  subagent_type="gsd-security-auditor",
  model="{AUDITOR_MODEL}",
  description="Verify threat mitigations for Phase {N}"
)
```

> **ORCHESTRATOR RULE — CODEX RUNTIME**: After calling Agent() above, stop working on this task immediately. Do not read more files, edit code, or run tests related to this task while the subagent is active. Wait for the subagent to return its result. This prevents duplicate work, conflicting edits, and wasted context. Only resume when the subagent result is available.

Handle return:
- `## SECURED` → record closures → Step 6
- `## OPEN_THREATS` → record closed + open, present user with accept/block choice → Step 6
- `## ESCALATE` → present to user → Step 6

## 6. Write/Update SECURITY.md

**State B (create):**
1. Create the audit with `record_security_audit` (below) when the phase has none; the template's columns are the fields of the `security_audit`, `threat`, `accepted_risk` and `audit_run` entities (the template ends this text)
2. Fill: frontmatter, threat register, accepted risks, audit trail
3. The command writes the audit, the threats' statuses and the audit trail

**State A (update):**
1. Update threat register statuses.
2. Append to the audit trail through the owning command, `record_security_audit` — it compares against the LAST `## Security Audit` block and skips the write entirely when nothing material changed (#5105 R3; #4887 Defect 2: an unchanged register no longer grows a new identical row on every re-run). A verdict id is a threat's ID; `T-{N}-SC` sets the supply-chain threat of every plan of the phase, and `<plan>/T-{N}-02` (the plan's name, then the ID) names one of two plans' threats that claim the same ID:

```bash
specforge gsd record-security-audit "$PHASE_ID" \
  --verdict '{"threats": [{"id": "T-{N}-01", "status": "closed", "evidence": "{file}"}]}' \
  --run-by "gsd-security-auditor"
```

**ENFORCING GATE:** If `threats_open > 0` after all options exhausted (user did not accept, not all verified closed):

```
GSD > PHASE {N} SECURITY BLOCKED
{K} blocking threats open — phase advancement blocked until threats_open: 0
▶ Fix mitigations then re-run: /gsd:secure-phase {N}
▶ Or ask the person to run `accept_risk` for the risks they accept, and re-run.
```

Do NOT emit next-phase routing. Stop here.

## 7. Commit

```bash
git add spec && git commit -m "docs(phase-${PHASE}): add/update security threat verification" && specforge collect --runner gsd-git --yes; : \
  --files "spec"
```

## 8. Results + Routing

**Secured (threats_open: 0):**
```
GSD > PHASE {N} THREAT-SECURE
threats_open: 0 — no blocking threats remain (threats_open: 0).
▶ /gsd:validate-phase {N}    validate test coverage
▶ /gsd:verify-work {N}       run UAT
```

Display `/clear` reminder.

</process>

<success_criteria>
- [ ] Security enforcement checked — exit if false
- [ ] Input state detected (A/B/C) — state C exits cleanly
- [ ] PLAN.md threat model parsed, register built
- [ ] SUMMARY.md threat flags incorporated
- [ ] threats_open: 0 AND register_authored_at_plan_time: true AND asvs_level == 1 → skip directly to Step 6 (L1 grep-depth sufficient)
- [ ] threats_open: 0 AND register_authored_at_plan_time: true AND asvs_level >= 2 → do NOT skip; auditor spawned for L2/L3 deep verification
- [ ] threats_open: 0 AND register_authored_at_plan_time: false → retroactive-STRIDE mode (Step 5), not skipped
- [ ] User gate with threat table presented
- [ ] Auditor spawned with complete context
- [ ] All three return formats (SECURED/OPEN_THREATS/ESCALATE) handled
- [ ] SECURITY.md created or updated
- [ ] threats_open > 0 BLOCKS advancement (no next-phase routing emitted)
- [ ] Results with routing presented on success
</success_criteria>
