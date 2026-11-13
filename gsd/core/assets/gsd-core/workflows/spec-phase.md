<!-- Ported from gsd-core v1.19.0 gsd-core/workflows/spec-phase.md, MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD -->
<purpose>
Clarify WHAT a phase delivers through a Socratic interview loop with quantitative ambiguity scoring.
Records the phase spec: falsifiable requirements that discuss-phase treats as locked decisions.

This workflow handles "what" and "why" — discuss-phase handles "how".
</purpose>

<ambiguity_model>
Score each dimension 0.0 (completely unclear) to 1.0 (crystal clear):

| Dimension         | Weight | Minimum | What it measures                                  |
|-------------------|--------|---------|---------------------------------------------------|
| Goal Clarity      | 35%    | 0.75    | Is the outcome specific and measurable?           |
| Boundary Clarity  | 25%    | 0.70    | What's in scope vs out of scope?                  |
| Constraint Clarity| 20%    | 0.65    | Performance, compatibility, data requirements?    |
| Acceptance Criteria| 20%   | 0.70    | How do we know it's done?                         |

**Ambiguity score** = 1.0 − (0.35×goal + 0.25×boundary + 0.20×constraint + 0.20×acceptance)

**Gate:** ambiguity ≤ 0.20 AND all dimensions ≥ their minimums → ready to record the spec.

A score of 0.20 means 80% weighted clarity — enough precision that the planner won't silently make wrong assumptions.
</ambiguity_model>

<interview_perspectives>
Rotate through these perspectives — each naturally surfaces different blindspots:

**Researcher (rounds 1–2):** Ground the discussion in current reality.
- "What exists in the codebase today related to this phase?"
- "What's the delta between today and the target state?"
- "What triggers this work — what's broken or missing?"

**Simplifier (round 2):** Surface minimum viable scope.
- "What's the simplest version that solves the core problem?"
- "If you had to cut 50%, what's the irreducible core?"
- "What would make this phase a success even without the nice-to-haves?"

**Boundary Keeper (round 3):** Lock the perimeter.
- "What explicitly will NOT be done in this phase?"
- "What adjacent problems is it tempting to solve but shouldn't?"
- "What does 'done' look like — what's the final deliverable?"

**Failure Analyst (round 4):** Find the edge cases that invalidate requirements.
- "What's the worst thing that could go wrong if we get the requirements wrong?"
- "What does a broken version of this look like?"
- "What would cause a verifier to reject the output?"

**Seed Closer (rounds 5–6):** Lock remaining undecided territory.
- "We have [dimension] at [score] — what would make it completely clear?"
- "The remaining ambiguity is in [area] — can we make a decision now?"
- "Is there anything you'd regret not specifying before planning starts?"
</interview_perspectives>

<process>

## Step 1: Initialize

```bash
# the commands below are specforge commands: there is no launcher to define
INIT=$(specforge query "$PHASE_ID" --depth 1 --format json)

```

Read from the answer: the node whose `id` is `$PHASE_ID` is the phase (its number, two digits, is the last segment of that id; `name`, `slug` and `requirements` are its fields; a phase not in the graph is an E003 error); the project state is computed and the roadmap and the requirements are entities, so no path is read. `response_language` and `commit_docs` are settings: `specforge settings --format json --extension @gsd/core` (the phase's spec is read below, from the graph).

**If `response_language` is set:** All user-facing text in this workflow — narration between tool calls, status updates, progress notes, findings, questions, and report prose — MUST be in `{response_language}`. Technical terms, code, and file paths stay in English.

**If `phase_found` is false:**
```
Phase [X] not found in roadmap.
Use /gsd:progress to see available phases.
```
Exit.

**Check for an existing spec:** the phase spec is the phase's `phase_spec` entity in the graph (an AI spec or a UI spec is never one); read it, never glob for a file: `SPEC_ID=$(specforge query "$PHASE_ID" --depth 1 --pick '.nodes[kind=phase_spec].id' --first 1); HAS_SPEC=$([ -n "$SPEC_ID" ] && echo true || echo false)`. Below, `has_spec` is `$HAS_SPEC` and the existing spec is the entity `$SPEC_ID`.

If `has_spec` is true:

**If `--auto`:** Auto-select "Skip" — leave the existing spec unchanged and exit with the same
message the interactive "Skip" prints. Log: `[auto] the phase spec exists — reusing as-is.` An unattended
run reuses an existing artifact rather than regenerating it (#4776): "Update it" re-scores and
rewrites the spec, discarding answers a person already recorded in it, and nobody is present to
notice.

**Otherwise:** Use AskUserQuestion:
- header: "Spec"
- question: "Phase [X] already has a spec. What do you want to do?"
- options:
  - "Update it" — Revise and re-score
  - "View it" — Show current spec
  - "Skip" — Exit (use existing spec as-is)

If "View": show it with `specforge query "$SPEC_ID" --depth 1 --format context`, then offer Update/Skip.
If "Skip": Exit with message: "Existing spec unchanged. Run /gsd:discuss-phase [X] to continue."
If "Update": Load the existing spec the same way, continue to Step 3; its fields change with `specforge set` / `specforge link`, and an interview answer once written is never rewritten.

## Step 2: Scout Codebase

**Read these files before any questions:**
- `{requirements_path}` — Project requirements
- `{state_path}` — Decisions already made, current phase, blockers
- ROADMAP.md phase entry — Phase description, goals, canonical refs

**Grep the codebase** for code/files relevant to this phase goal. Look for:
- Existing implementations of similar functionality
- Integration points where new code will connect
- Test coverage gaps relevant to the phase
- Prior phase artifacts (SUMMARY.md, VERIFICATION.md) that inform current state

**Synthesize current state** — the grounded baseline for the interview:
- What exists today related to this phase
- The gap between current state and the phase goal
- The primary deliverable: what file/behavior/capability does NOT exist yet?

Confirm your current state synthesis internally. Do not present it to the user yet — you'll use it to ask precise, grounded questions.

## Step 3: First Ambiguity Assessment

Before questioning begins, score the phase's current ambiguity based only on what ROADMAP.md and REQUIREMENTS.md say:

```
Goal Clarity:       [score 0.0–1.0]
Boundary Clarity:   [score 0.0–1.0]
Constraint Clarity: [score 0.0–1.0]
Acceptance Criteria:[score 0.0–1.0]

Ambiguity: [score] ([calculate])
```

**If `--auto` and initial ambiguity already ≤ 0.20 with all minimums met:** No interview: the spec is drawn straight from the phase and its requirements. Log: `[auto] Requirements already clear: recording the spec from the existing context.` Jump to Step 5.5.

**Otherwise:** Continue to Step 4.

## Step 4: Socratic Interview Loop

**Max 6 rounds.** Each round: 2–3 questions max. End round after user responds.

**Round selection by perspective:**
- Round 1: Researcher
- Round 2: Researcher + Simplifier
- Round 3: Boundary Keeper
- Round 4: Failure Analyst
- Rounds 5–6: Seed Closer (focus on lowest-scoring dimensions)

**After each round:**
1. Update all 4 dimension scores from the user's answers
2. Calculate new ambiguity score
3. Display the updated scoring:

```
After round [N]:
  Goal Clarity:       [score] (min 0.75) [✓ or ↑ needed]
  Boundary Clarity:   [score] (min 0.70) [✓ or ↑ needed]
  Constraint Clarity: [score] (min 0.65) [✓ or ↑ needed]
  Acceptance Criteria:[score] (min 0.70) [✓ or ↑ needed]
  Ambiguity: [score] (gate: ≤ 0.20)
```

**Gate check after each round:**

If gate passes (ambiguity ≤ 0.20 AND all minimums met):

**If `--auto`:** Jump to Step 5.5.

**Otherwise:** AskUserQuestion:
- header: "Spec Gate Passed"
- question: "Ambiguity is [score] — requirements are clear enough to record the spec. Proceed?"
- options:
  - "Yes — record the spec" → Jump to Step 5.5
  - "One more round" → Continue interview
  - "Done talking — record it" → Jump to Step 5.5

**If max rounds reached (6) and gate not passed:**

**If `--auto`:** Record the spec anyway — the judged verdict flags the dimensions below their minimum. Log: `[auto] Max rounds reached. Recording the spec with [N] dimensions below minimum. Planner will need to treat these as assumptions.`

**Otherwise:** AskUserQuestion:
- header: "Max Rounds"
- question: "After 6 rounds, ambiguity is [score]. [List dimensions still below minimum.] What would you like to do?"
- options:
  - "Record the spec anyway — flag gaps" → Jump to Step 5.5; the judged verdict's scores carry the dimensions below their minimum
  - "Keep talking" → Continue (no round limit from here)
  - "Abandon" → Exit without recording anything

**If `--auto` mode throughout:** Replace all AskUserQuestion calls above with Claude's recommended choice. Log decisions inline. Apply the same logic as `--auto` in discuss-phase.

**Text mode (`workflow.text_mode: true` or `--text` flag):** Use plain-text numbered lists instead of AskUserQuestion TUI menus.

## Step 5: (covered inline — ambiguity scoring is per-round)

## Step 5.5: Edge-Completeness Probe

Run AFTER the ambiguity gate passes (you probe edges of clear requirements, not vague
ones). Reference: @~/.claude/gsd-core/references/edge-probe.md.

**Non-English projects — `text_en` carries the classifier-facing translation; the SPEC is
not.** The shape cues the classifier matches are **English** word-boundary patterns, so
requirement prose written in another language matches nothing, classifies to zero shapes, and
lands every row in `unclassified` (#1110) — the taxonomy contributes nothing and `--auto`
leaves it all `unresolved`. When this project has `response_language` set, set the optional
`text_en` field of each `locked_req`: a faithful **English** translation of that
requirement (`specforge set <LR> text_en "<English>"`). `text_en` is **engine input, never
user-facing output**, so the `response_language` rule at the top of this workflow does not govern it — but the locked
requirement's own fields are NOT translated: keep them as the SPEC states them.
The SPEC keeps the original language — only `text_en` is translated, and requirement
`id`s are never translated or renumbered (coverage rows join back on `id`, and any Acceptance
Criteria you write from the resolved edges go into the SPEC in `response_language`). Populate
`text_en` for **every** requirement, not only the ones that look edge-relevant: the
`coverage.applicable = 0` warning below fires only when *all* requirements are unclassified, so a
partly-classified spec slips through with no signal at all. (#4656) The warning now ALSO
fires on the all-unclassified case itself: `coverage.unclassified` counts the soft-signal
rows, and the guard below fires when `coverage.unclassified` equals `coverage.applicable` — the case where the
classifier learned nothing about ANY requirement. When `response_language` is unset
(an English-language project), omit `text_en` — `text` is already English and the engine
falls back to it automatically (`text_en ?? text`).
If a requirement still classifies to zero shapes with `text_en` populated, it carries no cue in
any language (the recorded recall gap — ADR-857 §98 / ADR-550 D7b, not a translation failure);
set that requirement's `shapes` instead of relying on the prose classifier (`specforge set <LR>
shapes collection text`, any of `numeric_range`, `collection`, `text`, `stateful`, `io`).

Before the probe, the spec and its requirements are entities: when `$SPEC_ID` is empty, read the spec template with
`specforge gsd reference gsd-core/templates/spec.md` and run its fence (the `phase_spec`, one `locked_req` per
requirement gathered and one `interview` per question asked), then set `SPEC_ID="$PHASE_ID/${PADDED_PHASE}-SPEC"`.
The probe is always given the spec, never the phase.

**Runtime coverage compute — `specforge gsd edge-probe`:**

```bash
SPEC_ID=$(specforge query "$PHASE_ID" --depth 1 --pick '.nodes[kind=phase_spec].id' --first 1)
if [ -z "$SPEC_ID" ]; then
  echo "ERROR: record the phase spec and its locked requirements before the edge probe — it reads them from the graph." >&2
  exit 1
fi
# Invoke the engine and CAPTURE its report — it computes which categories apply per
# requirement. The report is RENDERED into context below (#3102); its resolved/dismissed/
# unresolved rows (resolved items carry verification: explicit|backstop) are the deterministic
# FLOOR the resolution loop consumes and unions with its own classification — the loop no longer
# re-derives the taxonomy from prose unaided. Floor, never ceiling: the classifier has a measured
# recall gap (ADR-857 §98 / ADR-550 D7b), so the model still ADDS any category the engine missed.
# The engine FAILS CLOSED (exit 1) on an invalid authored shape or bad input — so the capture
# MUST be exit-checked. A bare capture swallows that exit code, leaves $COVERAGE
# empty, and lets the workflow fall through to prose re-derivation: fail-OPEN at the boundary
# the engine validation exists to protect. Make the run fatal, then validate the captured
# report is well-formed JSON before the resolution loop consumes it.
if ! COVERAGE=$(specforge gsd edge-probe "$SPEC_ID" --format json); then
  echo "ERROR: edge-probe engine failed (invalid shapes or bad input) — fix the requirement(s) and re-run; never proceed with empty coverage." >&2
  exit 1
fi
# Exit-0-but-garbage guard: the report must parse as JSON with the expected { items[], coverage{} } shape.
if [ -z "$COVERAGE" ]; then
  echo "ERROR: edge-probe produced an unparseable or malformed coverage report — refusing to proceed with the resolution loop." >&2
  exit 1
fi
# Render the validated report into the model's visible context (#3102). Until here $COVERAGE was
# captured, shape-checked, and reduced to coverage.applicable — the engine's per-requirement
# items[] never reached the model, so the resolution loop below re-derived edge categories from
# requirement PROSE (the data-flow twin of #2733's control-flow discard). These rows are the
# deterministic FLOOR the resolution loop consumes. Printed RAW (not a bespoke table) so this
# step holds NO knowledge of the item schema: an ADR-550 D7a-style re-cut of the item/coverage
# shape cannot silently desync a hand-rolled renderer here — the engine stays the single source.
echo "### Edge-probe coverage report (deterministic proposals — the FLOOR for the resolution loop below):"
printf '%s\n' "$COVERAGE"
echo "### (end edge-probe coverage report)"
# Zero-applicable guard: a report where the engine proposed NO applicable edge across ANY
# requirement is far more likely a shape-classification miss (or malformed requirements) than
# a genuinely edge-free spec — the same fail-open shape as an invalid shape yielding
# applicable:0. Surface it loudly; the author must explicitly confirm "no applicable edges"
# below rather than silently emitting a green empty ## Edge Coverage section.
# Read coverage.applicable and coverage.unclassified from the report printed above, and check its { items[], coverage{} } shape:

# #4656: all-unclassified is the accidental-miss case the applicable:0 guard could never
# reach — #1110's soft-signal rows make applicable non-zero by construction there.

```

If the report's `coverage.applicable` is `0`, or every requirement is unclassified (`coverage.unclassified` equals it, #4656), do NOT proceed silently: ask the author to confirm via AskUserQuestion
("The edge probe found no applicable edges for any requirement — is this genuinely an
edge-free spec, or should we revisit the requirement wording / authored shapes?"). Only write
an empty `## Edge Coverage` section after explicit confirmation.

For each Requirement gathered so far:
1. Start from the edge-probe rows RENDERED above — the deterministic `items[]` are the FLOOR:
   every proposed `(requirement_id, category)` MUST be resolved below (Specify / Dismiss-with-
   reason / Backstop / Defer), none silently dropped. Then raise any applicable category the
   engine MISSED — the rows are a floor, never a ceiling: the classifier has a measured recall
   gap on terse prose (ADR-857 §98 / ADR-550 D7b), e.g. a CSV-export requirement whose
   `encoding` edge the shape cue under-fires. Union the engine's rows with your own
   classification (relevance filter — see the taxonomy in the reference); do not narrow to them.
   Reuse any edges the Round-4 Failure Analyst already surfaced as pre-resolved.
2. For each raised category, propose a CONCRETE candidate edge (not "consider
   boundaries" — e.g. "R2 merges intervals; what about `[[1,2],[2,3]]` that only touch?").
3. Resolve each with the user (AskUserQuestion; text mode → numbered list):
   - **Specify it** → the edge is `resolved` with `verification: explicit`, and its pass/fail line joins the spec's
     acceptance criteria.
   - **Dismiss (reason)** → `dismissed` with a required non-empty reason.
   - **Backstop with a test** → `resolved` with `verification: backstop`; note "held-out edge test" for plan-phase.
   - **Defer** → leave `unresolved`.

   Record each row as it is resolved, one call per row: `resolve-edge` keeps one row per `(requirement, category)`,
   creating or updating it, so a row is never written with `specforge new edge`. Leave out a flag the outcome has no
   value for (`--resolution` for a dismissal or a deferral, `--reason` unless dismissed, `--verification` unless
   resolved); the acceptance line runs only for **Specify it**:

   ```bash
   LR_ID="{the locked requirement, e.g. $SPEC_ID/LR1}"
   CATEGORY="{boundary, adjacency, empty, encoding, ordering, precision, idempotency, concurrency or unclassified}"
   STATUS="{resolved, dismissed or unresolved}"
   VERIFICATION="{explicit or backstop}"
   IFS= read -rd '' RESOLUTION <<'TEXT' || true; RESOLUTION=${RESOLUTION%$'\n'}
   {the pass/fail line, or the backstop note}
   TEXT
   IFS= read -rd '' REASON <<'TEXT' || true; REASON=${REASON%$'\n'}
   {why the edge is dismissed}
   TEXT
   specforge gsd resolve-edge "$LR_ID" "$CATEGORY" "$STATUS" --verification "$VERIFICATION" --resolution "$RESOLUTION" --reason "$REASON" || exit 1
   specforge link "$SPEC_ID" acceptance "$RESOLUTION" || exit 1
   ```
   - An `unclassified` row (probe `unclassified — review manually`) means the requirement's
     prose matched no shape cue (#1110) — treat it like any other candidate (**Specify**,
     **Dismiss (reason)**, or **Defer**). A manual-review nudge, not a hard block.

**Soft gate (after resolving):** the probe counts what is left; a proposal never recorded counts as unresolved, so
`--auto` records nothing for a row it leaves:

```bash
UNRESOLVED=$(specforge gsd edge-probe "$SPEC_ID" --pick .coverage.unresolved) || exit 1
```

- All applicable edges resolved → proceed to Step 5.6.
- Any `unresolved` → AskUserQuestion:
  - header: "Edge Coverage"
  - question: "[N] edge(s) are unresolved: [list]. What do you want to do?"
  - options: "Resolve now" (loop back) / "Record the spec anyway — flag unresolved" /
    "Keep probing"
  - On "anyway": the rows stay `unresolved`, which marks them `⚠ Edge unresolved — planner
    must treat as assumption`.

**`--auto` mode:** resolve over the **same rendered floor** (#3102) — every engine-proposed row
from Step 5.5's report (step 1) plus any category the classifier missed, never a narrower set.
For each: auto-`resolved` (verification: explicit) where a defensible acceptance criterion can be
written; otherwise auto-`resolved` (verification: backstop) (never auto-dismiss — a wrong
dismissal is the exact silent failure being eliminated). Log:
`[auto] edge coverage: E explicit, B backstop, U unresolved`.

**`unclassified` exception (#1110):** `--auto` leaves an `unclassified` candidate
**`unresolved`** (the soft gate surfaces it as a flagged planner assumption) — it never
auto-resolves it with `verification: backstop`. A missing shape is not evidence an edge exists, so minting a held-out
edge obligation on a requirement that may be genuinely edge-free would be a false claim and
risks a vacuous edge test. Leaving it `unresolved` keeps the zero-cue requirement visible
(never a silent drop) without fabricating an edge — which is exactly #1110's purpose: surface
it for review, do not auto-handle it.

The edge rows recorded above are the spec's edge coverage: nothing else is written.

## Step 5.6: Prohibition-Completeness Probe (must-NOT)

Run AFTER Step 5.5 (you probe the must-NOT axis of clear requirements, over the same
requirement list). Reference: @~/.claude/gsd-core/references/prohibition-probe.md — the
portable two-stage protocol, the canon-referral rule, and the status×verification schema
live there (size-cap discipline; keep this step lean).

**D1 — no compiled engine (ADR-550 D7b).** Unlike Step 5.5, the prohibition probe has NO
compiled recall engine and runs NO `node` invocation here. The recall stage is an LLM prose
pass: the closed eight-category edge taxonomy a classifier can apply does not exist for the
open values/safety/ethics must-NOT axis. Do NOT copy the Step 5.5 engine-resolution block.
Only the schema/projection layer is real code; the recall is prose.

For each Requirement gathered so far, run the two-stage recall→precision pass:

1. **Stage 1 — Recall (adversarial prose probe).** Ask the single adversarial question of the
   requirement: *"What could this feature silently become that the author would NOT want, but
   the spec does not forbid?"* Over-produce (~10 raw must-NOT candidates) — recall first.
2. **Stage 2 — Precision (one-pass classifier).** Filter the raw list in a single pass:
   **DROP routine-engineering** items (normal correctness/hygiene — "must not mutate input",
   "must not throw on empty" — owned by the edge probe or code review); **KEEP
   values / safety / ethics** items (manipulative framing, protected-attribute proxies, raw
   PII in plaintext). This collapses ~10 → ~2–3 genuine prohibitions.
3. **Canon-referral (ADR-550 D6, PROB-13).** A kept candidate that is canon security/compliance
   (OWASP / prototype-pollution / path-traversal / injection / GDPR / generic fairness) is
   NOT minted here — emit a one-line breadcrumb (*"prototype-pollution is canon — owned by
   /gsd:secure-phase + eslint; not minted here"*) and DROP it. Minting canon items duplicates
   /gsd:secure-phase and drowns the bespoke signal.
4. **Resolve each surfaced (non-canon) prohibition** (AskUserQuestion; text mode → numbered list):
   - **Keep it** → the prohibition is `resolved` with a verification tier: `test` (a mechanical negative
     test/lint/assertion exists) or `judgment` (real but not mechanically checkable — routes to judgment review),
     and its NEGATIVE acceptance criterion (a must-NOT line) joins the spec's acceptance criteria.
     - **Capture the wired-check descriptor on `test`-tier (#1278, SOFT).** When a prohibition is
       resolved `verification: test`, ALSO capture the descriptor of the wired check so
       `verify-phase` can LOCATE it deterministically (no verifier invention at verify time).
       Capture the flat scalars, the `check_*` fields of the `spec_prohibition`:
       - `check_kind` — `node-test` | `lint-rule`.
       - `check_target` — the negative-test file path (for `node-test`), or the path to lint
         (for `lint-rule`).
       - `check_rule` — the eslint rule id (e.g. `local/no-source-grep`); `lint-rule` only.
       - `check_violation_fixture` (#1279) — path to a KNOWN-BAD subject the wired check is run
         against to **machine-prove fail-first**; rides BOTH kinds. Capture it to let the item green
         end-to-end with zero hand-authoring at verify time; for `node-test` the negative test should
         read its subject from the `GSD_PROHIB_SUBJECT` env var so the prover can inject this fixture.
       - `check_clean_fixture` (#1346; **REQUIRED for `node-test` as of #1906**) — path to a
         KNOWN-CLEAN control subject. The `node-test` prover runs the check against it and requires
         GREEN — proving the violation's RED is caused by the subject's *content*, not by
         `GSD_PROHIB_SUBJECT` merely being set. For a `node-test` this is **mandatory**: omit it and
         the check is un-provable (fail-closed), never proven on the violation alone — so a deceptive
         content-independent test cannot pass. (`lint-rule` needs no clean fixture: its subject IS the
         linted file, no `GSD_PROHIB_SUBJECT` indirection.)
       This is a **SOFT capture (CHK-04): a `test`-tier prohibition WITHOUT a descriptor is still
       allowed** — if the author cannot yet name the wired check, leave the descriptor empty and
       proceed. It is NOT a hard authoring block; the item simply stays fail-closed/flagged
       downstream (an absent/partial descriptor — or one with no `check_violation_fixture` —
       → `descriptorFromProjection` null/under-specified/fixture-less → producer fail-closed
       locate-or-unprovable, never green). Do NOT capture `failFirst` here — it is a
       verify-time caller attestation, not a spec-authored field (#1279).
   - **Dismiss (reason)** → mark `dismissed` with a REQUIRED non-empty reason (PROB-05). The
     reason string is the audit trail; silence is not a valid dismissal.
   - **Defer** → leave `unresolved`.

   Record each prohibition as it is resolved, one `spec_prohibition` per row (the next id each time: `SP2`, …),
   leaving out a flag the outcome has no value for (`--reason` unless dismissed, `--verification` and the `check_*`
   descriptor unless resolved; the descriptor only for `test`). The acceptance line runs only for **Keep it**. A later
   change of a row's status is `specforge gsd resolve-prohibition <id> <status>`:

   ```bash
   LR_ID="{the locked requirement it guards, e.g. $SPEC_ID/LR1}"
   IFS= read -rd '' STATEMENT <<'TEXT' || true; STATEMENT=${STATEMENT%$'\n'}
   {the must-NOT, e.g. The export must not include raw email addresses}
   TEXT
   STATUS="{resolved, dismissed or unresolved}"
   VERIFICATION="{test or judgment}"
   IFS= read -rd '' REASON <<'TEXT' || true; REASON=${REASON%$'\n'}
   {why it is dismissed}
   TEXT
   CHECK_KIND="{node_test or lint_rule}"
   CHECK_TARGET="{the negative test file, or the path to lint}"
   CHECK_RULE="{the lint rule id; lint_rule only}"
   IFS= read -rd '' CHECK_VIOLATION_FIXTURE <<'TEXT' || true; CHECK_VIOLATION_FIXTURE=${CHECK_VIOLATION_FIXTURE%$'\n'}
   {a known-bad subject}
   TEXT
   IFS= read -rd '' CHECK_CLEAN_FIXTURE <<'TEXT' || true; CHECK_CLEAN_FIXTURE=${CHECK_CLEAN_FIXTURE%$'\n'}
   {a known-clean subject; required for node_test}
   TEXT
   specforge new spec_prohibition SP1 "$STATEMENT" --in spec/discuss.spec --spec "$SPEC_ID" --requirement "$LR_ID" --statement "$STATEMENT" --status "$STATUS" --verification "$VERIFICATION" --reason "$REASON" --check_kind "$CHECK_KIND" --check_target "$CHECK_TARGET" --check_rule "$CHECK_RULE" --check_violation_fixture "$CHECK_VIOLATION_FIXTURE" --check_clean_fixture "$CHECK_CLEAN_FIXTURE" || exit 1
   specforge link "$SPEC_ID" acceptance "Must not: $STATEMENT" || exit 1
   ```

**Soft gate (after resolving) — PROB-06:** read the rows, and the `unresolved` ones among them:

```bash
specforge query "$SPEC_ID" --depth 1 --kinds spec_prohibition --format context
```

- All applicable prohibitions resolved → proceed to Step 6.
- Any `unresolved` → AskUserQuestion:
  - header: "Prohibitions"
  - question: "[N] prohibition(s) are unresolved: [list]. What do you want to do?"
  - options: "Resolve now" (loop back) / "Record the spec anyway — flag unresolved" /
    "Keep probing"
  - On "anyway": the rows stay `unresolved`, which marks them `⚠ Prohibition unresolved — planner
    must treat as assumption`. This is a soft gate (write-anyway-with-flags), never a silent
    skip — the soft gate IS the control.

**`--auto` mode:** auto-`resolved` where a defensible negative acceptance criterion can be
written (test or judgment tier); otherwise leave `unresolved`. **`--auto` NEVER auto-dismisses
a prohibition** — a wrong dismissal is the exact silent failure this probe eliminates (PROB-06,
the load-bearing safety property). On a `test`-tier auto-resolution, capture the `check_kind` /
`check_target` / `check_rule` / `check_violation_fixture` / `check_clean_fixture` descriptor **only when a wired check is unambiguous**; otherwise
leave it empty — `--auto` NEVER fabricates a check path or fixture (a wrong locate is re-validated and
fails closed at the producer, but a fabricated path is still noise to avoid). Log:
`[auto] prohibitions: R resolved, U unresolved`.

**Text mode (PROB-09):** per Step 5's text-mode rule, replace the AskUserQuestion menus above
with plain-text numbered lists — there is NO hard AskUserQuestion dependency, so the probe
runs identically for non-Claude / text-mode hosts.

The prohibition rows recorded above are the spec's must-NOT axis: a `resolved`/`test` row is a checkable negative
acceptance criterion (with its `check_*` descriptor when captured), a `resolved`/`judgment` row routes to judgment
review, and an `unresolved` row is flagged as an assumption.

## Step 6: Check the spec

The spec, its requirements, interviews, edges and prohibitions are recorded. Check them against the rules below; a
fix is a `specforge set` / `specforge link` on the entity. The check prints the codes of `spec/discuss.spec`, which
also holds other phases' contexts, specs and logs: a code of theirs is not this step's to fix, and of this spec's own
entities none should print but the judged ones that wait for the verdict the prompt's `record_spec` step asks for
(`gsd/E202`, `gsd/W203`):

```bash
specforge check --format json --pick '.[file=spec/discuss.spec].code'
```

**Requirements for every requirement entry:**
- One specific, testable statement
- Current state (what exists now)
- Target state (what it should become)
- Acceptance criterion (how to verify it was met)

**Vague requirements are rejected:**
- ✗ "The system should be fast"
- ✗ "Improve user experience"
- ✓ "API endpoint responds in < 200ms at p95 under 100 concurrent requests"
- ✓ "CLI command exits with code 1 and prints to stderr on invalid input"

**Count requirements.** The display in discuss-phase reads: "Found the phase spec — {N} requirements locked.":

```bash
specforge query "$SPEC_ID" --depth 1 --pick '.nodes[kind=locked_req].id' --count
```

**Boundaries must be explicit lists:**
- "In scope" — what this phase produces
- "Out of scope" — what it explicitly does NOT do (with brief reasoning)

**Acceptance criteria must be pass/fail checkboxes** — no "should feel good" or "looks reasonable."

**If any dimensions are below minimum**, the judged verdict's `scores` carry them: `⚠ Below minimum — planner must treat as assumption`.

## Step 7: Commit

```bash
specforge git commit --add spec -m "spec(phase-${phase_number}): lock ${requirement_count} requirements for ${phase_name} (#2213)" && specforge collect --runner gsd-git --yes
```

If `commit_docs` is false the CLI returns `skipped`; the spec is recorded, not committed.

## Step 8: Wrap Up

Display:

```
Spec recorded — {N} requirements locked.

  Phase {X}: {name}
  Ambiguity: {final_score} (gate: ≤ 0.20)

Next: /gsd:discuss-phase {X}
  discuss-phase will read the spec and focus on implementation decisions only.
```

</process>

<critical_rules>
- Every requirement MUST have current state, target state, and acceptance criterion
- Boundaries section is MANDATORY — cannot be empty
- "In scope" and "Out of scope" must be explicit lists, not narrative prose
- Acceptance criteria must be pass/fail — no subjective criteria
- Nothing is recorded if the user selects "Abandon"
- Do NOT ask about HOW to implement — that is discuss-phase territory
- Scout the codebase BEFORE the first question — grounded questions only
- Max 2–3 questions per round — do not frontload all questions at once
- Step 5.5 edge probe runs after the ambiguity gate; dismissals require a reason; --auto never auto-dismisses
- Step 5.6 prohibition probe runs after the edge probe; dismissals require a reason; --auto never auto-dismisses a prohibition
</critical_rules>

<success_criteria>
- Codebase scouted and current state understood before questioning
- All 4 dimensions scored after every round
- Gate passed OR user explicitly chose to write despite gaps
- SPEC.md contains only falsifiable requirements
- Boundaries are explicit (in scope / out of scope with reasoning)
- Acceptance criteria are pass/fail checkboxes
- SPEC.md committed atomically (when commit_docs is true)
- User directed to /gsd:discuss-phase as next step
- Edge-completeness probe run; each edge row recorded; unresolved edges flagged as assumptions
- Prohibition-completeness probe run; each prohibition recorded; unresolved prohibitions flagged as assumptions
</success_criteria>
