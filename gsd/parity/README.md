# Parity suite

**Status: the config roster, the taskboard roadmap, the taskboard's discussed and researched phase, its plans, and its executed phase (summaries, progress and the recorded tests) and its UAT are built; the rest is being designed.** What makes "1:1 with GSD, version by version" a check rather than a hope.

| Check | Proves |
|---|---|
| roster | every command, agent, hook, capability and config key of the tracked gsd-core is contributed by one `@gsd/*` extension or listed as not applicable with a reason |
| scenario | a scripted scenario run by real gsd-core and by SpecForge `@gsd/*` (profile `gsd-compat`) yields the same graph: GSD's `.planning/` output is imported, then compared entity by entity, derived values included |
| rules | every rule in [rules.md](../core/spec/rules.md) fires on its fixture and stays silent on its counterpart |
| upgrade | for a new gsd-core release, the diff of its inventory manifest, `capability.json` files, templates, prompts and gate modules against the tracked one, written as a parity backlog |

`tracks` (the crate's `[package.metadata.gsd] tracks = "gsd-core@<version>"` and the handshake keyword `tracks:gsd-core@<version>`) moves only when roster, scenario and rules pass on that version. The fixture projects and
the version matrix live here.

## What is here

| Path | What |
|---|---|
| `upstream/v1.18.0/` | the tracked release's configuration roster, frozen so the roster check runs offline ([SOURCE.md](upstream/v1.18.0/SOURCE.md)); `core/tests/settings_roster.rs` reads it |
| `fixtures/taskboard/` | a small Node CLI with a GSD 1.18.0 `.planning/`: three phases and an inserted `02.1`, phase 1 executed, verified and run through UAT, phase 2 discussed, researched and planned; its three plans and the two summaries of phase 1 are in the twin's `spec/plans.spec` |
| `goldens/taskboard/` | what `gsd-tools` answers over that fixture: `roadmap analyze`, `phase-plan-index 1` and `2`, `state json`, `progress json`, `validate health`, `check.verify-context-drift 2` and `check.decision-coverage-plan` of phase 2; the plan verdicts of r37: `verify.plan-structure` of each plan, `check.verify-command-paths` and `check.verify-failure-directions` of phases 1 and 2, `check.gap-analysis-plan-post` of phases 1 and 2; `audit-uat` (r39), and `junit.xml`, what the fixture's own `npm test` reports (r38), with the machine's paths and timings normalised |
| `record.sh` | re-records the goldens |
| `upstream/v1.18.0/gate-cutover/` | ten verdicts of gsd-core's own gate-cutover fixtures (decision coverage, verify command paths, failing directions, post-planning gaps), copied unchanged; `core/tests/projects/gate_cutover/` is their SpecForge twin |

The taskboard's `verify-command-paths` and `verify-failure-directions` goldens are intentionally empty: every taskboard task has a bare `<verify>`, so there is no `<automated>` command to probe, and the comparison proves only that no blocker is reported. The E320 and E507 proof is the gate-cutover verdicts (`gate_cutover_plan_verdicts_reproduce`), whose blocker and missing rows are matched one for one. Adding an `<automated>` task to the taskboard would move other goldens, so it is not done.

The goldens are data: nothing runs gsd-tools in CI. `core/tests/cli/parity_roadmap.rs` compares the SpecForge twin in
`fixtures/taskboard/spec/` with `goldens/taskboard/roadmap-analyze.json`, `REQUIREMENTS.md` and `ROADMAP.md`. `core/tests/cli/parity_plans.rs` compares the twin's plans' waves, `autonomous`, dependencies, files and task counts with `phase-plan-index`, checks the plan verdict goldens against the twin's findings, and reproduces the gate-cutover verdicts on `core/tests/projects/gate_cutover/`. `core/tests/cli/parity_discuss.rs` compares the twin's contexts, decisions, discussion log and research with phase 2's `CONTEXT.md`, `DISCUSSION-LOG.md`, `RESEARCH.md`, PROJECT.md's Key Decisions and the two context goldens.

What `parity_roadmap.rs` compares: the phases `roadmap analyze` lists (number, name, goal, `depends_on`, requirements), every v1
and v2 requirement with its tier, and the `Out of Scope` rows with their reasons. What it does not: computed values (a
requirement's or phase's `status`, the progress percentages, `STATE.md`), which r38 derives and `parity_state.rs` compares. The twin's
milestone `v1.0` is synthetic: the fixture's GSD project is milestone-less (`"milestones": []`) and a SpecForge phase needs a
milestone to sit in; the test asserts both so a change to either is noticed.

`core/tests/cli/parity_state.rs` (r38) compares what execution derives: per phase, the plan and summary counts and the status label with `progress-json.json` (phases 02, 02.1 and 03 equal; phase 01 is `Executed` where gsd-tools says `Complete`, because `01-VERIFICATION.md` passed and the verdict that says so is an E501 judgement, r40's), the totals and the plan-based percent, and the three counts of `state-json.json` that equal `milestone.progress` (`total_phases`, `total_plans`, `completed_plans`); `completed_phases` (1) and `percent` (25) are verification-based and differ today (0 and 0), until r40 records the verdicts. `core/tests/cli/parity_verify.rs` (r39) compares the twin's UAT (`spec/verify.spec`: three tests, one gap) with `audit-uat.json` and the Summary of `01-UAT.md`: the one `issue` test by name and expected text, its gap's truth and reason, and the totals 3/2/1. The twin has no `verification` entity: `01-VERIFICATION.md`'s verdicts (the fingerprint, `covered_digest`) are not compared until r40. It compares each twin summary and deliverable with the `SUMMARY.md` frontmatter, and, after `collect --runner gsd-junit --report goldens/taskboard/junit.xml`, each check's computed status with the SUMMARY's `status: pass`.

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
