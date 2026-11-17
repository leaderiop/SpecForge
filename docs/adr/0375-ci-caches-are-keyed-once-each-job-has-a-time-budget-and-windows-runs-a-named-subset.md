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
2 GB cache costs more than that compile. `coverage.yml` and `release.yml` save nothing (`save-if: false`): the first
would save an instrumented multi-GB cache the day its job passes, the second saves caches scoped to a tag, which nothing
on main can read. `perf.yml` keeps its own nightly cache. A pull request saves nothing and reads main's.

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
runs on main (`dogfood`, which has never had a cache hit) or from its cap. The timeouts are unchanged: `ci` and `build`
40 minutes, `tests` and `effect-plane` 30, `dogfood` 20, `bootstrap` 45. Gate tests hold every budget under its cap,
every cap under its timeout, and the key rules of D1.

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
