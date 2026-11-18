<!-- Ported from gsd-core v1.19.0 gsd-core/templates/UAT.md, MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD -->
# UAT Template

Template for the `uat_session` entity of phase XX-name — persistent UAT session tracking.

---

## The UAT's Typed Edits

A phase's UAT is its `uat_session` entity, with one `uat_test` per test and one `uat_gap` per issue a test found. The session's `status` and `totals`, and a gap's `status`, `resolved_by` and `resolved_at`, are computed, never written. A test's `result` is the person's answer: the person records it with `specforge gsd set-uat-result`, and a result a test run produced is recorded with `specforge gsd record-automated-uat`. `specforge schema --kind uat_session` (likewise `uat_test` and `uat_gap`) lists the fields. Fill the values: free text in its quoted heredoc, as written, and an id, enum or path in its assignment. A refusal names its code: read it with `specforge explain <code>`.

Create the session; its `source` is every summary of the phase:

```bash
PHASE_ID="{the phase's id, e.g. v1.0/01}"
SESSION="${PHASE_ID##*/}-UAT"
PHASE_FILE=$(specforge query "$PHASE_ID" --depth 0 --pick '.nodes[0].file') || exit 1
[ -n "$PHASE_FILE" ] || { echo "no phase $PHASE_ID in the graph" >&2; exit 1; }
SUMMARIES=$(specforge query "$PHASE_ID" --depth 2 --kinds summary --pick '.nodes[kind=summary].id') || exit 1
[ -n "$SUMMARIES" ] || { echo "$PHASE_ID has no summary to test" >&2; exit 1; }
SOURCES=()
while IFS= read -r SUMMARY; do SOURCES+=(--source "$SUMMARY"); done <<<"$SUMMARIES"
specforge new uat_session "$SESSION" "Phase ${PHASE_ID##*/} acceptance" --in "$PHASE_FILE" --phase "$PHASE_ID" "${SOURCES[@]}" || exit 1
echo "UAT_SESSION=$PHASE_ID/$SESSION"
```

Add a test to the session, once per test:

```bash
UAT_SESSION="{the session's id, e.g. v1.0/01/01-UAT}"
TEST_ID="{U and the test's number, e.g. U1}"
IFS= read -rd '' NAME <<'TEXT' || true; NAME=${NAME%$'\n'}
{the test's name}
TEXT
IFS= read -rd '' EXPECTED <<'TEXT' || true; EXPECTED=${EXPECTED%$'\n'}
{what the person should observe}
TEXT
SESSION_FILE=$(specforge query "$UAT_SESSION" --depth 0 --pick '.nodes[0].file') || exit 1
[ -n "$SESSION_FILE" ] || { echo "no UAT session $UAT_SESSION in the graph" >&2; exit 1; }
specforge new uat_test "$TEST_ID" "$NAME" --in "$SESSION_FILE" --session "$UAT_SESSION" --expected "$EXPECTED" || exit 1
```

Record a gap when a test's answer is an issue:

```bash
TEST="{the test's id, e.g. v1.0/01/01-UAT/U2}"
GAP_ID="{G and the gap's number, e.g. G1}"
SEVERITY="{blocker, major, minor or cosmetic}"
IFS= read -rd '' TRUTH <<'TEXT' || true; TRUTH=${TRUTH%$'\n'}
{the expected behaviour the test found missing}
TEXT
IFS= read -rd '' REASON <<'TEXT' || true; REASON=${REASON%$'\n'}
User reported: {the person's words, verbatim}
TEXT
TEST_FILE=$(specforge query "$TEST" --depth 0 --pick '.nodes[0].file') || exit 1
[ -n "$TEST_FILE" ] || { echo "no UAT test $TEST in the graph" >&2; exit 1; }
specforge new uat_gap "$GAP_ID" "$TRUTH" --in "$TEST_FILE" --test "$TEST" --truth "$TRUTH" --reason "$REASON" --severity "$SEVERITY" || exit 1
```

Record a gap's diagnosis (one `missing` item per line):

```bash
GAP="{the gap's id, e.g. v1.0/01/01-UAT/U2/G1}"
IFS= read -rd '' ROOT_CAUSE <<'TEXT' || true; ROOT_CAUSE=${ROOT_CAUSE%$'\n'}
{the root cause the diagnosis found}
TEXT
IFS= read -rd '' ARTIFACTS <<'TEXT' || true; ARTIFACTS=${ARTIFACTS%$'\n'}
[{"path": "{a file the cause sits in, ROOT-relative}", "issue": "{what is wrong there}"}]
TEXT
IFS= read -rd '' MISSING <<'TEXT' || true; MISSING=${MISSING%$'\n'}
{what the fix must add or change}
TEXT
IFS=$'\n' read -rd '' -a MISSING_ITEMS <<<"$MISSING" || true
specforge set "$GAP" root_cause "$ROOT_CAUSE" || exit 1
specforge set "$GAP" artifacts "$ARTIFACTS" || exit 1
specforge set "$GAP" missing "${MISSING_ITEMS[@]}" || exit 1
```

`specforge query <phase> --depth 3 --kinds uat_session,uat_test,uat_gap --format context` reads the session back. The rules below say what a test's result carries, when a gap is recorded and how severity is inferred.

---

<section_rules>

**Tests:**
- `result` values: [pending], pass, issue, skipped, blocked
- If issue: add `reported` (verbatim) and `severity` (inferred)
- If skipped: add `reason` if provided
- If blocked: add `blocked_by` (tag) and `reason` (if provided)

**Gaps:**
- A `uat_gap` is recorded only when a test's answer is an issue.
- After diagnosis: set `root_cause`, `artifacts` and `missing`.
- The gaps feed directly into /gsd:plan-phase --gaps.
- A gap's `status` is computed: it is resolved once a plan whose `gap_closure` names it completes, and `resolved_by` and `resolved_at` name that plan and its summary's completion. A later regression against the same test is a new `uat_gap`, never a resolved one reopened.

</section_rules>

<severity_guide>

Severity is INFERRED from user's natural language, never asked.

| User describes | Infer |
|----------------|-------|
| Crash, error, exception, fails completely, unusable | blocker |
| Doesn't work, nothing happens, wrong behavior, missing | major |
| Works but..., slow, weird, minor, small issue | minor |
| Color, font, spacing, alignment, visual, looks off | cosmetic |

Default: **major** (safe default, user can clarify if wrong)

</severity_guide>
