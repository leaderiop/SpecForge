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
- `specforge-harness`: the ledger line of the twin research's `context_digest`.
