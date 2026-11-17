# CI's caches are keyed once, each job has a time budget, and Windows runs a named subset

**Status:** accepted (2026-11-17). Builds on [ADR 0066](0066-one-check-list.md) (one check list, whose shards CI
runs), [ADR 0335](0335-the-host-owns-every-effect-through-one-effect-plane.md) (the effect plane's legs on the other
two operating systems) and [ADR 0353](0353-a-pass-reads-the-facts-it-declares.md) D8 (perf is measured, never asserted
in time). It changes no protocol, SDK or host rule.

## Context

Four loose ends from the swap round meet in CI. Every figure below was measured on 2026-11-17 at `f7d90cf8c`.

- **The caches.** CI saves seven distinct cache keys from main, so no two jobs save the same key. But `tests` and
  `dogfood` restore a key no job saves, `lint`, `specs` and `build` each cache the same workspace's `target` under their
  own key, and a changed path list made a second entry for `build`. The repository's active caches totalled
  11,374,888,604 bytes in 12 entries, over GitHub's 10 GiB cap (10,737,418,240 bytes), because the superseded
  generations of 2026-11-14 were still alive.
- **The time.** No job has a budget under its timeout. The slowest cold runs of the last 80 were 1,605 s (`gsd`) and
  1,592 s (`specs`) of 2,400 s, and 1,413 s for the Windows effect plane of 1,800 s. A new rustc makes every job cold at
  once, so a budget has to hold cold.
- **Windows.** The Windows leg runs the effect plane's two steps and nothing from `gate` or `xtask`. The user's
  compile cache reads `HOME` alone, so on a Windows account with no `HOME` it is off, where the effect journal, the
  consent store and the registry files already read `USERPROFILE` too.
- **perf-report.** `cargo run -p xtask --bin perf-report` found, in the swap round, that a warm collect is slower than a
  cold one and over its budget. D6 records why.

## D6. perf-report's warm collect is over its budget, and the reason is recorded, not fixed

`perf-report` was run again at `f7d90cf8c` (the `sha` its report names), three runs per step, on a 32-core Linux
machine, on a copy of the tracked tree with a fresh `HOME`. It exits 1, as it did in the swap round. The second of two
back-to-back runs, which is the report on disk:

| Step | Measured | Budget | Work counted |
|------|----------|--------|--------------|
| check, no facts | 187 ms | none | no facts |
| collect cold | 2,591 ms | 10,000 ms | 2,714 files parsed, 3 read from the syntax cache, 39,872 facts |
| collect warm | 11,891 ms | 2,000 ms | 0 parsed, 2,717 read from the syntax cache |
| collect, one edit | 12,049 ms | none | 1 parsed, 2,716 read from the syntax cache |
| check, facts served | 2,701 ms | 2,000 ms | 39,872 facts |

The first run read 11,955 ms for the warm collect and 2,684 ms for `check` with facts served, so the figure is steady on
this machine. It is not steady between machines or days: the research for this decision measured the same steps at
4,209 ms warm and 2,626 ms for `check` on a scratch copy the same day, and the swap round's own report read 11.9 s warm.
What does not move is the order: a warm collect is slower than a cold one, and both it and `check` with facts served
are over their budgets. On the nightly's runner, three reads of the nightly `perf.yml` report show the same:

| Nightly read | Collect warm | Collect cold |
|--------------|--------------|--------------|
| first | 6,580 ms | 6,265 ms |
| second | 7,058 ms | 6,177 ms |
| third | 6,384 ms | 5,940 ms |

**The diagnosis.** A warm collect is slower than a cold one because a collect that has a previous collect's facts first
compiles the project with those facts served to it as read-only entities, and only then collects. That compile is the
same work as `check` with facts served (2.6 s against 0.2 s with no facts, on one thread, about 1.1 GB), and a cold
collect has no facts to serve, so it skips the compile and pays for parsing instead, on every core. Three experiments
on the research's run isolate it:

| Experiment | Wall | User / system | Peak memory |
|------------|------|---------------|-------------|
| cold collect (no `.specforge/`) | 1.82 s | 6.51 s / 1.15 s | 395 MB |
| warm collect (facts store and syntax cache present) | 4.30 s | 4.06 s / 3.17 s | 1,162 MB |
| warm collect with the fact store removed, syntax cache kept | 1.68 s | 1.67 s / 3.03 s | 337 MB |

The syntax cache is not the cost: 2,717 cached files read in parallel take 1.68 s. Of thirty stack samples of the
main thread, 17 sit in the project compile (the graph build and its settle, the registry's checks, the compute pass),
7 in the guest call and its encoding, 5 elsewhere and 1 in a directory listing.

**Why it is not fixed here.** `collect` is handed a project already compiled with the facts served. Compiling without
the previous stores crosses the CLI, `ops::collect` and `specforge-project`, and whether a collector's inputs need the
served entities is unverified: the collector reads the stores itself through `recorded_facts`, but its use of the known
entities is not checked. Cutting the compile itself is a performance change to the graph build, a project of its own.

**Owner and trigger.** The owner is the maintainer. The finding is reopened when any one holds:

- the nightly warning persists on a night after the compile with facts served has got cheaper;
- `check` with facts served passes 5 s on the runner;
- the root's collected facts pass 60,000 (39,872 at this record).

ADR 0353 D8's budgets (collect warm 2 s, `check` 2 s) are unchanged, and the nightly keeps warning. Re-basing them to
what the runner measures is the owner's decision, not this change's.

The reason: a measured, reproducible gap with a known cause and no local fix is worth more written down with an owner
and a trigger than patched across three crates in a phase about CI time; and a budget that nothing can meet yet is a
warning to keep, not a number to move to quiet it.
