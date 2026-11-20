# gsd-strict gates fresh verdicts, traced requirements and task trailers, and is measured

**Status:** accepted (2026-11-20). Builds on
[ADR 0288](0288-an-extensions-settings-are-typed-and-set-in-specforge-json.md) D1, D3 and D4 (a preset sets settings
and raises codes, never lowers them; an explicit setting wins over the preset),
[ADR 0377](0377-a-verdict-is-fresh-over-what-it-cites-and-owed-by-its-owner-state.md) D6 and D7 (the judged rules,
and `gsd/E516`), [ADR 0331](0331-the-gsd-requirement-thread.md) with
[ADR 0371](0371-links-come-from-gsds-fields-and-collected-data.md) D5 (`gsd/E515` reads a requirement's computed
`proof`), [ADR 0303](0303-a-collectors-plan-is-judged-for-host-errors-only.md) (a collector's plan is never refused
for `gsd/E521`) and ADR 0371 D1 (`gsd/E519` left with `truth.requirement`). It amends
[ADR 0351](0351-a-plan-lists-its-decisions-and-the-threads-gaps-have-codes.md) D6 and D8 (as ADR 0371 left them),
[ADR 0358](0358-a-risk-is-rated-and-an-invariant-is-judged.md) D12's gate bullet,
[ADR 0362](0362-a-judged-target-may-wait-for-its-owner-state.md)'s context line on `gsd/E516` and the last sentence of
[gsd ADR 0002](../../gsd/adr/0002-a-capability-extension-plugs-into-core-at-its-hook-points.md) D-S-5, and it
re-measures the three-profile claim of
[ADR 0374](0374-the-root-spec-runs-on-gsd-core-and-its-tests-name-requirements.md). No protocol change, no kernel
change, no new code and no new setting: one table of `@gsd/core` moves.

## Context

- **gsd-strict blocked where GSD warns, and stopped there.** A verdict that is not fresh, a complete requirement that no
  commit and passing test trace, and a task commit that names its task only by its title blocked under `gsd-plus`
  alone. `gsd-plus` also carried cross-workstream conflicts, so a project that wanted honest gates had to take a
  workstream rule with them.
- **The profile's description said "gsd-plus adds traceability gates".** The line between the two stances was drawn at
  what SpecForge can check, not at what changes how a team works.
- **Nothing measured the gates.** No test said how fresh the verdicts at a gate are, how exactly re-verification follows
  one edit, or how many rules the code books judge.

## D1. gsd-strict presets the traceability, verdict and trailer gates

The preset table of the `profile` setting:

| Setting | gsd-compat | gsd-strict | gsd-plus |
|---|---|---|---|
| `workflow.windows_enforce` | false | true | true |
| `specforge.task_commits` | false | true | true |
| `specforge.traceability_gate` | false | true | true |
| `specforge.verdict_gate` | false | true | true |
| `specforge.cross_workstream_conflicts` | false | false | true |
| `specforge.plan_paths_touched` | false | true | true |
| `specforge.task_trailers` | false | true | true |

`gsd/E515`, `gsd/E516` and `gsd/E521` are errors whenever their gate is on, under every profile, so only the settings
move: the loop that raises the codes whose strict severity is above their compat one is untouched. An explicit setting
beats the profile (ADR 0288): `{profile: gsd-strict, specforge.verdict_gate: false}` turns the verdict gate off.
`gsd-compat`, the default, presets none of them, as before.

The reason: each gate enforces a claim gsd-core's own flow already writes (a verdict its verifier records, a
requirement a summary completes, the `Gsd-Task:` trailer its executor writes), so blocking on it is strictness, not a
new way of working.

## D2. gsd-plus is gsd-strict and cross-workstream conflict detection

`specforge.cross_workstream_conflicts` (`gsd/E517`) is the one setting that separates the two profiles.

The reason: `gsd/E517` judges plans of other workstreams, which upstream runs independently by design. Refusing on it
changes how a team splits its work, which is a choice, not a check of a claim.

## D3. Under gsd-strict a verdict of another extension that is not fresh refuses core's gates

With `@gsd/security` loaded, a stale, unrecorded or unreadable `gsd.security/W302` or `gsd.security/W303` verdict
refuses every core gate, and a missing one refuses the gates whose scope holds its threat or its audit, since
`gsd/E516`'s excusal covers `gsd/` rules only (ADR 0377 D7). The diagnostic's suggestion names the step of the
verdict's extension that judges it, through `specforge query <entity> --inputs <rule>`, which prints its judge: for
both rules that is `secure_phase` (`/gsd-secure-phase`), which records the verdict again. A strict project that loads
`@gsd/security` runs it after an edit to a file a closed threat names, before it ships.

The reason: a mitigation the auditor judged is a claim on file like any other; a gate that let it go stale would vouch
for what nobody checked.

## D4. Three measures, each computed by a test from what specforge prints

- **Verdict freshness at the ship gate.** Of the judgements the ship gate of a phase reads (every one the graph lists,
  except a missing one outside the phase), the share whose state is `fresh`; the gate's `gsd/E516` diagnostics name
  exactly the others. Bar: 100% under `gsd-strict`, on the taskboard and vault twins.
- **Re-verification scope.** Summed over every single-file edit of a twin, the verdicts `specforge gsd re-verify
  <phase> --stale-only` lists (and on the vault, those `/gsd-secure-phase` lists), divided by the verdicts `specforge
  query <entity> --inputs <rule>` says are not fresh, with the over-judged and the under-judged sets both empty. An
  edit that stales nothing must list nothing, and adds to neither sum. Bar: exactly 1.0.
- **Judged rules.** The validation rules `@gsd/core` and `@gsd/security` declare as judged, each extension's count
  equal to its table. Bar: 12 or more.

Each measure has a control that makes it fail (a stale verdict, stripped verdicts, a skipped or a whole-phase
re-verification), so it cannot pass on nothing.

The reason: a gate is only as honest as the verdicts it reads, and a number a test computes from the CLI's output can
neither drift from the code nor be typed in.

## D5. gsd/E519 stays gone

`gsd/E519` read `truth.requirement`, which left `@gsd/core` with ADR 0371 D1. A requirement is traced through what
gsd-core writes (its plans' tasks, their commits and the passing tests that name them), which `gsd/E515` reads, so
strict's traceability is `gsd/E515`'s. No truth-to-requirement field returns
([ADR 0370](0370-gsd-core-holds-exactly-gsd-cores-kinds-and-fields.md) D1).

The reason: gsd-core's planner writes no requirement on a truth, so a rule that reads one would judge a field only
SpecForge's users could fill.

## Rejected

- **A fourth profile between strict and plus.** A profile is a stance, and three already cover GSD as it is, GSD made
  strict, and SpecForge's cross-workstream rules.
- **`gsd/E517` into gsd-strict.** D2.
- **Resurrecting `gsd/E519`.** D5.
- **Raising severities instead of presetting the gates.** The three are already errors, and a raise cannot turn a gated
  rule on.
- **gsd-strict as the default.** `gsd-compat` is GSD at its tracked version, and the parity suite runs on it.
- **Measures typed into a report instead of computed by tests.** A typed number does not move when the code does.
- **Counting judged rules from `rules.md` instead of the declaration.** The declaration is what the host runs;
  `rules.md` is checked against it, not the other way round.

## Limits

- The measures run on two twins and one gate (`/gsd-ship`).
- `gsd/E521` reads the commits gsd-git records, so it is silent on a project that records none.
- On a twin whose requirements no recorded commit traces, the ship gate is refused by `gsd/E515` whatever the verdicts
  are, so the freshness measure reads `gsd/E516`'s own gate entry and diagnostics.
- The A/B protocol's arm B names gsd-strict, so a run on a build after this decision has `gsd/E515` and `gsd/E516` at
  its gates. Its preflight compares check diagnostics, where both guards are silent; the tag that froze its first
  version keeps the old split.

## Measures

| Measure | What it counts | Computed by | Bar |
|---|---|---|---|
| Verdict freshness at the ship gate | fresh judgements the ship gate reads ÷ the judgements it reads; `gsd/E516` names the rest | `gsd/core/tests/cli/verdict_freshness.rs`, `gsd/security/tests/cli/verdict_freshness.rs` | 100% under gsd-strict |
| Re-verification scope | verdicts re-verification lists ÷ verdicts not fresh, over every single-file edit; over- and under-judged sets empty | `gsd/core/tests/cli/rejudge_scope.rs`, `gsd/security/tests/cli/rejudge_scope.rs` | exactly 1.0 |
| Judged rules | the judged validation rules of `@gsd/core` and `@gsd/security` | `gsd/security/tests/declaration.rs` | 12 or more |

Measured on 2026-11-20, each line as the named test prints it. The black-box tests run the `specforge` binary built
from the tree and the `@gsd/security` component, so each command runs from the `gsd/` directory with `SPECFORGE_BIN`
and `GSD_SECURITY_WASM` set as `gsd/core/README.md` says.

- **Verdict freshness at the ship gate, taskboard twin:**
  `verdict freshness at ship on the taskboard twin: 7 of 7 fresh (100%)`, printed by
  `cargo nextest run --locked -p gsd-core -E 'test(=verdict_freshness::every_verdict_the_ship_gate_reads_is_fresh_under_strict)' --no-capture`.
- **Verdict freshness at the ship gate, vault twin:**
  `verdict freshness at ship on the vault twin: 12 of 12 fresh (100%)`, printed by
  `cargo nextest run --locked -p gsd-security -E 'test(=verdict_freshness::every_verdict_the_ship_gate_reads_is_fresh_on_the_vault_under_strict)' --no-capture`.
- **Re-verification scope, taskboard twin:**
  `re-verification scope on the taskboard twin: 12 re-judged of 12 stale over 30 single-file edits (0 over-judged, 0 under-judged): 1.0`,
  printed by
  `cargo nextest run --locked -p gsd-core -E 'test(=rejudge_scope::re_verification_lists_exactly_the_stale_verdicts_after_any_single_file_edit)' --no-capture`.
- **Re-verification scope, vault twin, split by rule:**
  `re-verification scope on the vault twin: 15 re-judged of 15 stale over 23 single-file edits (0 over-judged, 0 under-judged; gsd.security/W302 5 of 5, gsd.security/W303 0 of 0, gsd/E501 10 of 10): 1.0`,
  printed by
  `cargo nextest run --locked -p gsd-security -E 'test(=rejudge_scope::re_verification_and_the_secure_phase_list_exactly_the_stale_verdicts_after_any_single_file_edit)' --no-capture`.
- **Judged rules:** `judged rules declared: 10 by @gsd/core and 2 by @gsd/security, 12 in all`, printed by
  `cargo nextest run --locked -p gsd-security -E 'test(=the_code_books_declare_at_least_twelve_judged_rules)' --no-capture`
  (it runs on the SDK's `MockHost` and needs neither variable).
- **The repository's own check under each profile**, the re-measure of ADR 0374's three-profile claim: in a copy of the
  repository (`git archive HEAD`), `specforge settings set @gsd/core profile <profile>` then `specforge check --format
  json` exits 0 with no diagnostic (`[]`) under gsd-compat, gsd-strict and gsd-plus, and the three outputs are
  byte-identical.

Each bar is asserted by the test that prints it, so a regression fails the build rather than this record.

## What each decision becomes

"Superseded" means the decision no longer holds. "Kept" means it holds as written. "Amended" means it holds with the
change named.

| ADR | Decision | Outcome | Why |
|---|---|---|---|
| 0351 | D6 | amended | `specforge.task_trailers` is on under gsd-strict too, not under gsd-plus only (D1) |
| 0351 | D8 | amended | the presets set `specforge.task_trailers` under gsd-strict and gsd-plus (D1) |
| 0358 | D12, the gate bullet | amended | a stale verdict on an invariant in force refuses a scoped gate under gsd-strict and gsd-plus (D1) |
| 0362 | Context, the `gsd/E516` line | amended | the same, for an early verdict on a pending target (D1) |
| gsd 0002 | D-S-5, the last sentence | amended | judged mitigation is built (ADR 0377 D6), and refuses core's gates under gsd-strict and gsd-plus (D3) |
| 0374 | As built, "The three profiles" | re-measured | the root checks under the new presets (see Measures and Consequences) |
| 0371 | D1 | kept | `gsd/E519` stays gone (D5) |

ADR 0351: 2 amended. ADRs 0358 and 0362 and gsd ADR 0002: 1 amended each. ADR 0374: 1 re-measured. ADR 0371: 1 kept.

## Consequences

Each holds once the change this ADR opens lands.

- A project under gsd-strict is refused at a gate on a verdict that is not fresh, on a complete requirement no commit
  and passing test trace, and has an error on a task commit without its `Gsd-Task:` trailer.
- Under gsd-strict any verdict that is stale, unrecorded or unreadable anywhere in the project refuses every gate (ADR
  0377 D7), including after ordinary edits to files a closed phase's verdicts cite, until the verdict is recorded
  again. The way out for a project that will not re-judge is the explicit `specforge.verdict_gate: false` beside the
  profile, which beats the preset; the profile test shows it.
- `gsd-plus` differs from gsd-strict by `gsd/E517` alone.
- `gsd-compat` is unchanged, and so is every parity case.
- Outside a gate the three guards are silent, and `gsd/E521` reads only recorded commits, so `specforge check` on a
  project that records no commit reports the same diagnostics under every profile.
- The `@gsd/core` blob is re-vendored for the preset table and the profile's description.

## Would reopen it

- A measured adopter complaint that gsd-strict blocks work no one can fix.
- A second gate that the freshness measure should read.
- A third extension that declares judged rules.
