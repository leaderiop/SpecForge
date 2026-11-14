<!-- Ported from gsd-core v1.19.0 gsd-core/templates/verification-report.md, MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD -->
# Verification Report Template

## The Verification's Typed Edits

A phase's verification is its `verification` entity: its `status` (passed, gaps_found, human_needed), `score`, `covered_files` and `re_verification` are computed from the verifier's verdicts, never written. The verifier records one `gsd/E501` verdict per `truth` (and the rules on artifacts and key links the same way): it reads the truth's inputs naming what the verdict will cite, judges, and records the outcome with the `basis` copied verbatim, citing every file it read and every test fact it relied on by its id (the `test` step of `specforge trace --route gsd/requirement-thread --in <phase> --format json`). The citations are part of the basis: read the inputs with `--cite-file <file>` and `--cite-test <test fact id>` for each one `CITED` lists, or the record is refused as stale. `specforge schema --kind verification` (likewise `truth`) lists the fields. Fill the values: free text in its quoted heredoc, as written, and an id or outcome in its assignment. A refusal names its code: read it with `specforge explain <code>`.

```bash
TRUTH="{the truth's id, e.g. v1.0/01/01-01/TR1}"
OUTCOME="{verified, verified_coincidental, failed, uncertain or present_behavior_unverified}"
IFS= read -rd '' BASIS <<'TEXT' || true; BASIS=${BASIS%$'\n'}
{the basis object of specforge query "$TRUTH" --inputs gsd/E501 --cite-file <file> --cite-test <test fact id> --format json, verbatim}
TEXT
IFS= read -rd '' CITED <<'TEXT' || true; CITED=${CITED%$'\n'}
[{"file": "{an implementation file you read, ROOT-relative}"}, {"test": "{a test fact id the thread's test step prints}"}]
TEXT
IFS= read -rd '' FINDINGS <<'TEXT' || true; FINDINGS=${FINDINGS%$'\n'}
[{"severity": "{error or warning}", "message": "{what is missing or wrong, with its evidence}"}]
TEXT
specforge gsd record-verdict "$TRUTH" E501 "$OUTCOME" gsd-verifier "$BASIS" --cited "$CITED" --findings "$FINDINGS" || exit 1
```

A verified truth takes `[]` as its findings. `specforge query <phase> --depth 2 --kinds verification --format context` reads the computed verification back. The guidelines below say what the outcomes, the evidence in a finding's message and the severities mean.

---

## Guidelines

**Status values (the verification's computed `status`):**
- `passed` — All must-haves verified, no blockers
- `gaps_found` — One or more critical gaps found
- `human_needed` — Automated checks pass but human verification required

**Per-truth outcomes (a verdict's `OUTCOME`: `verified`, `present_behavior_unverified`, `verified_coincidental`, `failed` and `uncertain`, in this order):**
- `✓ VERIFIED` — supporting artifacts pass all checks; for a behavior-dependent truth, a behavioral test exercised the asserted behavior
- `⚠️ PRESENT_BEHAVIOR_UNVERIFIED` — present + wired, but a state transition or cancellation/cleanup/ordering invariant was not exercised by any test. Counts toward `behavior_unverified`, routes to human verification, and is *excluded* from the verified score. Per-truth only — on its own the overall `status:` becomes `human_needed` (unless a higher-precedence `gaps_found` also applies); the item is preserved in `behavior_unverified_items` regardless.
- `✓ VERIFIED (coincidental-reliance)` — an **advisory** qualifier on a truth that *is* verified but holds for an incidental reason rather than a guaranteed one (#1955): `undeclared-precondition` (state nothing in the phase's artifacts or a declared prerequisite guarantees), `incidental-ordering` (an order or side effect nothing in the code enforces), or `fixture-only` (the test's own setup establishes the precondition; the production path has no equivalent). The base `✓ VERIFIED` token is kept verbatim and leading, so it counts toward the verified score exactly as before — the advisory changes no score and no status, and never produces a human-verification item. Each flagged truth is listed in `coincidental_reliance_items` with the reason and what to harden. Not applied to a truth that never reached `✓ VERIFIED`, nor to a `PASSED (override)` truth.

  **Before a truth's `verified` verdict is recorded, run the reliance check on it.** Ask why the truth holds and classify the evidence you already recorded, not your confidence in it. Flag it when the evidence names one of the three reasons above. Do NOT flag: a precondition the code establishes or explicitly defaults; ordering the code enforces (await, explicit sequencing); a fixture merely supplying input the real caller also supplies; unease naming no specific state, ordering, or fixture. The check is endogenous and so weaker than an exogenous tag (`gsd-core/references/honest-verifier.md`) — which is why it is advisory and never a gate. The usual fix is to promote the hidden assumption into a declared precondition.
- `✗ FAILED` — artifact missing, stub, or unwired
- `? UNCERTAIN` — can't verify programmatically

**Evidence types:**
- For EXISTS: "File at path, exports X"
- For SUBSTANTIVE: "N lines, has patterns X, Y, Z"
- For WIRED: "Line N: code that connects A to B"
- For FAILED: "Missing because X" or "Stub because Y"

**Severity levels (a finding's `severity`):**
- `error`: prevents goal achievement, must fix (upstream's 🛑 Blocker)
- `warning`: incomplete, but does not block (upstream's ⚠️ Warning); a notable point that is no problem is no finding
