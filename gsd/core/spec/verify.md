# Verification, UAT, validation, audits

```
 phase ─1──0..1─ verification ─*─ verify_gap ─→ plan (closed_by)
                     ├─*─ override ─→ truth          (human-only)
                     └─*─ human_item ─→ uat_test
        truth | artifact | key_link ◀── verdicts (judged, K7) + collector evidence;  requirement: computed from them
 phase ─1──0..1─ uat_session ─*─ uat_test ─0..1─ uat_gap ─→ plan (resolved_by), debug_session
 phase ─1──0..1─ validation ─*─ validation_row ─→ task, requirement
                     ├─*─ wave0_item
                     └─*─ manual_check
 milestone ─1──*─ milestone_audit
 project ─1──*─ broken_window ─→ phase, refactor_proposal
 phase ─1──*─ refactor_proposal
```

File: `verify.spec` in the phase folder; broken windows in `ops/windows.spec`.

## What changes from GSD

GSD's verifier writes `NN-VERIFICATION.md` with a status, a score and per-truth tables, and nothing stops an agent
from writing `status: passed`. Here the verifier records **verdicts** as `verdict` members on each truth, artifact and key
link it judged (K7), the collectors record **evidence** (test results, DOM checks), and every status below is computed from
the two. A verdict whose inputs changed since it was recorded is stale, and a stale verdict never counts as a pass:
the item and its plan, the code the item or its plan names, and every file the verifier cited
([rules.md](rules.md#judged-rules-inputs)).

| GSD per-item status | Here |
|---|---|
| truth `verified`, `failed`, `uncertain`, `present_behavior_unverified`, `verified_coincidental` | the verifier's verdict on the truth; `verified` needs a named passing test in the evidence for an `explicit` truth (honest verifier) |
| truth `passed_override` | an `override` entity, written by a human |
| artifact `verified`, `orphaned`, `stub`, `missing`; data flow `flowing`, `static`, `disconnected`, `hollow_prop` | `missing` is a pass (the file is absent); the rest are the verdict |
| key link `wired`, `not_wired`, `partial` | a pass (the pattern is searched) when `pattern` is set, else the verdict |
| requirement `satisfied`, `blocked`, `needs_human`, `orphaned` | computed from the truths and deliverables that trace to it |

## verification

GSD: `NN-VERIFICATION.md`. One per phase.

| Field | Type | Flags | GSD |
|---|---|---|---|
| phase | → phase | R I | `phase:` |
| deferred | {truth → truth R, addressed_in → phase R, evidence str R} | | `deferred:` |
| status | enum{pending, passed, gaps_found, human_needed} | C | `status:`: gaps_found when any truth, artifact or link fails; human_needed when any human item is open; a backstop truth with no explicit evidence is human_needed, never passed |
| score | str `N/M` | C | `score:` |
| verified | ts | C | `verified:`: the latest verdict's time |
| covered_files, covered_digest | files, str | C | the files the phase's verifier verdicts' `file()` inputs and `cited` name, and one digest over their current digests (the judgements' `files`) |
| re_verification | {previous_status enum{pending, passed, gaps_found, human_needed}, previous_score str, gaps_closed →* truth \| artifact \| key_link, gaps_remaining →* truth \| artifact \| key_link, regressions →* truth \| artifact \| key_link} | C | `re_verification:`, against the git baseline (K4): its status and score; items failing there and passing now, failing there and not passing now, passing there and failing now (a stale verdict is not a failure). Absent when the baseline has no verification of the phase; items absent from it are not compared. The baseline's verdicts are judged at that commit, over its files; its test inputs are not judged (results are per machine) |
| anti_patterns | {file file, line int, text str, severity enum{blocker, warning, info}} | C | the anti-pattern scan, a pass |

**verify_gap**: a failed or partial truth, as an entity a gap-closure plan can name. Name `VG1`, scoped by the phase.
`verification R`, `truth → truth R`, `reason str R`, `artifacts {path file, issue str}`, `missing strs`;
`status enum{open, closed}` and `closed_by → plan` are `C` (closed when a `gap_closure` plan naming it completes).
An open verify_gap puts the verifier in re-verification mode (failed items in full, passed items as a regression check);
`re_verification` reports the outcome, it does not start it.

**override** (kind `U`): `verification R`, `truth → truth R`, `reason str R`, `by str R`, `at date R`. The host refuses
an override in any extension's plan and any MCP write (K5 D7); a human writes it by hand or from an interactive
`specforge` command. An agent that edits the file directly is not stopped: the commit shows it, and more than three in
a phase warns.

**human_item**: `verification R`, `test str R`, `expected str R`, `why_human str R`, `uat → uat_test`.

## UAT

GSD: `NN-UAT.md`, written by `/gsd-verify-work`.

**uat_session**: `phase → phase R I`, `source →* summary R I`, `started ts I`;
`status enum{testing, partial, complete, diagnosed}` and `totals {total int, passed int, issues int, pending int, skipped int, blocked int}` are `C`.
The test being asked now is session state.

**uat_test**: `### N. [Test Name]`. Name `U1`, scoped by the phase.

| Field | Type | Flags | GSD |
|---|---|---|---|
| session | → uat_session | R I | parent |
| expected | str | R | expected behaviour |
| criterion | int | | the phase success criterion it proves |
| from_task | → task | | a harvested `<human-check>` |
| result | enum{pending, pass, issue, skipped, blocked} | H | written from the human's answer |
| source | enum{human, automated} | H | an automated result comes from a collector (DOM, e2e) |
| reported | str | | the human's words, verbatim, when `issue` |
| severity | enum{blocker, major, minor, cosmetic} | | major when absent |
| reason | str | | why skipped; `Deferred follow-up…` counts as passing |
| blocked_by | enum{server, physical_device, release_build, third_party, prior_phase} | | when blocked |

**uat_gap**: GSD `## Gaps`. Name `G-03-1`, scoped by the phase.
`test → uat_test R`, `truth str R`, `reason str`, `severity enum{blocker, major, minor, cosmetic}`, `root_cause str`,
`artifacts {path file, issue str}`, `missing strs`, `debug → debug_session`;
`status enum{open, resolved}`, `resolved_by → plan` and `resolved_at` are `C`.

## validation (Nyquist)

GSD: `NN-VALIDATION.md`, seeded by `/gsd-plan-phase` and reconciled by `/gsd-validate-phase`.

**validation**: `phase → phase R I`, `framework str`, `config_file file`, `quick_cmd str`, `full_cmd str`,
`max_latency_s int`; `approval {by str, at ts}` is `H`; `status enum{draft, validated}`, `nyquist_compliant bool` and
`wave_0_complete bool` are `C`.

**validation_row**: the Per-Task Verification Map. `validation R`, `task → task R`, `requirement → requirement`,
`threat → threat` (when `@gsd/security` is loaded), `test_type enum{unit, integration, smoke, e2e, manual} R`,
`command str`; `wave`, `file_exists` and `status enum{pending, green, red, flaky}` are `C`.

**wave0_item**: `validation R`, `path file R`, `for → requirement`, `kind enum{test_stub, fixture, framework_install} R`;
`done bool C` (the file exists and the collector saw it run).

**manual_check**: `validation R`, `behavior str R`, `requirement → requirement`, `why_manual str R`, `instructions text R`.

## milestone_audit

GSD: `v<version>-MILESTONE-AUDIT.md`, written by `/gsd-audit-milestone`. Name `A1`, scoped by the milestone.

| Field | Type | Flags | GSD |
|---|---|---|---|
| milestone | → milestone | R I | the version |
| tech_debt | {phase → phase, item str R} | | `tech_debt:` |
| integration | enum{passed, issues} | V | the integration checker's cross-phase verdict |
| debt_accepted | {by str R, at date R} | | a human accepts the `tech_debt` status |
| gaps | {requirement → requirement, status enum{unsatisfied, partial, orphaned}} | C | `gaps:`: requirements of the milestone not complete |
| nyquist | {compliant int, partial int, not_validated int, missing int} | C | `nyquist:` |
| status | enum{passed, gaps_found, tech_debt} | C | `status:` |

`/gsd-complete-milestone` refuses a milestone whose latest audit is neither `passed` nor `tech_debt` with
`debt_accepted`.

## broken_window

GSD: `.planning/WINDOWS.md`, the broken-windows ledger. Name `BW1`, project-scoped, cross-phase.

| Field | Type | Flags | GSD |
|---|---|---|---|
| kind | enum{stub, todo, fixme, skipped_test, lint_warning, unmet_truth, unrun_verify, deviation} | R | `kind` |
| phase | → phase | R | the phase that found it |
| file | file | | `file` |
| line | int | | `line` |
| description | str | R | `description` |
| waiver | {reason str R, by str R, at ts R} | | `waive` (a reason is required) |
| fixed_at | ts | | `fix` |
| refactor | → refactor_proposal | | a strict-mode refactor proposal recorded as a `deviation` |
| status | enum{open, waived, fixed} | C | waived when `waiver` is set; fixed when `fixed_at` is set or the linked proposal is accepted or declined |
| milestone | → milestone | C | from the phase |
| recorded_at | ts | H I | when the window was recorded: set by the plan that creates it |

The project computes the window counts ([project.md](project.md)). Writers: the executor, the
verifier and the refactor trigger. A resolved window is never waived or fixed again (gsd/E513); none is open at ship
when `workflow.windows_enforce` is on (gsd/E512; off under `gsd-compat`, on under `gsd-strict` and `gsd-plus`).

## refactor_proposal

GSD: `NN-REFACTOR.md`, written at `execute:post` when `refactor.trigger_enabled` is on and a touched function's
complexity crosses `refactor.complexity_threshold` or grows past `refactor.complexity_jump_delta`. Name `RP1`, scoped by
the phase.

| Field | Type | Flags | GSD |
|---|---|---|---|
| phase | → phase | R I | `phase` |
| target_file | file | R I | `target_file` |
| target_function | str | R I | `target_function` |
| score | int | R H I | `score` (decision points) |
| baseline | int | H I | `baseline`: the function's anchor score |
| delta | int | H I | `delta` |
| reasons | enums{threshold, jump} | R H I | `reason` |
| candidates | {file file, function str, line int, score int, baseline int, delta int} | H I | `candidates` |
| disposition | {outcome enum{accepted, declined} R, reason str, by str R, at ts R} | H | `refactor accept\|decline` |
| status | enum{proposed, accepted, declined} | C | `status`: from `disposition` |

Under `refactor.trigger_strict` the proposal also records a `deviation` window, cleared when the proposal is
dispositioned, never when the score improves. With `workflow.windows_enforce` off that window cannot block ship
(gsd/W514).
