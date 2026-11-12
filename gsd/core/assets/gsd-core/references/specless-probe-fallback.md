<!-- Ported from gsd-core v1.19.0 gsd-core/references/specless-probe-fallback.md, MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD -->
# Spec-less Probe Fallback — protocol

Lazy-loaded by `workflows/plan-phase.md` step 7.95 (the gate) and the `<specless_probe_fallback>`
planner block. When a phase SPEC did NOT supply `## Edge Coverage` / `## Prohibitions`, plan-phase
runs the same probe protocol the SPEC path uses and authors the predicates into PLAN.md `must_haves`
(ADR-857 Phase 6 — the *else branch* of the `<downstream_consumer>` SPEC-conditional lift). This is
core workflow-body substrate — NOT the `PLAN_PRE_HOOKS_JSON contribution into planner` capability rail
(D-03). Section absence is read from the graph in the gate: the spec's `edge` and `spec_prohibition`
entities; this file holds the
*run-the-probe* half so the capped plan-phase.md stays lean (#717/#1074 budget).

## 0. Gate — toggle + per-section absence (run first, in the orchestrator)

Reads the default-ON toggle and computes `EDGE_ABSENT` / `PROHIB_ABSENT` from the spec's `edge` /
`spec_prohibition` entities; records a VISIBLE skip when disabled or when the phase has no requirement IDs (never a silent
skip, never a hard-fail). Sets `SPECLESS_FALLBACK`, `EDGE_ABSENT`, `PROHIB_ABSENT`, and
`SPECLESS_FALLBACK_DISABLED` for §A and the planner prompt.

```bash
# Toggle defaults ON (D-04 / RAIL-05): any value other than literal "false" enables.
SPECLESS_CFG="{{setting:workflow.specless_probe_fallback|true}}"   # workflow.specless_probe_fallback (the settings)
SPECLESS_FALLBACK=true; [[ "$SPECLESS_CFG" == "false" ]] && SPECLESS_FALLBACK=false

# Per-section absence (D-05 / RAIL-03): "not supplied" = the spec holds no row of the section's kind;
# no spec ($SPEC_FILE empty) => not supplied => fallback fires.
EDGE_ABSENT=1; PROHIB_ABSENT=1
if [ -n "$SPEC_FILE" ]; then
  EDGES=$(specforge query "$SPEC_FILE" --depth 1 --kinds edge --pick '.nodes[kind=edge].id' --count) || exit 1
  PROHIBS=$(specforge query "$SPEC_FILE" --depth 1 --kinds spec_prohibition --pick '.nodes[kind=spec_prohibition].id' --count) || exit 1
  [ "$EDGES" -gt 0 ] && EDGE_ABSENT=0
  [ "$PROHIBS" -gt 0 ] && PROHIB_ABSENT=0
fi

# Disabled path - record the skip VISIBLY, never silently (RAIL-05 / PROH-4); the note rides into the
# planner prompt (Step 8) so the plan records that no probe predicates were generated.
SPECLESS_FALLBACK_DISABLED=""
if [[ "$SPECLESS_FALLBACK" != "true" ]]; then
  echo "WARNING: probe fallback disabled (workflow.specless_probe_fallback=false); skip recorded, not silent." >&2
  SPECLESS_FALLBACK_DISABLED="probe fallback disabled (workflow.specless_probe_fallback=false): no probe-derived predicates generated for SPEC-absent sections this run."
fi

# Nothing-to-probe guard: the fallback derives predicates from requirement TEXT, so zero requirement IDs => nothing to
# probe => skip VISIBLY (like the disabled path), NOT a hard-fail: a no-SPEC + no-requirements phase does not abort
# under the default-ON fallback. The phase's requirement entities are counted; none => no requirements.
SPECLESS_REQ_N=$(specforge query "$PHASE_ID" --depth 1 --kinds requirement --pick '.nodes[kind=requirement].id' --count) || exit 1
if [[ "$SPECLESS_FALLBACK" == "true" ]] && [ "$SPECLESS_REQ_N" -eq 0 ]; then
  echo "info: spec-less probe fallback: phase has no requirement IDs - nothing to probe; skipping (visible skip)." >&2
  SPECLESS_FALLBACK=false
  SPECLESS_FALLBACK_DISABLED="spec-less probe fallback skipped: phase has no requirement IDs to probe (visible skip)."
fi
```

## A. Edge probe (deterministic) — run when `SPECLESS_FALLBACK=true` AND `EDGE_ABSENT=1`

Mirrors spec-phase Step 5.5 verbatim; the ONLY divergence (D-02) is the subject: the phase, whose
requirement entities are probed, instead of the spec. Leave `$COVERAGE` empty when
`EDGE_ABSENT=0` — a SPEC-supplied section is never re-run (section-level precedence).

```bash
# THE ONE DIVERGENCE (D-02): the subject is THIS phase; its requirement entities are probed.
# Invoke + CAPTURE, exit-checked (engine FAILS CLOSED exit 1 on bad shape; a bare capture
# would swallow it and fall through to prose re-derivation = fail-OPEN).
if ! COVERAGE=$(specforge gsd edge-probe "$PHASE_ID" --format json); then
  echo "ERROR: edge-probe engine failed (invalid shapes or bad input) — fix the requirement(s); never proceed with empty coverage." >&2
  exit 1
fi
# Exit-0-but-garbage guard: report must parse as JSON with { items[], coverage{} }.
if [ -z "$COVERAGE" ]; then
  echo "ERROR: edge-probe produced an unparseable/malformed coverage report — refusing to proceed." >&2
  exit 1
fi
# Zero-applicable guard: surface a likely classification miss loudly (spec-phase 5.5:277 shape).
# Read the report: check its { items[], coverage{} } shape; coverage.applicable 0 is likely a classification miss: say so

```

**Edge `--auto` resolution rules (reuse spec-phase 5.5 verbatim, D-06):** auto-`resolved`
(verification: explicit) where a defensible acceptance criterion can be written (→ a plain
`must_haves.truths` string); else auto-`resolved` (verification: backstop) → author it as a
**structured flat-scalar marker** `{ statement: <the check>,
verification: backstop }` in `must_haves.truths`, NOT a prose note (the verifier branches
deterministically on the `verification: backstop` field; a parenthetical is unparseable — the #1110
fragility; flat scalar `verification:` key, never a nested object, ADR-550 #1278). A `backstop` truth
the verifier cannot confirm with explicit evidence abstains → `human_needed` (reason
`insufficient_spec`), never a silent pass (#1154; `gsd-core/references/honest-verifier.md`). **Never
auto-dismiss** (a wrong dismissal is the exact silent failure this eliminates). An `unclassified` row
stays **`unresolved`** (#1110) — never auto-resolved with backstop — and is surfaced to the planner as a flagged
assumption. Pass `$COVERAGE` (+ the gate's `$SPECLESS_FALLBACK_DISABLED` note) into the gsd-planner
prompt (Step 8). When `EDGE_ABSENT=0`, `$COVERAGE` is empty and this does not run.

## B. Prohibition recall (LLM prose pass) — run when `PROHIB_ABSENT=1`

There is NO compiled prohibition engine and NO `node` invocation (ADR-550 D7b) — the gsd-planner runs
this in-prompt. Full two-stage protocol, canon-referral rule, and status×verification schema live in
`~/.claude/gsd-core/references/prohibition-probe.md` (do not inline it). Summary:

- **Stage 1 — Recall (adversarial).** Per requirement: *"What could this feature silently become that
  the author would NOT want, but the spec does not forbid?"* Over-produce (~10 raw must-NOT candidates).
- **Stage 2 — Precision.** DROP routine-engineering (normal correctness/hygiene — owned by the edge
  probe or code review); KEEP values / safety / ethics (~2–3 survive).
- **Canon-referral drop (ADR-550 D6).** A kept candidate that is canon security/compliance (OWASP /
  prototype-pollution / path-traversal / injection / GDPR / generic fairness) is NOT minted — emit a
  one-line breadcrumb and DROP it.

**Fallback `--auto` divergence (D-06 / RAIL-04 / PROH-1):** author each kept prohibition as
**flagged-unverified with NO wired-check descriptor**. NEVER write `check_kind` / `check_target` /
`check_rule` / `check_violation_fixture` / `check_clean_fixture` — there is no human to wire/verify a
check, and a descriptor-less item is what keeps it fail-closed (it disposes
`{status:'unverified', flagged:true}` downstream via the reused `dispositionForProhibition`). **Never
auto-dismiss**; never fabricate a check path. Surface any `unresolved` prohibition as a flagged
assumption — never a silent drop.

## C. Authoring (the `<downstream_consumer>` else-branch)

Author the fallback report into `must_haves` with the SAME lift the SPEC path uses — only the source
changes (the fallback report, not the SPEC):

- **Edges →** every resolved (verification: explicit) edge's acceptance criterion → `must_haves.truths` as a plain string;
  every resolved (verification: backstop) edge → `must_haves.truths` as a structured `{ statement, verification: backstop }`
  marker (NOT prose; #1110/#1278), which abstains → `human_needed` at verify time when unconfirmed
  (#1154); every `unresolved`/`unclassified` row → an explicit flagged assumption (never a silent drop).
- **Prohibitions →** every kept prohibition → the `must_haves.prohibitions:` sibling block (NOT
  `truths`, ADR-550 D3) via the single `projectProhibitions` serializer (Hyrum — no second
  serializer), authored **descriptor-less** (no `check_*` scalar) so each disposes flagged-unverified.
- **Section-level precedence:** a SPEC-supplied section is never re-run or overwritten — exactly one
  producer per section.
- **No-silent-drop equality:** for each section, (# probe-surfaced items) == (# authored into
  `must_haves` + # surfaced as flagged assumptions).
