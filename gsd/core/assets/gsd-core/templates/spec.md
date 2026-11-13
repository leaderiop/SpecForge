<!-- Ported from gsd-core v1.19.0 gsd-core/templates/spec.md, MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD -->
# Phase Spec Template

Template for the `phase_spec` entity of phase XX-name — locks requirements before discuss-phase.

**Purpose:** Capture WHAT a phase delivers and WHY, with enough precision that requirements are falsifiable. discuss-phase reads this spec and focuses on HOW to implement (skipping "what/why" questions already answered here).

**Key principle:** Every requirement must be falsifiable — you can write a test or check that proves it was met or not. Vague requirements like "improve performance" are not allowed.

**Downstream consumers:**
- `discuss-phase` — reads the spec at startup; treats Requirements, Boundaries, and Acceptance Criteria as locked; skips "what/why" questions
- `gsd-planner` — reads locked requirements to constrain plan scope
- `gsd-verifier` — uses acceptance criteria as explicit pass/fail checks

---

## The Spec's Typed Edits

A phase's spec is entities in `spec/discuss.spec`: one `phase_spec`, one `locked_req` per requirement and one
`interview` per question of the interview, written with `specforge new` when the ambiguity gate passes; nothing writes
a spec by hand. Its `edge` and `spec_prohibition` rows are recorded as they are resolved (spec-phase Steps 5.5 and 5.6).
`specforge schema --kind phase_spec` (likewise `locked_req`, `interview`, `edge`, `spec_prohibition`) lists the fields,
and `specforge new phase_spec --help` prints them as flags. A list of texts takes its flag once per item (`--in_scope
"Backup command" --in_scope "Compression"`). Fill the values: free text in its quoted heredoc, as written, and an id, enum or date in its assignment; leave out a flag you have no value for, and repeat
a `new locked_req` (with its `link … traces` line) or `new interview` line per item with the next id (`LR2`, `IV2`).
The spec's `scores` are the judged verdict `specforge gsd record-spec` asks for, never written here. A refusal names its
code: read it with `specforge explain <code>`.

```bash
PHASE_ID="{the phase's id, e.g. v1.0/03}"
PADDED_PHASE="{the phase number, zero-padded, e.g. 03}"
IFS= read -rd '' NAME <<'TEXT' || true; NAME=${NAME%$'\n'}
{the phase name}
TEXT
CREATED="{today, YYYY-MM-DD}"
IFS= read -rd '' GOAL <<'TEXT' || true; GOAL=${GOAL%$'\n'}
{one precise sentence: what is true when this phase is done, not a feature list}
TEXT
IFS= read -rd '' BACKGROUND <<'TEXT' || true; BACKGROUND=${BACKGROUND%$'\n'}
{what exists today and why this phase is needed, grounded in the codebase scout}
TEXT
IFS= read -rd '' IN_SCOPE <<'TEXT' || true; IN_SCOPE=${IN_SCOPE%$'\n'}
{what this phase produces: one --in_scope per item}
TEXT
IFS= read -rd '' OUT_OF_SCOPE <<'TEXT' || true; OUT_OF_SCOPE=${OUT_OF_SCOPE%$'\n'}
{what this phase explicitly does not do, with brief reasoning: one --out_of_scope per item}
TEXT
IFS= read -rd '' CONSTRAINT <<'TEXT' || true; CONSTRAINT=${CONSTRAINT%$'\n'}
{a performance, compatibility, data volume or dependency constraint: one --constraints per item}
TEXT
IFS= read -rd '' ACCEPTANCE <<'TEXT' || true; ACCEPTANCE=${ACCEPTANCE%$'\n'}
{a pass/fail criterion: one --acceptance per item}
TEXT
IFS= read -rd '' REQ_TITLE <<'TEXT' || true; REQ_TITLE=${REQ_TITLE%$'\n'}
{the requirement's short label}
TEXT
IFS= read -rd '' CURRENT <<'TEXT' || true; CURRENT=${CURRENT%$'\n'}
{what exists now}
TEXT
IFS= read -rd '' TARGET <<'TEXT' || true; TARGET=${TARGET%$'\n'}
{what it should become: not improve X but X becomes Y}
TEXT
IFS= read -rd '' REQ_ACCEPTANCE <<'TEXT' || true; REQ_ACCEPTANCE=${REQ_ACCEPTANCE%$'\n'}
{the falsifiable check: how a verifier confirms it was met}
TEXT
TRACES="{the project requirement it refines, e.g. TASK-03}"
ROUND="{the interview round}"
PERSPECTIVE="{researcher, simplifier, boundary_keeper or failure_analyst}"
IFS= read -rd '' QUESTION <<'TEXT' || true; QUESTION=${QUESTION%$'\n'}
{the question asked}
TEXT
IFS= read -rd '' ANSWER <<'TEXT' || true; ANSWER=${ANSWER%$'\n'}
{the answer}
TEXT
IFS= read -rd '' LOCKED <<'TEXT' || true; LOCKED=${LOCKED%$'\n'}
{what the answer locked}
TEXT
SPEC_ID="$PHASE_ID/${PADDED_PHASE}-SPEC"
specforge new phase_spec "${PADDED_PHASE}-SPEC" "$NAME spec" --in spec/discuss.spec --phase "$PHASE_ID" --created "$CREATED" --goal "$GOAL" --background "$BACKGROUND" --in_scope "$IN_SCOPE" --out_of_scope "$OUT_OF_SCOPE" --constraints "$CONSTRAINT" --acceptance "$ACCEPTANCE" || exit 1
specforge new locked_req LR1 "$REQ_TITLE" --in spec/discuss.spec --spec "$SPEC_ID" --title "$REQ_TITLE" --current "$CURRENT" --target "$TARGET" --acceptance "$REQ_ACCEPTANCE" || exit 1
specforge link "$SPEC_ID/LR1" traces "$TRACES" || exit 1
specforge new interview IV1 "$QUESTION" --in spec/discuss.spec --spec "$SPEC_ID" --round "$ROUND" --perspective "$PERSPECTIVE" --question "$QUESTION" --answer "$ANSWER" --locked "$LOCKED" || exit 1
```

**What goes where:**

- `goal`: one precise sentence. `background`: the current state, from the codebase scout.
- `locked_req`: one specific, testable statement each, with its `current`, `target` and `acceptance`; `traces` names
  the project requirement it refines, when one does.
- `in_scope` / `out_of_scope`: explicit lists; an out-of-scope item carries its reasoning.
- `constraints`: performance, compatibility, data volume, dependency or platform constraints.
- `acceptance`: pass/fail criteria. A specified edge and a kept prohibition each append a line here
  (`specforge link "$SPEC_ID" acceptance "<the line>"`).
- `interview`: one per question, its `round` and `perspective`; an answer, once written, is never rewritten.
- `edge`: one row per `(requirement, category)`, recorded by `specforge gsd resolve-edge`; never `specforge new edge`.
- `spec_prohibition`: one per must-NOT a requirement needs, `status` resolved, dismissed or unresolved, a resolved row
  with its `verification` tier (`test` or `judgment`) and, for `test`, its `check_*` descriptor when known.
- The ambiguity report is the judged verdict's `scores`: a dimension below its minimum is flagged there for the
  planner to treat as an assumption.

<guidelines>
**Every requirement needs all three fields:**
- Current: grounds the requirement in reality — what exists today?
- Target: the concrete change — not "improve X" but "X becomes Y"
- Acceptance: the falsifiable check — how does a verifier confirm this?

**The ambiguity scores must reflect the actual interview.** A dimension below its minimum is flagged ⚠ in the judged verdict — the planner knows to treat it as an assumption rather than a locked requirement.

**The interviews are evidence of rigor.** Don't skip it. It shows that requirements came from discovery, not assumption.

**Boundaries protect the phase from scope creep.** The out-of-scope list with reasoning is as important as the in-scope list. Future phases that touch adjacent areas can point to this spec to understand what was intentionally excluded.

**The spec is a one-way door for requirements.** discuss-phase treats them as locked. If requirements change after the
spec is recorded, the user changes the spec first (`specforge set` / `specforge link` on its entities), then re-runs
discuss-phase.

**The spec does not replace the context.** They serve different purposes:
- the `phase_spec`: what the phase delivers (requirements, boundaries, acceptance criteria)
- the `context`: how the phase will be implemented (decisions, patterns, tradeoffs)

discuss-phase records the context after reading the spec.
</guidelines>
