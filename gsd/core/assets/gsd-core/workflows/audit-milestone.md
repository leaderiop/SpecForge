<!-- Ported from gsd-core v1.19.0 gsd-core/workflows/audit-milestone.md, MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD -->
@~/.claude/gsd-core/references/response-language-directive.md

<purpose>
Verify the milestone achieved its definition of done: read its requirements along their threads (requirement → plans → tasks → commits → tests → truths) and the phases' verifications, record the cross-phase integration verdict on the milestone's audit, and record its tech debt. The audit's gaps, Nyquist counts and status are computed from the graph.
</purpose>

<required_reading>
Read all files referenced by the invoking prompt's execution_context before starting.
</required_reading>

<available_agent_types>
Valid GSD subagent types (use exact names — do not fall back to 'general-purpose'):
- gsd-integration-checker — Checks cross-phase integration
</available_agent_types>

<process>

## 0. Initialize Milestone Context

```bash

# the commands below are specforge commands: there is no launcher to define

INIT=$(specforge query "$MILESTONE" --depth 1 --format json)

AGENT_SKILLS_CHECKER=""   # the host install carries the skills of gsd-integration-checker
```

Read from the answer: the node whose `id` is `$MILESTONE` is the milestone (its version is that id; `name` and `progress` are its fields, and `phase_count` and `completed_phases` are its `progress`'s `phases` and `phases_complete`), and its phases are among the other nodes. `commit_docs` is a setting: `specforge settings --format json --extension @gsd/core`.

Resolve integration checker model:
```bash
integration_checker_model=""   # resolve-model is not part of @gsd/core: the host picks the model
```

## 1. Determine Milestone Scope

```bash
# Get phases in milestone (sorted numerically, handles decimals)
specforge query "$MILESTONE" --kinds phase --format json
```

The version is `$MILESTONE`. Its phases are the answer's `phase` nodes, its definition of done is the milestone's `goal` and the phases' `success_criteria`, and its requirements are the phases' `requirements`.

Resolve the milestone's audit, the `milestone_audit` entity this workflow records into. A re-audit reuses it; the first audit creates `<milestone>/A1`, before the integration checker is spawned, since its verdict is recorded on it:

```bash
AUDIT=$(specforge query "$MILESTONE" --depth 1 --kinds milestone_audit --pick '.nodes[kind=milestone_audit].id' --last 1) || exit 1
if [ -z "$AUDIT" ]; then
  MILESTONE_FILE=$(specforge query "$MILESTONE" --depth 0 --pick '.nodes[0].file') || exit 1
  [ -n "$MILESTONE_FILE" ] || { echo "no milestone $MILESTONE" >&2; exit 1; }
  specforge new milestone_audit A1 "$MILESTONE audit" --in "$MILESTONE_FILE" --milestone "$MILESTONE" || exit 1
  AUDIT="$MILESTONE/A1"
fi
echo "AUDIT=$AUDIT"
```

The audit's fields are listed by `specforge schema --kind milestone_audit`: `gaps`, `nyquist` and `status` are computed, `integration` is the checker's verdict, `tech_debt` is written in step 6, and `debt_accepted` is the person's.

## 2. Read the Requirement Threads

Trace every requirement of the milestone's phases along its thread, and read the phases' verifications. A thread that falls short prints a `no <step>` marker where it stops; a truth prints its verdict state:

```bash
specforge trace --route gsd/requirement-thread --in "$MILESTONE" --partial
specforge query "$MILESTONE" --depth 2 --kinds verification --format context
```

From each phase's verification and the threads of its requirements, note:
- **Status:** passed | gaps_found
- **Critical gaps:** (if any — these are blockers)
- **Non-critical gaps:** tech debt, deferred items, warnings
- **Anti-patterns found:** TODOs, stubs, placeholders
- **Requirements coverage:** which requirements satisfied/blocked

A phase with no verification is an "unverified phase": a blocker.

## 3. Spawn Integration Checker

With phase context collected:

The checker reads its pack, the audit's `gsd/integration-checker` route: the audit, the milestone's phases, their verifications and the requirements they map.

Print: "Spawning integration checker (runs in a subagent — no output until it returns, ~1–5 min; expected, not a freeze)"

<!-- #2517 model-omit-on-inherit -->

> **Model omission (#2517).** Omit the `model` parameter entirely when the value it would carry (`integration_checker_model`) is `"inherit"` or empty. An empty value 404s on runtimes without native tier aliases — the default on non-Claude runtimes. Omitting it inherits the orchestrator's model. See @gsd-core/references/model-profile-resolution.md.

```
Agent(
  prompt="Check cross-phase integration and E2E flows.

<required_reading>
specforge context gsd/integration-checker "{AUDIT}"
Run the line above first. Its pack is the audit, the milestone's phases, their verifications and the requirements they map, at the integration-checker route's budget.
</required_reading>

MUST map each integration finding to affected requirement IDs where applicable.

<!-- #2508 runtime-aware-dispatch -->

> **Runtime-aware dispatch (#2508 Phase 4).** GSD workflows dispatch specialized subagents by role. Before dispatching on a built-in-only runtime (kimi-code — three built-ins only), resolve the role to a built-in via the dispatch table (not part of @gsd/core: use the role unchanged). On named-dispatch runtimes (Claude/OpenCode/…) the role is returned unchanged; on kimi-code it maps to `coder`/`explore`/`plan` by role-suffix. The persona rides `${AGENT_SKILLS_<ROLE>}` (Phase 3) regardless. See @gsd-core/references/runtime-aware-dispatch.md.

Verify cross-phase wiring and E2E user flows.
${AGENT_SKILLS_CHECKER}",
  subagent_type="gsd-integration-checker",
  model="{integration_checker_model}"
)
```

> **ORCHESTRATOR RULE — CODEX RUNTIME**: After calling Agent() above, stop working on this task immediately. Do not read more files, edit code, or run tests related to this task while the subagent is active. Wait for the subagent to return its result. This prevents duplicate work, conflicting edits, and wasted context. Only resume when the subagent result is available.

### 3b. Record the Integration Verdict

The checker's finding is the judged rule `gsd/E518`, and the rule judges the milestone's audit. Once the checker returns, read the rule's inputs, then record the verdict once, on the audit (unless the checker's return says it recorded it there itself): the outcome is `passed` or `issues`, the basis is the `basis` object of the inputs' answer copied verbatim, and each integration gap the checker found is a finding.

```bash
AUDIT=$(specforge query "$MILESTONE" --depth 1 --kinds milestone_audit --pick '.nodes[kind=milestone_audit].id' --last 1) || exit 1
[ -n "$AUDIT" ] || { echo "the milestone has no audit: run step 1 first" >&2; exit 1; }
specforge query "$AUDIT" --inputs gsd/E518 --format json
```

```bash
AUDIT=$(specforge query "$MILESTONE" --depth 1 --kinds milestone_audit --pick '.nodes[kind=milestone_audit].id' --last 1) || exit 1
[ -n "$AUDIT" ] || { echo "the milestone has no audit: run step 1 first" >&2; exit 1; }
OUTCOME="{passed or issues}"
IFS= read -rd '' BASIS <<'TEXT' || true; BASIS=${BASIS%$'\n'}
{the basis object of the inputs' answer, verbatim}
TEXT
IFS= read -rd '' FINDINGS <<'TEXT' || true; FINDINGS=${FINDINGS%$'\n'}
[{"severity": "{error or warning}", "message": "{an integration gap the checker found; [] when it found none}"}]
TEXT
[ -n "$BASIS" ] || { echo "no basis: read the rule's inputs first" >&2; exit 1; }
specforge gsd record-verdict "$AUDIT" E518 "$OUTCOME" gsd-integration-checker "$BASIS" --findings "$FINDINGS" || exit 1
```

A refusal names its code: read it with `specforge explain <code>`. A `stale_revision` refusal means the inputs changed since they were read: read them again.

## 4. Collect Results

Combine:
- Phase-level gaps and tech debt (from step 2)
- Integration checker's report (wiring gaps, broken flows)

## 5. Check Requirements Coverage (3-Source Cross-Reference)

MUST cross-reference three independent sources for each requirement:

### 5a. The Requirements and Their Phases

The milestone's requirements are its phases' `requirements`: each with its id, text (`specforge query <requirement> --depth 0 --format context`), assigned phase and computed `status` (`pending` or `complete`).

### 5b. The Threads' Verification State

For each requirement, its thread (step 2) names the plans that claim it, their tests, and the truths with their verdict state; its phase's verification gives the phase's `status`. The audit's computed `gaps` already name each requirement that is `unsatisfied`, `partial` or `orphaned`:

```bash
AUDIT=$(specforge query "$MILESTONE" --depth 1 --kinds milestone_audit --pick '.nodes[kind=milestone_audit].id' --last 1) || exit 1
[ -n "$AUDIT" ] || { echo "the milestone has no audit: run step 1 first" >&2; exit 1; }
specforge query "$AUDIT" --depth 0 --pick '.nodes[0].fields.gaps'
```

### 5c. Extract SUMMARY.md Frontmatter Cross-Check

For each phase's SUMMARY.md, extract `requirements-completed` from YAML frontmatter:
```bash
specforge query "$MILESTONE" --depth 3 --kinds summary --pick '.nodes[kind=summary]' --fields id,fields.requirements_completed   # a row per summary: its id, then its requirement ids as a JSON list (cells split at 0x1f)
```

### 5d. Status Determination Matrix

For each REQ-ID, determine status using all three sources:

| Phase verification `status` | Summary `requirements_completed` | Requirement `status` | → Final Status |
|-----------------------------|----------------------------------|----------------------|----------------|
| passed                      | listed                           | complete             | **satisfied**  |
| passed                      | listed                           | pending              | **satisfied** (its status follows once the thread closes) |
| passed                      | missing                          | any                  | **partial** (verify manually) |
| gaps_found                  | any                              | any                  | **unsatisfied** |
| no verification             | listed                           | any                  | **partial** (verification gap) |
| no verification             | missing                          | any                  | **unsatisfied** |

### 5e. FAIL Gate and Orphan Detection

The audit's `status` is computed: an `unsatisfied` or `orphaned` requirement among its `gaps` makes it `gaps_found`, whatever the matrix's other rows say.

**Orphan detection:** a requirement a phase maps but no plan claims (its thread stops at `no plan`) is `orphaned` in the audit's `gaps`, and counts as `unsatisfied`: it was assigned but never planned, so no phase verified it.

## 5.5. Nyquist Compliance Discovery

Skip if the Nyquist capability is inactive.

```bash
VERIFY_POST_HOOKS_JSON=$(for c in ai codebase ops security ui; do specforge gsd "$c-hooks" verify:post --pick . 2>/dev/null || [ $? -eq 2 ] || echo "⚠ $c-hooks failed: its hooks are left out" >&2; done)   # one compact JSON answer per line, one per capability extension: its activeHooks / skippedHooks are those of every line, in order
```

Resolve active step hooks from `VERIFY_POST_HOOKS_JSON` where `kind == "step"` and `ref.skill == "validate-phase"`.

If no active validate-phase step hook exists: skip entirely.

The audit counts the phases by their Nyquist state in its computed `nyquist`, from each phase's `validation` entity:

```bash
AUDIT=$(specforge query "$MILESTONE" --depth 1 --kinds milestone_audit --pick '.nodes[kind=milestone_audit].id' --last 1) || exit 1
[ -n "$AUDIT" ] || { echo "the milestone has no audit: run step 1 first" >&2; exit 1; }
specforge query "$AUDIT" --depth 0 --pick '.nodes[0].fields.nyquist'
specforge query "$MILESTONE" --depth 2 --kinds validation --format context
```

| Status | Condition |
|--------|-----------|
| COMPLIANT (`compliant`) | the phase's validation is approved and its rows cover every task |
| PARTIAL (`partial`) | approved, with tasks its rows leave uncovered or red |
| NOT-VALIDATED (`not_validated`) | a validation no one approved yet: validate-phase has not reconciled it, a coverage TODO, not a compliance failure |
| MISSING (`missing`) | the phase has no validation |

Discovery only — never auto-calls `/gsd:validate-phase`.

## 6. Record the Tech Debt

The audit's `gaps`, `nyquist` and `status` are computed, and its `integration` is the verdict of step 3b; what this step writes is the tech debt: one `{phase, item}` object per non-critical, deferred item of steps 2 and 4, as one JSON list (`[]` when there is none). Accepting the debt is the person's (`debt_accepted`), never this workflow's.

```bash
AUDIT=$(specforge query "$MILESTONE" --depth 1 --kinds milestone_audit --pick '.nodes[kind=milestone_audit].id' --last 1) || exit 1
[ -n "$AUDIT" ] || { echo "the milestone has no audit: run step 1 first" >&2; exit 1; }
IFS= read -rd '' TECH_DEBT <<'TEXT' || true; TECH_DEBT=${TECH_DEBT%$'\n'}
[{"phase": "{the phase's id, e.g. v1.0/01}", "item": "{one deferred item, e.g. TODO: add rate limiting}"}]
TEXT
specforge set "$AUDIT" tech_debt "$TECH_DEBT" || exit 1
specforge query "$AUDIT" --depth 0 --pick '.nodes[0].fields.status'
```

The report presented in step 7 is read from the audit (`specforge query "$AUDIT" --depth 0 --format context`): its requirements, phases, integration and tech debt.

**Status values:**
- `passed` — all requirements met, no critical gaps, minimal tech debt
- `gaps_found` — critical blockers exist
- `tech_debt` — no blockers but accumulated deferred items need review

## 7. Present Results

Route by status (see `<offer_next>`).

</process>

<offer_next>
Output this markdown directly (not as a code block). Route based on status:

---

**If passed:**

## ✓ Milestone {version} — Audit Passed

**Score:** {N}/{M} requirements satisfied
**Report:** the spec

All requirements covered. Cross-phase integration verified. E2E flows complete.

---

## ▶ Next Up — [${PROJECT_CODE}] ${PROJECT_TITLE}

**Complete milestone** — archive and tag

/clear then:

/gsd:complete-milestone {version}

---

---

**If gaps_found:**

## ⚠ Milestone {version} — Gaps Found

**Score:** {N}/{M} requirements satisfied
**Report:** the spec

### Unsatisfied Requirements

{For each unsatisfied requirement:}
- **{REQ-ID}: {description}** (Phase {X})
  - {reason}

### Cross-Phase Issues

{For each integration gap:}
- **{from} → {to}:** {issue}

### Broken Flows

{For each flow gap:}
- **{flow name}:** breaks at {step}

### Nyquist Coverage

| Phase | VALIDATION.md | Compliant | Action |
|-------|---------------|-----------|--------|
| {phase} | exists/missing | true/false/partial | `/gsd:validate-phase {N}` |

Phases needing validation: run `/gsd:validate-phase {N}` for each flagged phase.

---

## ▶ Next Up — [${PROJECT_CODE}] ${PROJECT_TITLE}

**Close the gaps inline** — gap planning happens as part of this audit's
output (see the Unsatisfied Requirements, Cross-Phase Issues, Broken Flows,
and Nyquist Coverage sections above). Insert one closure phase per gap (or
per group of related gaps) using the standard phase chain:

/clear then:

/gsd:phase --insert <after-phase> "Close gap: <REQ-ID> — <description>"
/gsd:discuss-phase <new-phase>
/gsd:plan-phase <new-phase>
/gsd:execute-phase <new-phase>

`<after-phase>` is the phase the closure phase goes after. `<new-phase>` is
the number the insert prints for the phase it creates (e.g. `5.1` after `5`,
or `5.2` if `5.1` exists), never `<after-phase>`.

For Nyquist-coverage gaps flagged in the table above, prefer running
`/gsd:validate-phase <N>` for each flagged phase (and `/gsd:secure-phase
<N>` if the security audit was flagged) before inserting a new closure phase —
they may close the gap retroactively without a new phase.

---

**Also available:**
- cat the spec — see full report
- /gsd:complete-milestone {version} — proceed anyway (accept tech debt)

---

---

**If tech_debt (no blockers but accumulated debt):**

## ⚡ Milestone {version} — Tech Debt Review

**Score:** {N}/{M} requirements satisfied
**Report:** the spec

All requirements met. No critical blockers. Accumulated tech debt needs review.

### Tech Debt by Phase

{For each phase with debt:}
**Phase {X}: {name}**
- {item 1}
- {item 2}

### Total: {N} items across {M} phases

---

## ▶ Options

**A. Complete milestone** — accept debt, track in backlog

/gsd:complete-milestone {version}

**B. Plan a cleanup phase** — address the debt above before completing.
Insert a closure phase using the standard chain:

/clear then:

/gsd:phase --insert <after-phase> "Address tech debt: <area>"
/gsd:discuss-phase <new-phase>
/gsd:plan-phase <new-phase>
/gsd:execute-phase <new-phase>

`<after-phase>` is the phase the cleanup phase goes after. `<new-phase>` is
the number the insert prints for the phase it creates (e.g. `5.1` after `5`,
or `5.2` if `5.1` exists), never `<after-phase>`.

---
</offer_next>

<success_criteria>
- [ ] Milestone scope identified
- [ ] The milestone's audit resolved, or created as `<milestone>/A1`, before the checker's spawn
- [ ] Every requirement traced along its thread, and every phase's verification read
- [ ] The summaries' `requirements_completed` read for each phase
- [ ] 3-source cross-reference completed (threads and verifications, summaries, requirement status)
- [ ] Orphaned requirements read from the audit's computed `gaps`
- [ ] Tech debt and deferred gaps aggregated
- [ ] Integration checker spawned with the audit's context line (`specforge context gsd/integration-checker`)
- [ ] The integration verdict (`gsd/E518`) recorded once, on the audit
- [ ] The audit's `tech_debt` recorded; its `status` read (an unsatisfied requirement makes it `gaps_found`)
- [ ] Nyquist compliance read from the audit's `nyquist` (if enabled)
- [ ] Phases with no validation flagged with validate-phase suggestion
- [ ] Results presented with actionable next steps
</success_criteria>
