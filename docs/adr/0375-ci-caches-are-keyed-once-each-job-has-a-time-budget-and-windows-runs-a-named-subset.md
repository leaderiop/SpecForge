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

## D1. Each cache is keyed once, under one prefix

`ci.yml` defines one workflow-level `CACHE_PREFIX` and passes it as rust-cache's `prefix-key` on every step. One bump
of it starts every cache anew, and a change to a cache's path list bumps it: a changed path list is a new cache
version, which orphans the old entry, as the `build` job's did. Every step names a `shared-key` instead of `key`, so
the saved key carries no job id and a restore-only step can name a saver's key. The savers are `lint`, `gsd-crates`,
`specs`, `build` and `effect-plane`, and `windows-subset`, the job D4's split made, each saved only on main.
`dogfood` restores `specs`. The `tests` jobs carry no cache: they compile only `cargo gate`, and a restore of a
2 GB cache costs more than that compile. `coverage.yml` and `release.yml` carry no cache step: the first would save an
instrumented multi-GB cache the day its job passes, the second would save caches scoped to a tag, which nothing on main
can read, and no job saves a key either could restore. `perf.yml` keeps its own nightly cache. A pull request saves nothing and reads main's.

The reason: with a `key` and no prefix, a job's key is its job id, so a restore-only job restored a key no job saved,
and nothing could start every cache anew at once; one prefix is one bump, and a `shared-key` names a cache by what it
holds.

## D2. Main keeps one generation of each cache

A `cache-prune` job, on pushes to main only and after the saving jobs, with `actions: write` and `contents: read` and no
other permission, lists main's cache entries and deletes every entry that a newer entry of its group supersedes. A group
is the key without its two trailing hashes, the toolchain's and the lockfile's; a key of any other shape is never
grouped and so never deleted. The previous prefix's entries are deleted once, by the maintainer, after the first main
run on the new prefix, and not by CI: the prune job leaves each old group's newest entry alone.

The reason: GitHub evicts the least recently used entries once the total passes its cap, and one a run restored counts
as used, so a superseded generation stays alive and pushes the total past it; deleting a cache is a destructive act on
a shared resource, so the job's reach is main's pushes, one group's older entries and nothing else.

## D3. Each job has a time budget under its timeout

A first step records the clock and a last step fails the job with an `::error` when it took longer than its
`BUDGET_SECONDS`. A budget is the smaller of 1.25 times the job's cold wall time rounded up to a whole minute, and its
timeout less 300 seconds, set from this change's own cold run. A job a pull request cannot run takes its budget from its
runs on main (`dogfood`, which has never had a cache hit) or from its cap (`cache-prune`). The timeouts are unchanged:
`ci` and `build` 40 minutes, `tests` and `effect-plane` 30, `dogfood` 20, `bootstrap` 45; the two jobs this change adds
take `windows-subset` 30 and `cache-prune` 10. Gate tests hold every budget under its cap, every cap under its timeout,
and the key rules of D1. They also hold that nothing lets a run pass past a budget: the clock and the check are each
exactly `name`, `shell: bash` and `run`, and no job has a `continue-on-error` of its own.

The reason: a timeout cuts a job at the cap and says only that it ran long; a budget under it fails the run while the
job still finishes, and so names the job that slipped before a cold run lands on the timeout.

## D4. The Windows leg runs a named subset

On `windows-latest`, `ci.yml` runs three named steps. They run in a job of their own, `windows-subset` (`Windows
subset`, 30 minutes, its own cache key saved only on main), because the effect-plane leg would have passed 1,500 s with
them: its worst cold time, 1,413 s, plus the steps' 52 s, 300 s and 503 s on their first green run there (CI run
44185508652) is 2,268 s. The effect-plane legs keep their two steps. What ran, on CI run 44190098653, the job's first
run, with no cache (1,467 s):

| Step | Filter | Tests run | What it covers |
|------|--------|-----------|----------------|
| `Windows subset: checked_tree` | `cargo nextest run --no-fail-fast -p gate -E 'test(/^checked_tree::/)'` | 4 tests run: 4 passed | the copy of the tracked tree, its times and modes, and a destination over 260 characters |
| `Windows subset: USERPROFILE beside HOME` | `cargo nextest run --no-fail-fast -p specforge-component --lib -E 'test(/user_compile_cache/)'` | 6 tests run: 6 passed | the user's home (D5), with a drive-letter case that only Windows compiles |
| `Windows subset: absorb` | `cargo nextest run --no-fail-fast -p xtask --bin absorb --test absorb_e2e` | 96 tests run: 96 passed | absorb's unit and end-to-end tests, including the three symbolic-link refusals and a target over 260 characters |

Before checkout the job sets `git config --global core.autocrlf false` and `core.longpaths true`, in bash so a failed
config fails the step, so a checkout keeps the line endings the tests expect and a long path is not cut by git. A
Windows-only nextest override gives `binary_id(xtask::absorb_e2e)` a longer kill time, since each of its tests loads a
wasm extension. A symbolic link a test cannot create fails the test; a directory falls back to a junction, which std
reads as a symbolic link too. Every symbolic-link test made its links: the three refusals passed, and their helpers
panic rather than return without a link. nextest keeps a passing test's output, so the log does not say whether a
directory link was a junction. Nothing is skipped.

The subset found two defects in SpecForge on its first runs, both fixed where they were, not in the tests. A wasm
engine built with native unwind information off is refused by wasmtime on Windows, so every test that loads an
extension failed: the engine now keeps it on Windows only. An import resolved its candidate path with the OS's
separator, while source keys are joined by `/`, so every import across directories was E025 on Windows: a candidate is
now keyed as a source key is minted, a relative name joined by `/` on every OS and an absolute path as the OS writes it.
Two absorb tests compared a printed path with a `/` literal; they now build the expected path with the OS's separator.

What stays `cfg(unix)`, each with its reason (getting the leg green added no test to the list):

- `xtask/src/revendor.rs`'s three pinned-rustc tests and their helper: they run `sh` as a stand-in for `rustc`, and
  revendor is a maintainer tool whose only CI home is the Linux bootstrap job;
- `gate/src/run/tests.rs`'s `a_failed_answer_keeps_what_it_printed`: it runs `sh`, and the gate runs only on Linux;
- the two exec-bit blocks in `gate/src/checked_tree/tests.rs`: Windows has no exec bit, and its permission, read-only,
  is covered by the test beside them;
- `crates/adapters/registry-client/tests/home.rs`: a guard on that suite's own isolation, and the crate is not on the
  Windows leg;
- `xtask/src/build_blobs.rs` and `xtask/src/perf_report.rs` set only `HOME`: both are Linux-only tools.

The rest of the workspace's `cfg(unix)` code is outside the named subset. One import moved under `cfg(unix)` on the
way, and no test with it: the top-level import of `Ceiling` in `crates/adapters/process/tests/runner.rs` served only a
test that was already `cfg(unix)`, which now imports it itself, so the Windows build of that suite no longer warns.

The reason: the leg tests the host's file effects, and the copy, the compile cache's directory and absorb's writes are
the code that touches Windows paths, links and line endings; a subset that fails on a link it cannot make, and not one
that skips it, cannot pass on a platform it never exercised.

## D5. The user's home is HOME, else USERPROFILE, absolute only

The compile cache's directory (`user_compile_cache_dir`) reads the home as the effect journal, the consent store and the
registry files already do: `HOME`, else `USERPROFILE`, and a relative home is no home. Each reader's lookup is a pure
function its tests call with values, never by setting the process environment.

The reason: with `HOME` unset, as in a Windows shell, the wasm compile cache was off and every test that loads a builtin
compiled it again; three readers agreeing and one not is a defect of the one, and a test that sets the process
environment races every other test in its process.

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

## As built

Measured on 2026-11-18. The decisions above are as decided; where the Context's figures and these differ, these are
the record.

**The cold runs.** CI run 44205447318 and Bootstrap run 44205447468, on `2bbb79512`, the commit that set
`CACHE_PREFIX` to `v1-rust`, are the cold run: every job with a cache step logged only `v1-rust-` keys and found no
cache. The first budgets were D3's formula over that one run's wall times (the job's `completed_at` less its
`started_at`), set on `7f35d7a66`, whose run, CI 44209528364 and Bootstrap 44209528354, was cold again (a pull request
saves nothing) and passed every budget step. The next run, CI 44214080378 and Bootstrap 44214080385 on `8a1bc7cc0`,
which changed only this record, failed two: gsd took 1,602 s of 1,440 s and Bootstrap 1,226 s of 1,140 s. One cold
sample was too few: on the same code gsd ran in 1,054 s and in 1,602 s, 52 % apart. Every budget was set again by the
same formula over the slowest wall time seen on the current code: the three runs above, the runs since the Windows
subset got its job (CI 44190098653, 44196595140 and 44200960705; Bootstrap 44196595045 and 44200960703, and the five
Bootstrap runs before them since the Windows fixes, up to 1,360 s), and the 80-run history before this change. No
timeout changed.

| Job | Cold run (s) | Confirmation (s) | Third run (s) | Slowest seen (s) | First budget (s) | Budget (s) | Timeout (s) |
|-----|-------------:|-----------------:|--------------:|-----------------:|-----------------:|-----------:|------------:|
| `ci` lint | 238 | 249 | 240 | 1,135, the history | 300 | 1,440 | 2,400 |
| `ci` gsd | 1,112 | 1,054 | 1,605 | 1,605 | 1,440 | 2,040 | 2,400 |
| `ci` specs | 1,570 | 1,110 | 1,194 | 1,592, the history | 1,980 | 2,040 | 2,400 |
| `build` | 1,141 | 880 | 709 | 1,319, the history | 1,440 | 1,680 | 2,400 |
| `tests`, slowest of four partitions | 295 | 356 | 325 | 413 | 420 | 540 | 1,800 |
| `effect-plane` windows-latest | 1,369 | 1,292 | 1,388 | 1,413, the history | 1,500, its cap | 1,500, its cap | 1,800 |
| `effect-plane` macos-latest | 782 | 1,012 | 894 | 1,086, the history | 1,020 | 1,380 | 1,800 |
| `windows-subset` | 1,318 | 1,087 | 1,043 | 1,467, its first run | 1,500, its cap | 1,500, its cap | 1,800 |
| Bootstrap | 910 | 1,065 | 1,231 | 1,360 | 1,140 | 1,740 | 2,700 |
| `dogfood` (main only) | none on a pull request | | | 637, main's 28 runs | 840 | 840 | 1,200 |
| `cache-prune` (main only) | none on a pull request | | | none | 300, its cap | 300, its cap | 600 |

Two budgets sit at their cap, both on Windows: the effect-plane leg and `windows-subset`. The effect-plane Windows leg
is the job closest to its timeout, 387 s under it at its slowest, and closest to its budget, 119 s under it on the
third run. On a warm run the same jobs take far less: CI run 44200960705, which restored main's caches of the
previous prefix, took 107 s for lint, 814 s for gsd, 668 s for specs, 512 s for the Windows leg and 278 s for macOS.

**Known risks, each with its trigger.** Runner speed varies by more than the formula's quarter: gsd by 52 % and macOS
by 29 % between cold runs of the same code, which is why a budget now comes from the slowest run seen and not from one.
The two Windows budgets cannot rise: a faster cold time, or main's warm cache, is what widens them. Lint's budget comes
from its 80-run history and sits far above what its shard takes today, so it catches only a large slip. A cold run past a budget has that budget set again by the same formula over the slowest run seen, never by
raising a timeout; a cold job past its budget twice after that reopens this record (below).

**Windows.** The subset runs in `windows-subset`, because D1's estimate for the effect-plane leg with the subset was
2,268 s, over 1,500 s. It has been green on CI run 44190098653 (its first, 1,467 s with no cache), on the cold run
(1,318 s: `checked_tree` 88 s, `USERPROFILE` 706 s, absorb 477 s with 96 of 96 tests), on the confirmation run and on the third run.
Getting the leg green took these commits, each at its source:

- `4e6d5a98c`: the wasm engine keeps native unwind information on Windows, where wasmtime refuses an engine without it;
- `cf3c6a9bd`: an import's candidate key joined by `/`, so imports across directories resolve on Windows. It made an
  absolute spec root on Linux `//tmp/...`, which a Linux test caught; `51d9cb739` fixed that by minting the candidate
  key as a source key is minted;
- `eb70dc17c` and `afe81f710`: two absorb refusal tests build the held path with the OS's separator;
- `e4d095537`: `Ceiling` is imported only by the `cfg(unix)` test that uses it;
- `c8f50af24`: the containment and reach tests of `specforge-ops` that link the Unix way now compile only on Unix,
  which removes seven warnings from the Windows leg's build. They were Unix-only tests before; Linux runs the same 39.

The resolver's own tests are not in the subset: on Windows an import across directories under a relative spec root is
held end to end, by absorb's tests, whose fixture's sources import across directories and whose target checks clean.
An absolute spec root on Windows, whose keys are written `\\?\C:\...`, has no Windows test; a `windows-subset` step
running `cargo nextest run -p specforge-resolver --test tests -E 'test(/^spec_dir::/)'` (the crate's one test
binary, `spec_dir` a module of it), with a case keyed `\\?\C:\...`, would close it once the job's budget has room for
it.

No test the subset names was made Unix-only. The gate's three workflow tests that run a step's own script under bash
(two the budget check, one the clock) are `cfg(unix)`: the gate runs on Linux.

**The prune program's dry run.** Read at 2026-11-18T01:04:47Z, without deleting anything: main held 12 entries,
11,374,888,604 B (11.37 GB). `cache-prune`'s `STALE_CACHES` program over them selects 5 entries, the superseded
generations of 2026-11-14 of lint, gsd, specs and both effect-plane legs, 4,427,543,229 B; an independent grouping of
the same list selects the same 5. After them, 7 entries remain, 6,947,345,375 B. Its first deletions happen on main's
first push after this change; a pull request never runs it.

**Projected, confirmed after the merge.** A pull request saves no cache and does not run `dogfood` or `cache-prune`, so
these figures are labelled projected, confirmed after the merge: projections from what was measured on the day, each
against 10,000,000,000 B (10 GB; GitHub's cap is 10 GiB, 10,737,418,240 B, so 10 GB is the stricter reading).

| Total | Bytes | Against 10,000,000,000 B |
|-------|------:|--------------------------|
| measured on 2026-11-18 at 01:04:47 UTC, 12 entries, all of the previous prefix | 11,374,888,604 (11.37 GB) | over |
| after `cache-prune`'s selection on that list | 6,947,345,375 (6.95 GB) | under |
| from the first main run until the previous prefix's entries are deleted | 13,894,690,750 (13.89 GB) | over |
| one generation per key, the steady state | 6,947,345,375 (6.95 GB) | under |

The steady state sums the newest entry of each of main's 7 groups, today's sizes standing in for the new prefix's
(the `build` job's old `tests-build` entry for its new key). The new prefix adds one key no entry has yet,
`windows-subset`; at the Windows effect-plane cache's size, 678,343,286 B, the steady state would be about 7.63 GB,
still under. The total until the deletion counts the previous prefix's survivors and a full new generation side by
side, and is expected over the cap, so GitHub may evict the least recently used entries until the maintainer deletes
the previous prefix's entries (D2); that deletion is the maintainer's first check after the merge, and perf's nightly
cache, which keeps rust-cache's default prefix, is not among them. `dogfood`'s slowest of 28 runs on main was 637 s,
against its 840 s budget and its 1,200 s timeout; it has never had a cache hit, and now restores `specs`, so its first
main run is expected faster than that.

**Deviations from the decisions as first planned.** The switch from `key` to `shared-key` landed in the commit that
bumped the prefix, not before it: renaming the saved keys makes every job as cold as the bump does, so one cold run
measures both. Bootstrap's first budget, 1,080 s from an older run, failed its own step on its first run (1,297 s),
the budget check's first live trip; it was set again from the runs on the code of the day, then from the cold run,
and last from its slowest run seen. D3 sets a budget from "this change's own cold run"; one cold run proved too few,
so every budget is now set from the slowest of the cold runs this change made and the history before it.

## Rejected

- **One cache for lint, specs and build.** Their `target` contents differ by profile, one saver would make the three
  wait on each other, and each would restore more than 3 GB.
- **GitHub's lazy eviction instead of a prune job.** A restored generation counts as used, so it outlives its
  replacement and the total passes the cap.
- **A gate check for the budgets.** The effect plane never runs `cargo gate`, and the job's own clock, with its cache
  restore, installs and downloads, is what the timeout cuts.
- **Raising a timeout.** It hides a slow job instead of naming it.
- **A composite action for the two budget steps.** A local composite action needs the checkout first and cannot have a
  post step, so it would still be two `uses:` lines per job, against three lines inline.
- **A YAML parser in the gate.** The gate keeps its two dependencies; its tests read the workflow line by line, as the
  bootstrap job's timeout test does.
- **Editing `.gitattributes` for line endings.** The root-layout check describes that file's one line, and a Windows
  checkout is made right by `core.autocrlf` before checkout, with no tracked change.
- **A separate Windows job from the start.** It costs a second cache and a second runner for a subset that may fit;
  D4 splits it when the cold time says so.
- **Fixing perf-report in this change.** The cost is a compile across three crates (D6).
- **Widening `dogfood` to pull requests to measure it.** It has never had a cache hit, so its history on main is its
  cold evidence.

## Consequences

- Every cache is named by what it holds, and one edit of `CACHE_PREFIX` makes every job cold at once.
- A pull request after a prefix bump is cold on every push, because a pull request saves nothing and reads main's
  entries; the bump is the last change of the work it measures.
- Main's total cache is one generation of each key, which is under the 10 GiB cap.
- A job past its budget fails the run before it reaches its timeout.
- A run takes one more Windows runner, the `windows-subset` job, and the effect-plane legs keep their time.
- perf-report keeps warning every night until D6's trigger is met.

## Would reopen it

- A cold job past its budget twice.
- rust-cache changing how it composes a key.
- The Windows leg's cold time past 1,500 s.
- D6's trigger.
