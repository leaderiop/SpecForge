# Taskboard (fixture)

A task board for the command line: `node src/cli.js add <title>`, `node src/cli.js done <id>`. It exists to give the
parity suite a GSD 1.18.0 project to read; see [gsd/parity/README.md](../../README.md) for how `.planning/` was made
and how the goldens are recorded.

- `.planning/` is the GSD project: milestone-less, phases 1, 2, 2.1 (inserted) and 3. Phase 1 is executed, verified and
  has a UAT session; phase 2 has a context, a discussion log and a research and is planned; 2.1 and 3 are empty.
- `src/`, `test/`: what phase 1 built. `npm test` runs `node --test` and writes `junit.xml` (ignored); `record.sh` records it in `goldens/taskboard/junit.xml`.
- `spec/`: the SpecForge twin of the roadmap (project, requirements, milestone and phases) and of phase 2's discussion
  (`discuss.spec`) and of the three `PLAN.md` files (`plans.spec`: a bare `<verify>` is `verify_text`) and of phase 1's two `SUMMARY.md` files and their deliverables, compared with the
  goldens and `.planning/` by `gsd/core/tests/cli/parity_roadmap.rs`, `parity_discuss.rs`, `parity_plans.rs` and `parity_state.rs`.
- The truths name no requirement and no test: `specforge trace TASK-01` follows the requirement through GSD's own
  fields to its plan, tasks, the commits that name them and the items those changed, the plan's truth, and the
  summary's coverage item (`deliverable`), whose check names the test (`test/tasks.test.js#<test name>`, matched to a
  fact `@code/core` serves once `@javascript/node-test` has collected it).
- `specforge-harness`: the ledger lines of phase 1's seven verifier verdicts (recorded over the collected tests) and of
  the UAT results and the twin research's `context_pack`. Each truth's verdict cites the files its truth is about
  (`src/tasks.js`, `test/tasks.test.js`), so an edit to `src/cli.js` stales no truth, only 01-02's artifact and key
  link that name it; an artifact or a key link cites nothing and reads the files it names.
