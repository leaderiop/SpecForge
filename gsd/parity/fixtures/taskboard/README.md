# Taskboard (fixture)

A task board for the command line: `node src/cli.js add <title>`, `node src/cli.js done <id>`. It exists to give the
parity suite a GSD 1.18.0 project to read; see [gsd/parity/README.md](../../README.md) for how `.planning/` was made
and how the goldens are recorded.

- `.planning/` is the GSD project: milestone-less, phases 1, 2, 2.1 (inserted) and 3. Phase 1 is executed, verified and
  has a UAT session; phase 2 is planned; 2.1 and 3 are empty.
- `src/`, `test/`: what phase 1 built. `npm test` runs `node --test` and writes `junit.xml` (ignored).
