# Parity suite

**Status: the config roster, the taskboard roadmap and the taskboard's discussed and researched phase are built; the rest is being designed.** What makes "1:1 with GSD, version by version" a check rather than a hope.

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
| `fixtures/taskboard/` | a small Node CLI with a GSD 1.18.0 `.planning/`: three phases and an inserted `02.1`, phase 1 executed, verified and run through UAT, phase 2 discussed, researched and planned |
| `goldens/taskboard/` | what `gsd-tools` answers over that fixture: `roadmap analyze`, `phase-plan-index 1` and `2`, `state json`, `progress json`, `validate health`, `check.verify-context-drift 2` and `check.decision-coverage-plan` of phase 2 |
| `record.sh` | re-records the goldens |

The goldens are data: nothing runs gsd-tools in CI. `core/tests/cli/parity_roadmap.rs` compares the SpecForge twin in
`fixtures/taskboard/spec/` with `goldens/taskboard/roadmap-analyze.json`, `REQUIREMENTS.md` and `ROADMAP.md`. `core/tests/cli/parity_discuss.rs` compares the twin's contexts, decisions, discussion log and research with phase 2's `CONTEXT.md`, `DISCUSSION-LOG.md`, `RESEARCH.md`, PROJECT.md's Key Decisions and the two context goldens.

What `parity_roadmap.rs` compares: the phases `roadmap analyze` lists (number, name, goal, `depends_on`, requirements), every v1
and v2 requirement with its tier, and the `Out of Scope` rows with their reasons. What it does not: computed values (a
requirement's or phase's `status`, the progress percentages, `STATE.md`), which no round before r38 derives. The twin's
milestone `v1.0` is synthetic: the fixture's GSD project is milestone-less (`"milestones": []`) and a SpecForge phase needs a
milestone to sit in; the test asserts both so a change to either is noticed.

### Re-recording

```sh
(cd ~/Projects/OpenSource/gsd-core && git archive v1.18.0 | tar -x -C /tmp/gsd-core-1.18.0)
(cd /tmp/gsd-core-1.18.0 && npm ci --ignore-scripts && npm run build:lib)
GSD_CORE_DIR=/tmp/gsd-core-1.18.0 gsd/parity/record.sh
git diff --exit-code gsd/parity/goldens
```

The script refuses a `GSD_CORE_DIR` that is not 1.18.0, runs the verbs on a copy of the fixture that is its own git
repository (so `validate health` does not report the machine's worktrees), and normalises timestamps and hashes, so a
second run changes nothing. To track a newer release, change the tag in `upstream/` and here, re-record, and review
the diff of the goldens: that diff is the parity backlog.

### How the taskboard was made

Every verb that writes a `.planning/` file was run for real (`config-new-project`, `phase insert`, `scaffold phase-dir`,
`phase complete`, `verification fingerprint`, `commit`), and the narrative files a workflow's agents would write
(`PROJECT.md`, `REQUIREMENTS.md`, the plans, summaries, verification report and UAT) follow gsd-core's templates but were
written by hand: the workflows need a model session, which a recording script cannot run. The goldens are therefore
`gsd-tools` over a faithful `.planning/`, not a transcript of `/gsd-new-project`.

Phase 2's `02-CONTEXT.md`, `02-DISCUSSION-LOG.md` and `02-RESEARCH.md` (round r36) follow the v1.18.0 templates and were also written by hand: `workflow.research` is false in the fixture's config, so the research stands for a run of `/gsd-plan-phase 2 --research`. The decision-coverage golden records 0 of 3 decisions covered, because `02-01-PLAN.md` cites none of `D-01`..`D-03`; the plan-coverage rule `gsd/E310` (r37) needs the plan to cite them.
