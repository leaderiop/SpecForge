# Verification, UAT, validation, audits

```
 phase ─1──0..1─ verification ─*─ verify_gap ─→ plan (closed_by)
                     ├─*─ override ─→ truth          (human-only)
                     └─*─ human_item ─→ uat_test
        truth | artifact | key_link ◀── verdicts (judged, K7) + collector evidence;  requirement: computed from them
 phase ─1──0..1─ uat_session ─*─ uat_test ─0..1─ uat_gap ─→ plan (resolved_by), debug_session (@gsd/ops)
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
| artifact `verified`, `orphaned`, `stub`, `missing`; data flow `flowing`, `static`, `disconnected`, `hollow_prop` | `missing`, `min_lines`, `contains` and `exports` are a pass over the file's facts (the file is absent, too short by upstream's `split('\n')` count, lacks the text or a name it exports, found by substring as upstream does); the rest are the verdict |
| key link `wired`, `not_wired`, `partial` | a pass (the pattern is searched in `from`, and in `to`, as upstream) when `pattern` is set, else the verdict |

The passes that read project files (artifact facts, a key link's `pattern`, `anti_patterns`, `validation_row.file_exists`,
`wave0_item.done`, the refactor trigger's complexity) are buildable since protocol 1.11.0 (K11) and built in r39 (the scan pass; the refactor trigger's complexity waits for r41; the verify command's own paths are `gsd/E320`, built in r37): a pass declares the paths it
reads (`reads`) and receives a `FileFact` per file (`exists`, `bytes`, `lines`, `digest`) and, for declared text reads, the text.
Every status computed from test evidence declares `reads_tests` (K12): compute, check and guard passes then receive the
recorded test report, and I205 says once when none is recorded.
The timestamps a command writes (`approval.at`, `recorded_at`, a refactor `disposition.at`) are the host's clock,
`CommandInput.now` (K14).
| requirement `satisfied`, `blocked`, `needs_human`, `orphaned` | computed from the truths and deliverables that trace to it |

## verification

GSD: `NN-VERIFICATION.md`. One per phase. Name `03-VERIFICATION`.

| Field | Type | Flags | GSD |
|---|---|---|---|
| phase | → phase | R I | `phase:` |
| deferred | {truth → truth R, addressed_in → phase R, evidence str R} | | `deferred:` |
| status | enum{pending, passed, gaps_found, human_needed} | C | `status:`: gaps_found when any truth, artifact or link fails; human_needed when any human item is open; passed needs every truth, artifact and key link verified by a fresh gsd/E501 verdict or an override (the scan only fails, except that a key link whose literal `pattern` it found is verified; a regular-expression pattern is `unchecked` and keeps the phase `pending`), and a complete scan; a backstop truth with no explicit evidence is human_needed, never passed |
| score | str `N/M` | C | `score:` as `verified / total`, upstream's: every truth that is not deferred is in `M`; a behavior-unverified truth is in `M` and not in `N` |
| verified | ts | C | `verified:`: the latest verdict's time |
| covered_files | files | C | the files the phase's verifier verdicts' `file()` inputs and `cited` name. |
| covered_digest | str | C | one digest (`fnv1a64:` and 16 hex digits) over the `(path, digest)` of every file the phase's E501 judgements cover, sorted and de-duplicated: it changes when a covered file does. Absent while no verdict cites a file |
| re_verification | {previous_status enum{pending, passed, gaps_found, human_needed}, previous_score str, gaps_closed →* truth \| artifact \| key_link, gaps_remaining →* truth \| artifact \| key_link, regressions →* truth \| artifact \| key_link} | C | `re_verification:`, against the git baseline (K4): items failing there and passing now, failing there and not passing now, passing there and failing now (a stale verdict is not a failure), by verdict (truths, artifacts, key links) and by status (verify gaps). `previous_status` and `previous_score` are NOT BUILT: the baseline compile sees only a commit's spec files, so its scan states and status would be wrong (kernel: serve source files from the commit). Absent when the baseline has no verification of the phase; items absent from it are not compared. The baseline's verdicts are judged at that commit, over its files; its test inputs are not judged (results are per machine). `gaps_*` are ids as text (`strs`), and also name verify gaps |
| anti_patterns | {file str, line int, text str, severity enum{blocker, warning, info}} | C | the anti-pattern scan, a pass; it lists what it read |
| scan_partial | bool | C | set when a file of the phase's plans was not served (the host's text budget, a cap, a truncated read): the scan may have missed an anti-pattern, so the phase is not `passed`. Unset when every file was read. There is no information code for it |

**verify_gap**: a failed or partial truth, as an entity a gap-closure plan can name. Name `VG1`, scoped by its verification.
`verification R`, `truth → truth R`, `reason str R`, `artifacts {path str, issue str}`, `missing strs`;
`status enum{open, closed}` and `closed_by → plan` are `C` (closed when a `gap_closure` plan naming it completes).
An open verify_gap puts the verifier in re-verification mode (failed items in full, passed items as a regression check);
`re_verification` reports the outcome, it does not start it.

**override** (kind `U`; name `OV1`): `verification R`, `truth → truth R`, `reason str R`, `by str`, `at date` (`by` and `at` are not `R`: gsd/E503 is the one diagnostic of their absence). The host refuses
to create, delete or retitle an override in any extension's plan and any MCP write (K5 D7), and to `set`, `unset` or
`link` any of its fields (`truth`, `reason`, `by`, `at` are human fields, K5 D7); a human writes it by hand or from an
interactive `specforge` command. An agent that edits the file directly is not stopped: the commit shows it, and more than three in
a phase warns.

**human_item** (name `HI1`): `verification R`, `test str R`, `expected str R`, `why_human str R`, `uat → uat_test`.

## UAT

GSD: `NN-UAT.md`, written by `/gsd-verify-work`.

**uat_session** (name `03-UAT`): `phase → phase R I`, `source →* summary R I`, `started ts I`;
`status enum{testing, partial, complete, diagnosed}` and `totals {total int, passed int, issues int, pending int, skipped int, blocked int}` are `C`.
The test being asked now is session state.

**uat_test**: `### N. [Test Name]`. Name `U1`, scoped by its session.

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

**uat_gap**: GSD `## Gaps`. Name `G1`, scoped by its test (`v1.0/01/01-UAT/U2/G1`).
`test → uat_test R`, `truth str R`, `reason str`, `severity enum{blocker, major, minor, cosmetic}`, `root_cause str`,
`artifacts {path str, issue str}`, `missing strs`;
`status enum{open, resolved}`, `resolved_by → plan` and `resolved_at` are `C`. `debug → debug_session` is an
enhancement declared by `@gsd/ops` on this kind: core depends on no other `@gsd/*` extension.

## validation (Nyquist)

GSD: `NN-VALIDATION.md`, seeded by `/gsd-plan-phase` and reconciled by `/gsd-validate-phase`.

**validation** (name `03-VALIDATION`): `phase → phase R I`, `context_digest str H` (as on the spec and research; gsd/W406), `framework str`, `config_file str`, `quick_cmd str`, `full_cmd str`,
`max_latency_s int`; `approval {by str, at ts}` is `H`; `status enum{draft, validated}`, `nyquist_compliant bool` and
`wave_0_complete bool` are `C`.

**validation_row**: the Per-Task Verification Map. Name `VR1`. `validation R`, `task → task R`, `requirement → requirement`,
`test_type enum{unit, integration, smoke, e2e, manual} R`,
`command str`, `file str` (the row's test file; a `str`, as wave 0 may create it); `wave`, `file_exists` and `status enum{pending, green, red, flaky}` are `C`. `threat → threat` is an
enhancement declared by `@gsd/security` on this kind.

**wave0_item** (name `W0-1`): `validation R`, `path str R` (a `file` would be E016 on the file wave 0 creates), `for → requirement`, `kind enum{test_stub, fixture, framework_install} R`;
`done bool C` (the file exists and the collector saw it run).

**manual_check** (name `MC1`): `validation R`, `behavior str R`, `requirement → requirement`, `why_manual str R`, `instructions text R`.

## milestone_audit

GSD: `v<version>-MILESTONE-AUDIT.md`, written by `/gsd-audit-milestone`. Name `A1`, scoped by the milestone.

| Field | Type | Flags | GSD |
|---|---|---|---|
| milestone | → milestone | R I | the version |
| tech_debt | {phase → phase, item str R} | | `tech_debt:` |
| integration | enum{passed, issues} | V | the integration checker's cross-phase verdict |
| debt_accepted | {by str R, at date R} | U | a human accepts the `tech_debt` status: refused in every plan and MCP write, written by hand or from an interactive `specforge` command |
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
| file | str | | `file` (a `str`: the file may since have been deleted) |
| line | int | | `line` |
| description | str | R | `description` |
| waiver | {reason str R, by str R, at ts R} | H I | `waive` (a reason is required): written by `waive_window` |
| fixed_at | ts | H I | `fix`: written by `fix_window` |
| refactor | → refactor_proposal | | a strict-mode refactor proposal recorded as a `deviation` |
| status | enum{open, waived, fixed} | C | waived when `waiver` is set or the linked proposal is declined; fixed when `fixed_at` is set or the linked proposal is accepted |
| milestone | → milestone | C | from the phase |
| recorded_at | ts | H I | when the window was recorded: set by the plan that creates it |

The project computes the window counts ([project.md](project.md)). Writers: the executor, the
verifier and the refactor trigger. A resolved window is never waived or fixed again (gsd/E513, a pass: `waiver` and `fixed_at` are never both set, and each is `I`); none is open at ship
when `workflow.windows_enforce` is on (gsd/E512; off under `gsd-compat`, on under `gsd-strict` and `gsd-plus`).

## refactor_proposal

GSD: `NN-REFACTOR.md`, written at `execute:post` when `refactor.trigger_enabled` is on and a touched function's
complexity crosses `refactor.complexity_threshold` or grows past `refactor.complexity_jump_delta`. Name `RP1`, scoped by
the phase.

| Field | Type | Flags | GSD |
|---|---|---|---|
| phase | → phase | R I | `phase` |
| target_file | str | R I | `target_file` |
| target_function | str | R I | `target_function` |
| score | int | H I | `score` (decision points) |
| baseline | int | H I | `baseline`: the function's anchor score |
| delta | int | H I | `delta` |
| reasons | enums{threshold, jump} | H I | `reason` |
| candidates | {file str, function str, line int, score int, baseline int, delta int} | H I | `candidates` |
| disposition | {outcome enum{accepted, declined} R, reason str, by str R, at ts R} | H | `refactor accept\|decline` |
| status | enum{proposed, accepted, declined} | C | `status`: from `disposition` |

A harness field is never `R` (the DSL refuses it); the trigger that writes `score` and `reasons` also writes them together.

Under `refactor.trigger_strict` the proposal also records a `deviation` window, cleared when the proposal is
dispositioned, never when the score improves. With `workflow.windows_enforce` off that window cannot block ship;
reporting that is gsd/W514, NOT BUILT (kernel: `SettingDescriptor.gates_when_on`).

## Known differences from GSD

Kept on purpose, listed so none is silent:

- A truth the verifier is `uncertain` about stays `pending`; upstream makes it a human item (`human_needed`). A
  backstop truth with no explicit evidence is not `human_needed`: only a fresh verdict makes it verified. A verification with no truth,
  whose artifacts and key links all pass, is `passed 0/0`.
- A verification is judged one per phase, but nothing refuses a second: the last by id wins, and W504 counts per
  verification, not per phase. The same holds for a phase's validation (the first by id).
- A milestone audit lists a requirement of a `gaps_found` phase as `partial`, where upstream lists it `unsatisfied`, and
  does not list the complete requirements of phases that have not passed. The audit's status is `gaps_found` either way.
- A UAT session with a blocked test or an unexcused skip and open gaps that all have root causes is `diagnosed`;
  upstream says `partial`. A session with no test is `complete`. A UAT status is computed, so it differs from the
  `status:` a writer sets when a run ends (the twin: [parity](../../parity/README.md)).
- The scan marks a directory or an unreadable file `unchecked` where upstream fails it; `unchecked` never fails an item.
  Debt-marker matching is by whole word (`FIXMEs`, `FIXME_LATER` are not blockers; upstream greps the substring).
- `gsd/W318` flags any stale judgement on a plan, not only the plan-check's. Harmless while no other rule judges a plan.
- The host charges the 1 MiB text budget per entity and path and in plan order: an older phase can spend it, and a
  file named by several fields is read several times. A phase left with an unread file is `scan_partial`.
