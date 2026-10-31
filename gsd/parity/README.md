# Parity suite

**Status: the four checks are built (r44); three run in the gate (the roster, the rules: every code and the 171 gate-cutover verdicts, and the scenario over the taskboard twin) and the upgrade is a procedure (`upgrade.sh`); [ACCEPTANCE.md](ACCEPTANCE.md) counts what they cover.** What makes "1:1 with GSD, version by version" a check rather than a hope.

| Check | Proves |
|---|---|
| roster | every command, agent, hook, capability and config key of the tracked gsd-core is contributed by one `@gsd/*` extension or listed as not applicable with a reason |
| scenario | a scripted scenario, recorded once from real gsd-core (`record.sh`) and replayed on SpecForge `@gsd/core` (profile `gsd-compat`), yields the same answers: the recorded goldens are compared with the twin's, derived values included (nothing runs real gsd-core in the gate) |
| rules | every rule in [rules.md](../core/spec/rules.md) fires on its fixture and stays silent on its counterpart |
| upgrade | for a new gsd-core release, the diff of its inventory manifest, `capability.json` files, templates, prompts and gate modules against the tracked one, written as a parity backlog |

`tracks` (the crate's `[package.metadata.gsd] tracks = "gsd-core@<version>"` and the handshake keyword `tracks:gsd-core@<version>`) moves only when roster, scenario and rules pass on that version. The fixture projects and
the version matrix live here.

## What is here

| Path | What |
|---|---|
| `upstream/v1.19.0/` | the tracked release's configuration roster, frozen so the roster check runs offline ([SOURCE.md](upstream/v1.19.0/SOURCE.md)); `core/tests/settings_roster.rs` reads it |
| `fixtures/taskboard/` | a small Node CLI with a GSD 1.18.0 `.planning/`: three phases and an inserted `02.1`, phase 1 executed, verified and run through UAT, phase 2 discussed, researched and planned; its three plans and the two summaries of phase 1 are in the twin's `spec/plans.spec` |
| `goldens/taskboard/` | what `gsd-tools` answers over that fixture: `roadmap analyze`, `phase-plan-index 1` and `2`, `state json`, `progress json`, `validate health`, `check.verify-context-drift 2` and `check.decision-coverage-plan` of phase 2; the plan verdicts of r37: `verify.plan-structure` of each plan, `check.verify-command-paths` and `check.verify-failure-directions` of phases 1 and 2, `check.gap-analysis-plan-post` of phases 1 and 2; `audit-uat` (r39), and `junit.xml`, what the fixture's own `npm test` reports (r38), with the machine's paths and timings normalised |
| `fixtures/vault/`, `goldens/vault/` | the hand-written security fixture: a phase with a threat model, an audit and an accepted risk, its `specforge-harness` ledger for the hand-written H fields, and what gsd-tools answered over it (hooks, config, threat IDs, the gate); `security/tests/cli/parity_scenario.rs` compares the twin with them |
| `record.sh` | re-records the goldens |
| `upstream/v1.19.0/gate-cutover/` | 171 verdicts of gsd-core's own gate-cutover fixtures (decision coverage, verify command paths, failing directions, post-planning gaps), copied unchanged; `core/tests/projects/gate_cutover/` is their SpecForge twin |

The taskboard's `verify-command-paths` and `verify-failure-directions` goldens are intentionally empty: every taskboard task has a bare `<verify>`, so there is no `<automated>` command to probe, and the comparison proves only that no blocker is reported. The E320 and E507 proof is the gate-cutover verdicts (`gate_cutover_plan_verdicts_reproduce`), whose blocker and missing rows are matched one for one. Adding an `<automated>` task to the taskboard would move other goldens, so it is not done.

The goldens are data: nothing runs gsd-tools in CI. `core/tests/cli/parity_roadmap.rs` compares the SpecForge twin in
`fixtures/taskboard/spec/` with `goldens/taskboard/roadmap-analyze.json`, `REQUIREMENTS.md` and `ROADMAP.md`. `core/tests/cli/parity_plans.rs` compares the twin's plans' waves, `autonomous`, dependencies, files and task counts with `phase-plan-index`, checks the plan verdict goldens against the twin's findings, and reproduces the gate-cutover verdicts on `core/tests/projects/gate_cutover/`. `core/tests/cli/parity_discuss.rs` compares the twin's contexts, decisions, discussion log and research with phase 2's `CONTEXT.md`, `DISCUSSION-LOG.md`, `RESEARCH.md`, PROJECT.md's Key Decisions and the two context goldens.

What `parity_roadmap.rs` compares: the phases `roadmap analyze` lists (number, name, goal, `depends_on`, requirements), every v1
and v2 requirement with its tier, and the `Out of Scope` rows with their reasons. What it does not: computed values (a
requirement's or phase's `status`, the progress percentages, `STATE.md`), which r38 derives and `parity_state.rs` compares. The twin's
milestone `v1.0` is synthetic: the fixture's GSD project is milestone-less (`"milestones": []`) and a SpecForge phase needs a
milestone to sit in; the test asserts both so a change to either is noticed.

`core/tests/cli/parity_state.rs` (r38) compares what execution derives: per phase, the plan and summary counts and the status label with `progress-json.json` (every phase equal: the twin carries the verifier's seven E501 verdicts on phase 01, committed in `fixtures/taskboard/spec/plans.spec` and `specforge-harness` by `GSD_BLESS=1 cargo test the_twin_holds_its_verifier_verdicts`; stripped of them, phase 01 is `Executed` where gsd-tools says `Complete`), the totals and the plan-based percent, and the counts of `state-json.json` that equal `milestone.progress` (`total_phases`, `total_plans`, `completed_plans`, and `completed_phases` 1 and `percent` 25). `core/tests/cli/parity_verify.rs` (r39) compares the twin's UAT (`spec/verify.spec`: three tests, one gap) with `audit-uat.json` and `01-UAT.md`: the one `issue` test by name and expected text, its gap's truth and reason, and the totals, parsed from the file's `## Summary`. One difference is named, not hidden: the session's status is `partial` where the file and the golden say `complete` (the file's writer set it when the run finished; the twin's is computed, and its one gap is open with no root cause). The verification is an equality: `01-VERIFICATION.md` says `passed`, `3/3` (the verifier merges the roadmap's two success criteria with the plans' must-haves, as `agents/gsd-verifier.md` says), and so does the twin; stripped of its verdicts it is `pending`, `0/3`. A report from `collect` stales the verdicts recorded over `none` tests, so a test that collects and then needs `Complete` runs the verifier again.

`parity_state.rs` also compares each twin summary and deliverable with the `SUMMARY.md` frontmatter, and, after `collect --runner gsd-junit --report goldens/taskboard/junit.xml`, each check's computed status with the SUMMARY's `status: pass`.

`core/tests/cli/parity_scenario.rs` (r44) is the scenario check: its `SCENARIO` table names, for each of the 19 goldens of `goldens/taskboard/`, the one test that compares the twin with it and the text that test reads it by. `every_taskboard_golden_is_compared_once` fails when a golden is added, removed or left unread, or when a named test is deleted or stops reading its golden; a golden that several tests read (`progress-json.json`) is named once, by its main comparison.

`upgrade.sh <gsd-core-dir> <tag>` (r44) is the upgrade check. It extracts the tag's inventory manifest, capability ids, config manifest and the list of files changed since the tracked tag (`git diff --name-status`), then runs `core/tests/upgrade.rs`'s `write_the_backlog`, which writes `backlog/<tag>.md`: one row per change, with its family, the upstream path and the table of `@gsd/core` it lands in (`ROSTER`, `AGENTS`, `HOOK_ROSTER`, the asset table, `NOT_EMBEDDED`, `MODULES`, the README capability table, `spec/settings.md`, `GATE_CUTOVER`), the files of `gsd/core/src` whose header cites the path, or `unowned`. It needs git and cargo, not the network, and reads the checkout without writing to it. Its other tests run offline on the vendored snapshot: the diff of the tracked tag against itself is empty, every path a header cites is in the snapshot, and `core/tests/declaration.rs` requires `tracks` to name the one vendored directory, so moving it fails the build until every check reads the new snapshot.

### Re-recording

```sh
(cd ~/Projects/OpenSource/gsd-core && git archive v1.18.0 | tar -x -C /tmp/gsd-core-1.18.0)
(cd /tmp/gsd-core-1.18.0 && npm ci --ignore-scripts && npm run build:lib)
GSD_CORE_DIR=/tmp/gsd-core-1.18.0 gsd/parity/record.sh
git diff --exit-code gsd/parity/goldens
```

The script refuses a `GSD_CORE_DIR` that is not 1.18.0, runs the verbs on a copy of the fixture that is its own git
repository (so `validate health` does not report the machine's worktrees), runs the fixture's `npm test` (node's JUnit
reporter) for `junit.xml`, and normalises timestamps, hashes, paths and timings, so a second run changes nothing. To track a newer release, change the tag in `upstream/` and here, re-record, and review
the diff of the goldens: that diff is the parity backlog.

### How the taskboard was made

Every verb that writes a `.planning/` file was run for real (`config-new-project`, `phase insert`, `scaffold phase-dir`,
`phase complete`, `verification fingerprint`, `commit`), and the narrative files a workflow's agents would write
(`PROJECT.md`, `REQUIREMENTS.md`, the plans, summaries, verification report and UAT) follow gsd-core's templates but were
written by hand: the workflows need a model session, which a recording script cannot run. The goldens are therefore
`gsd-tools` over a faithful `.planning/`, not a transcript of `/gsd-new-project`.

Phase 2's `02-CONTEXT.md`, `02-DISCUSSION-LOG.md` and `02-RESEARCH.md` (round r36) follow the v1.18.0 templates and were also written by hand: `workflow.research` is false in the fixture's config, so the research stands for a run of `/gsd-plan-phase 2 --research`. `02-01-PLAN.md` cites `D-01`..`D-03`, so the decision-coverage golden records 3 of 3 covered (GSD's blocking plan-phase gate passes). `gsd/E310` (r37) counts, as GSD does, every trackable `D-NN` of the phase's CONTEXT (a decision with `origin context`), not the `settled` ones; the twin's plan cites all three, so it is clean, and the twin's context, decisions and deferred idea stay as they are (any edit moves the committed `context_digest` literal and its ledger line).

## Known limits of the checks (r44 review)

- `upgrade.sh` does not extract the 86 capability-declared config keys, so a change to one shows only as a changed
  file. `locate` files the nested `src/<dir>/` modules and `bin/lib/*.cjs` as "other", which can raise a false
  "unowned" alarm. `upgrade_diff_of_the_tracked_release_is_empty` compares the snapshot with itself; a real extraction of the tag
  was compared once by hand.
- A "Reproduced" row passes when a test of that name contains the row's name, comments included; the pinned totals
  merge Reproduced and ByConstruction. The scenario check's needles are substrings. `GSD_BLESS=1` also rewrites
  `fixtures/taskboard/spec/*` in `parity_verify.rs`.
- The silent evidence of 38 of the 61 built codes is the unedited twin (`rules_silent.rs`), which does not set up the
  code's precondition.
- Four rows are analogues rather than reproductions: `verify-schema-drift-env-skip` goes through a setting,
  `decision-coverage-plan-context-absent` deletes decisions, `verify-context-drift-no-context-md` is probably
  reachable, and the four `decision-coverage-verify-*` rows say "not built in r44" though the verified phase now exists.
- The taskboard fixture's seven verdicts name `gsd-verifier@0.1.0` (recorded before the 1.0.0 bump); freshness ignores it.
- `install_hosts.rs` normalises any `@gsd/core@<x>` to `VERSION`; the version itself is pinned in `declaration.rs` and
  `main.rs`.
