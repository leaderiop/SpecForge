<!-- Ported from gsd-core v1.19.0 gsd-core/templates/VALIDATION.md, MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD -->
# Phase {N} — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

## The Validation's Typed Edits

A phase's validation strategy is its `validation` entity, with one `validation_row` per task of the Per-Task Verification Map and one `wave0_item` per file wave 0 creates. A row's `status` is computed from the task's linked tests; the validation's `nyquist_compliant` (every row green and every task covered) and `wave_0_complete` are computed, and its `status` reads validated once a person approves it with `specforge gsd approve-validation`: none is written. While a phase has a validation, each of its tasks needs an automated verify or a wave-0 item for its requirements (`gsd/E314`). `specforge schema --kind validation` (likewise `validation_row` and `wave0_item`) lists the fields. Fill the values: free text in its quoted heredoc, as written, and an id, enum or path in its assignment. A refusal names its code: read it with `specforge explain <code>`.

Record the validation when the phase has none, then one row (run once per row). With `@gsd/security` loaded a row also takes its Threat Ref and Secure Behavior, `--threat <threat id>` and `--secure_behavior` (the expected secure behavior, or none):

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

`specforge query <phase> --depth 2 --kinds validation,validation_row,wave0_item --format context` reads the strategy back. The guidance below says how often to sample and what the sign-off checks.

---

## Sampling Rate

- **After every task commit:** Run `{quick run command}`
- **After every plan wave:** Run `{full suite command}`
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** {N} seconds

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < {N}s
- [ ] `nyquist_compliant` reads true
