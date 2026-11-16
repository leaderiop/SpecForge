# @gsd/core is vendored as a builtin, with a size allowance of its own

**Status:** accepted (2026-11-16). **Supersedes**
[gsd ADR 0001](../../gsd/adr/0001-one-cargo-workspace-built-on-demand.md) in these places only: D4 ("The blob is not
committed; it is built on demand"); its Rejected item "A committed blob listed in `COMMITTED`"; D6's step "the wasm
build", its "with both variables set" (`GSD_CORE_WASM` leaves, `SPECFORGE_BIN` stays) and the Consequences clause "and a
release wasm"; D5's "that load the built blob (`GSD_CORE_WASM`)" and the sentence "A missing variable fails the test
naming the command that sets it" as far as it names that variable; D7's clause "writes `"extensions": ["@gsd/core=<GSD_CORE_WASM>"]`"
and its sentence "A user runs `specforge add <path to gsd_core.wasm>`". D1 to D3, the rest of D5 to D7
(the native and `MockHost` tests, `SPECFORGE_BIN`, the clippy and CLI-build steps, the shard of its own, the copy of a
project from `gsd/core/tests/projects/`) and D8 to D10 stand. **Amends**
[ADR 0338](0338-one-foundation-extension-holds-the-generic-value-checkers.md) D8 (a committed blob stays at most
2,500,000 B, except one named per-blob allowance; the cap does not change),
[ADR 0328](0328-a-builtin-is-embedded-and-a-non-specforge-builtin-catalogues-its-codes.md) D3 (one builtin lives outside
`extensions/`: the crate `gsd/core`, artifact `gsd_core.wasm`, named `@gsd/core`) and
[ADR 0360](0360-the-self-spec-moves-to-gsd-core-by-a-one-time-converter.md) D9 (option a is chosen: vendor it as a
committed builtin). Builds on [ADR 0364](0364-the-software-kinds-are-dropped-and-a-census-gives-every-old-kind-its-fate.md)
and [ADR 0365](0365-a-module-is-dropped-and-no-kind-is-added-to-hold-it.md) (they add no kind for the old spec, so
the blob carries `@gsd/core` as its own later changes leave it). Cites [ADR 0214](0214-a-builtins-name-is-refused-where-extensions-load.md) (a builtin's name is
refused where an extension loads from a path, E028; it is kept whole) and
[ADR 0070](0070-every-committed-blob-is-listed-once-and-built-one-way.md) (every committed blob is listed once and built
one way). No protocol change comes with this decision.

## Context

ADR 0360 D9 left the root's way of loading `@gsd/core` open among three options, and gsd ADR 0001 D4 had decided the
blob is built on demand, because "listing `@gsd/core` there would tie the kernel's revendor to a domain extension". A
spike answered whether option a holds: it staged, vendored, embedded and loaded `@gsd/core` by name in a throwaway
worktree at `f61370cb9`, never committed. Its full recipe, with the raw logs and every command, is kept outside the
repository in `../SpecForge-planning/r4b/spike-recipe.md` (`../SpecForge-planning/r4b/BASELINE.md` holds the figures
below as they were measured). The owner chose option a, with the gsd suite reading the vendored blob, on 2026-11-16.

**Sizes** (`stat -c %s` on the file; the staged build is `cargo xtask build-blobs --install`, which builds through
`[profile.blob]`: opt-level z, fat LTO, one codegen unit, strip):

| What | Bytes |
|---|---|
| `gsd_core.wasm`, blob profile, through the stager | 5,663,081 |
| `gsd_core.wasm`, `gsd/` release build, in the spike's worktree (22.9 s, warm dependencies) | 5,663,812 |
| the same build in the scratch copy of the measured baseline | 5,663,652 |
| the global cap of ADR 0338 D8 | 2,500,000 |
| the fourteen builtins git holds at the base (built with rustc 1.98.1) | 12,184,137 |
| the same fourteen rebuilt by the spike's revendor (rustc 1.99.1) | 12,166,212 |
| all fifteen after the spike's revendor | 17,829,293 |
| the sum if the old fourteen are kept as committed | 17,847,218 |

The blob is 2.27 times the global cap and 31.8% of the new total. The two builds of the same source differ from byte
2,981, so the blob profile is the number that counts. `build-blobs --verify` rebuilt every blob, `gsd/core` among them,
and reported each `[same]` (4 min 11 s): the build is deterministic. The committed blobs were built with rustc 1.98.1;
the machine that measured has 1.99.1, and a re-vendor on 1.99.1 rewrites all fourteen old blobs and their
`inputs.json`, not only the new one (`--verify` compares only blobs built by the same rustc).

**Time** (test-profile `specforge` CLI, whose wasmtime and cranelift build at opt-level 3; a fresh
`SPECFORGE_WASMTIME_CACHE` directory per run; the median of five; this machine, nothing else building):

| What | Measured |
|---|---|
| cold compile of `gsd_core.wasm` (a check of a project loading it, minus the same check cached) | 306 ms |
| cached load (a check cached with `@gsd/core`, minus a check with `@specforge/checks` alone) | 52 ms |
| the same cached load for a 1.1 MB builtin, `@specforge/product` | 11 ms |
| the compiled cache entry for `gsd_core.wasm` | 3,836,678 B |
| `cargo xtask revendor` with the row, attempt 3, recompiling xtask first | 351 s |
| `build-blobs --install`, fifteen blobs, cold target | 4 min 52 s |

The constraint `wasm_compile_cache_budget` (`spec/governance/constraints.spec`) reads `cached_extension_load < 50ms`. The
52 ms is an upper bound, since it also counts the handshake and `describe` calls that declare the extension, and it is
2 ms over on this machine; first compilation "under 500ms for a typical component (<1MB)" is for a component five times
smaller. The cost scales about with size: `@gsd/core` is 5.0 times `@specforge/product` and loads in 4.7 times its time.
`../SpecForge-planning/r4b/spike/compile-timings.txt` has the raw values.

**CI, cold** (`gh run view <id> --json jobs`: `completedAt` minus `startedAt` of the job; the gate's own steps from
`gh run view <id> --log --job <job>`, the lines `cargo gate: <check> passed in <n>s`; a run is cold when the log says
`No cache found`):

| Run | Job | Wall time | Gate steps |
|---|---|---|---|
| 43828995410 (pull request, `449db75e9`) | Check list (build) | 1,319 s | `build` 847 s, `self-spec-sample` 432 s |
| 43850086577 (push to main, `f61370cb9`) | Check list (build) | 1,168 s | `build` 775 s, `self-spec-sample` 339 s |
| 43586384133 (pull request, `7ede503f6`) | Check list (gsd), cold | 1,605 s | `gsd` 1,560 s |
| 43828995410 | Check list (gsd), cache restored | 1,044 s | `gsd` 970 s |
| 43850086577 | Check list (gsd), cache restored | 1,349 s | `gsd` 1,278 s |

The build job's `self-spec-sample` step and the `gsd` shard both build the `gsd-core` and `gsd-security` wasm with
`gate/src/gsd.rs`'s `build_wasm` before they read it.

**By name, by path.** With the row, `specforge check` of `gsd/core/tests/projects/roadmap` with `@gsd/core` and
`@specforge/checks` enabled and `GSD_CORE_WASM` unset exits 0 and prints `[]`; the same blob loaded by path by a CLI
without the row prints the same diagnostics, none on the clean project and the same 8 on `roadmap_errors`, by name and
by path alike. With the row, an entry `@gsd/core=<path>` is refused with E028 (`declares '@gsd/core', the name of a
builtin extension: only the builtin loads under it`), and every gsd kind then answers E024, because the entry is
dropped. A CLI without the row refuses `@gsd/core` by name as "enabled but not installed": the row is what makes the
name resolve.

**What the spike changed**, one row per file it touched or found it must touch, and the change each lands with (the
vendoring change is this decision's; the vocabulary change is
[ADR 0369](0369-the-builtin-vocabulary-maps-a-kind-keyword-to-every-builtin-that-declares-it.md)'s). The four rows
before the last are consumers the spike did not meet: a review found them afterwards, and each breaks or turns false
once the row exists. Their line numbers are those of the tree at `f61370cb9`:

| File | Change | Lands with |
|---|---|---|
| `gsd/core/wasm/gsd_core.wasm` | A seed blob must exist before xtask or the CLI compile, since `include_bytes!` reads it: build it with `cd gsd && cargo build --locked -p gsd-core --release --target wasm32-wasip2`, copy it in, and `build-blobs --install` replaces it with the stager's bytes (5,663,812 B seed, 5,663,081 B staged) | the vendoring change |
| `crates/runtime/component/src/builtins.rs` | `builtins!` takes a second row list after a `;`, `"<name>" at "<repo path>" as "<artifact>"`: `crate_dir` and the `include_bytes!` path are the repo path as given, the artifact is the crate's own name (`gsd_core.wasm`, not `specforge_ext_*`), and `@gsd/core` is the last row, so `COMMITTED` lists it after `@javascript/source` | the vendoring change |
| `xtask/src/build_blobs.rs` | `Stage::prepare` for a `gsd/` crate: `cargo metadata` runs in `gsd/`, the stage `Cargo.toml` gets the SDK patch `specforge-extension-sdk = { path = "crates/runtime/extension-sdk" }` (hard-coded in the spike; the real change parses it from `gsd/.cargo/config.toml`), the stage lock, `Stage::build`'s subset check and `collect_inputs` read `gsd/Cargo.lock`, and `gsd/.cargo/config.toml` is recorded as an input (578 inputs); the root manifest's inherited tables already give `gsd-core` and `gsd-kit` the same edition, license and lints | the vendoring change |
| `xtask/src/revendor.rs` | `refresh_lockfile` runs `cargo metadata` in `gsd/` for a `gsd/` crate and watches `gsd/Cargo.lock`; the `git add` line and the `lockfiles refreshed` line name it. Without it revendor stops at the lockfile step: `failed to select a version for the requirement specforge-extension-sdk = "^0.3.0"`, `revendor: cargo metadata failed for gsd/core` | the vendoring change |
| `crates/pipeline/registry/src/compilation/candidates.rs` | `CandidateVocabulary::of` returns Err when two builtins declare one kind keyword; the spike kept the first declarer (a hack), and the real change maps a keyword to every declarer (ADR 0369) | the vocabulary change |
| `crates/pipeline/registry/src/compilation/candidates.rs`, test `two_candidates_declaring_one_keyword_are_refused` | Asserts the Err the hack removed: left failing in the spike | the vocabulary change |
| `gate/src/blob_inputs.rs` | `"gsd/core"` joins `BLOB_CRATES`, so `blobs()` and `fed_by` read `gsd/core/wasm/inputs.json` and a `gsd/` edit feeds the `builtins` check (`build-blobs --check` iterates `COMMITTED` and read the record without it) | the vendoring change |
| `gate/src/blob_inputs/tests.rs` | `blobs_lists_the_recorded_blobs` allows `gsd/core` among the listed prefixes; it panicked at line 128 without it | the vendoring change |
| `gate/src/blob_size.rs` | `ALLOWANCES`, keyed by repo path, and `cap_of(path)`; `verdict` uses it. The check reads git's file list, so an untracked blob passes unseen (`git add -N` showed it); then it failed: `gsd/core/wasm/gsd_core.wasm is 5663081 B, over the 2500000 B cap` | the vendoring change |
| `crates/runtime/component/tests/blob_size.rs` | `TOTAL_BUDGET` 12,200,000 to 17,900,000 and the comment's totals re-measured; it fails at 12,200,000 | the vendoring change |
| `crates/runtime/component/tests/builtins_table.rs` | `the_table_is_the_extensions_directory_by_the_naming_rule` adds `@gsd/core` as the one named row outside `extensions/` before comparing; it failed with `Named { crate_dir: "gsd/core", name: "@gsd/core", artifact: "gsd_core.wasm" }` on the left only | the vendoring change |
| `gsd/core/wasm/inputs.json` | Generated by `build-blobs --install`: the artifact digest, `built_with`, 578 inputs | the vendoring change |
| `crates/runtime/component/tests/declarations/gsd_core/` | Generated by revendor: 16 wire answers, 638,817 B (304 snapshot files written in all) | the vendoring change |
| `crates/runtime/component/data/builtin-vocabulary.json` | Generated by revendor: 60 lines added, every `@gsd/core` kind no old builtin declares; revendor refused the row before the hack: `revendor: @specforge/product and @gsd/core both declare the kind keyword 'persona'` | the vocabulary change |
| `crates/runtime/installed/src/resolve.rs` (`LoadProblem::BuiltinsName`), `crates/runtime/installed/tests/builtin_name.rs` | Not changed, and not to be: once the row exists an entry `@gsd/core=<path>` is E028 (ADR 0214) | none: the host stays whole |
| `gsd/core/tests/cli/harness.rs`, `gsd/core/tests/cli/harness_ab.rs` | Write `@gsd/core=<GSD_CORE_WASM>` into `specforge.json`: refused with E028 once the row exists; they name `@gsd/core` and read the vendored blob | the vendoring change |
| `xtask/src/absorb/write.rs` (`Target::create`), `xtask/src/absorb.rs`, `xtask/tests/absorb_e2e.rs` | `absorb` requires `GSD_CORE_WASM` and scaffolds the target by writing the wasm path as an extension entry, so its `specforge.lock` records a `local:` path: refused with E028 once the row exists; it names `@gsd/core` and reads the vendored blob | the vendoring change |
| `gate/src/gsd.rs`, `gate/src/self_spec_sample/steps.rs` | Build the wasm and set `GSD_CORE_WASM` (`steps.rs` lines 118 and 199: line 118 calls `crate::gsd::build_wasm`, line 199 sets the variable; `gsd.rs` line 53 sets it too); the consumers above then fail. The `gsd-core` build and the variable leave both | the vendoring change |
| `.github/workflows/bootstrap.yml` | The pull-request `paths:` lists `extensions/**` but not `gsd/**`: add `gsd/core/**`, `gsd/kit/**`, `gsd/Cargo.lock` and `gsd/.cargo/**`, so a gsd edit runs `build-blobs --verify` | the vendoring change |
| `docs/agents/revendor.md`, `docs/agents/briefs/_shared.md` | Say builtins live in `extensions/`: name the one exception and add `gsd/Cargo.lock` to the `git add` line | the vendoring change |
| `evaluation/gsd-ab/seed.py`, `evaluation/gsd-ab/OPERATOR.md` | The seed writes `<name>=<path>` into a project's `extensions` for each `@gsd/*` name (`seed.py` line 195): for `@gsd/core` that entry is refused with E028 once the row exists. The operator guide documents building `gsd_core.wasm` and passing it with `--gsd-core` (`OPERATOR.md` lines 23 to 45). A `specforge` binary built before the row (arm Bv1 runs the tag `gsd-ab-v1`) has no builtin of that name and still takes the path: the vendoring change says what the harness does for such a binary. `@gsd/security` keeps its path | the vendoring change |
| `gsd/core/README.md` | Documents `GSD_CORE_WASM` for the tests (line 41), "The component is built on demand and never committed" (line 49) and a by-path `specforge add` of the wasm (line 55), which E028 refuses once the row exists: it describes the vendored blob and the name | the vendoring change |
| `gsd/README.md` | Line 170 describes the packaging as "built on demand": it says the blob is vendored | the vendoring change |
| `gsd/parity/ACCEPTANCE.md` | Line 247 is `specforge publish` of `gsd/target/wasm32-wasip2/release/gsd_core.wasm`: publish refuses a component that declares a builtin's name, as install does, so the line goes or names what a user publishes instead | the vendoring change |
| `gsd/adr/0001-one-cargo-workspace-built-on-demand.md` | Its status paragraph names this ADR and every clause it supersedes; no other line changes | this ADR |

## D1. `BUILTINS` holds `@gsd/core` from `gsd/core`

The row is `@gsd/core` at `gsd/core`, artifact `gsd_core.wasm`, the one builtin outside `extensions/`. The crate does not
move (gsd ADR 0001: the folder moves whole), and the kernel reaches into `gsd/`, never the reverse.

## D2. The blob stager builds `gsd/core` as the gsd workspace does

`cargo metadata` runs in `gsd/`, `gsd/Cargo.lock` is its lock, and the SDK patch of `gsd/.cargo/config.toml` is replayed
into the stage. `inputs.json` records `gsd/Cargo.lock` and `gsd/.cargo/config.toml`. `blob_inputs` and `bootstrap.yml`
track `gsd/core`, `gsd/kit`, `gsd/Cargo.lock` and `gsd/.cargo/**`.

## D3. One named allowance and a raised total, each measured at the vendoring commit

The decision is a rule, not two constants. `gate/src/blob_size.rs` names one allowance, for
`gsd/core/wasm/gsd_core.wasm`: the final blob's size at the vendoring commit, rounded up to the next 100,000 B.
`TOTAL_BUDGET` is the builtins' measured total at that commit, rounded up the same way. Every other blob stays under
the 2,500,000 B cap, and the cap does not change.

At the spike the rule yields 5,700,000 B for the allowance (the blob profile's 5,663,081 B, 36,919 B of headroom) and
17,900,000 B for `TOTAL_BUDGET`, up from 12,200,000 B (the new total of 17,829,293 B, 70,707 B of headroom; 52,782 B if
the old blobs are kept as committed, and the same rounding either way). These figures are what the rule yields at the
spike, not the constants. The vendoring change measures again, because the changes to `@gsd/core`'s kinds and fields
that come before it, each decided in a later ADR, move the blob's size. `@code/core` gains no kind (ADR 0364), so no
other blob grows for these decisions.

## D4. A test proves the committed blob is the source's build

A test builds the gsd workspace's `gsd-core` through the same stage (`build-blobs --verify` does) and requires it to be
byte-equal to the committed blob. It must not compare against the `gsd/` release build, which differs (5,663,812 B
against 5,663,081 B), and it holds only for blobs built by one rustc.

So the test fails, and never skips, in CI when a blob's `built_with` differs from the running rustc.
`build-blobs --verify` prints `[skip]` there today, and `rust-toolchain.toml` names the `stable` channel, so a test
that skipped would pass on every toolchain update without comparing anything. The vendoring change pins the rustc that
builds the blobs and names the CI job that runs the test; this ADR names neither. It is a job's test, not a check that
runs when a project is checked (D5).

## D5. The root's checks and the gsd suite read the vendored blob by name

The gsd suite (`gsd/core/tests/cli/harness.rs`, `harness_ab.rs`), `cargo xtask absorb` (`Target::create`, its scaffold's
`local:` path in `specforge.lock`, `xtask/tests/absorb_e2e.rs`), the gate's `gsd` step and
`gate/src/self_spec_sample/steps.rs` write `@gsd/core` in `specforge.json`, never a path, and read the blob the row
embeds. The host refuses a builtin's name loaded from a path (E028, ADR 0214), and that stays: no exception is made for
a source build. The `gsd-core` wasm build leaves CI, and no check sets `GSD_CORE_WASM`. A project loading `@gsd/core`
checks with nothing to build. What ties the blob to the source is D4's test, not a build at check time.

## D6. `@gsd/core` is vendored once, after its kinds, fields and rules are final

`@gsd/core` is vendored once: after the changes that settle its kinds, its fields and its rules, each decided in a
later ADR, and after the vocabulary change
([ADR 0369](0369-the-builtin-vocabulary-maps-a-kind-keyword-to-every-builtin-that-declares-it.md)), without which
revendor refuses the row: `revendor: @specforge/product and @gsd/core both declare the kind keyword 'persona'`. No
protocol change precedes it
([ADR 0368](0368-a-bound-is-part-of-a-requirements-text-and-the-prove-pass-and-the-protocol-do-not-change.md)): the
protocol stays 2.2.0, and no builtin is re-vendored for a protocol change.

The spike's `@gsd/core` shared seven kind keywords with the old builtins: `decision`, `deliverable`, `journey`,
`milestone`, `persona`, `term` and `invariant`. That was the spike's measurement, not a standing list. The shared
keywords are read from the staged declaration when the vocabulary is generated, never copied from the spike. A later
`@gsd/core` change re-vendors.

## Rejected

- **Raising the global cap.** `@specforge/checks` relies on it (ADR 0338's cap ladder, D16): its blob is 2,451,167 B
  against the cap, and a cap that took 5.7 MB would take every blob.
- **Compressing `gsd/core/assets` as the answer to the size cap.** A decoder dependency and a cost on every prompt
  render, and the blob stays over the cap. It is rejected for the cap only: it remains a remedy for the cached load
  (Consequences).
- **Copying the crate into `extensions/`.** It breaks gsd ADR 0001's whole-folder move and its SDK patch.
- **An exception to ADR 0214 for a source build** (D9 option a with a suite that tests `GSD_CORE_WASM`). The host's rule
  is what keeps a name from loading two different ways; a distinct name for the source build would make the suite test
  a component other than the one that ships.
- **D9 option b**, a path in `specforge.json` and a build before each check: every fresh clone builds the gsd workspace
  before its first check. **Option c**, a registry package pinned by digest: it needs a gsd release pipeline first.

## Consequences

- Every binary that embeds `specforge-component` (the CLI, xtask, the test binaries) grows by the blob's bytes,
  5,663,081 B at the spike, because `include_bytes!` embeds it (arithmetic, no binary was measured; the final blob is
  measured at the vendoring commit, D3). A project that does not load `@gsd/core` pays only that.
- **The cached load breached its constraint by an upper bound of 2 ms at the spike, and the vendoring change settles
  it.** 52 ms against `cached_extension_load < 50ms` (the constraint `wasm_compile_cache_budget` in
  `spec/governance/constraints.spec`, priority critical). The figure includes the declaration calls, as does the 11 ms
  of a 1.1 MB builtin measured the same way. The cold compile, 306 ms, is paid once per cache directory (a cache entry
  is 3,836,678 B), and a clean CI runner pays it once per cache and per test binary with its own cache. The cached
  load is re-measured on the final blob before the row lands. At or under 50 ms, the figure is recorded. Over 50 ms,
  the assets are compressed and the load is measured again, or the owner decides; then `wasm_compile_cache_budget` is
  amended in the same change, or the exception is recorded with an owner and a trigger. The spec is never left
  claiming 50 ms while the blob measures more. The remedies, cut and re-measured one step at a time as ADR 0338's cap
  ladder (D16) does: compressing `gsd/core/assets`, feature-gating its optional text families, or measuring the load
  apart from the declaration calls.
- The vendored blob is a double golden: `kinds.json` by `GSD_BLESS` and the declaration snapshots by revendor, and every
  `@gsd/core` change fails the `builtins` check until re-vendored. The revendor takes 351 s with the row, about 4.5
  minutes of it `build-blobs --install`.
- An edit to `gsd/core` reaches the gsd black-box suite only after a revendor (351 s at the spike) and a CLI rebuild.
  The suite reads the blob the row embeds, not the working tree, so until then it tests the last vendored component.
  No local run loads a fresh build under the builtin's name either (D5, ADR 0214).
- The first revendor on rustc 1.99.1 rewrites all fourteen old blobs (their total falls 17,925 B): that diff is
  larger than the new row.
- The `gsd-core` wasm build leaves the build job's `self-spec-sample` step and the `gsd` shard; the figures above are the
  times they ran with it, and no figure of the removed build is claimed here.
- `@gsd/security` is not a builtin by this decision and keeps its path and `GSD_SECURITY_WASM`.

## Would reopen it

- The blob passing about 6.5 MB, or the allowance of D3 needing a second blob.
- The cold compile or the cached load outgrowing what Consequences records, or the constraint's measure changing.
- `gsd/` leaving the repository (D9 option c).

## Review findings

A review of ADRs 0364 to 0369 on 2026-11-16 raised 25 findings: CR-01, WR-01 to WR-19 and IN-01 to IN-05. Each has one
row, in the ADR it concerns most; these are this ADR's. Phases are those of milestone v4.1: 2 the vocabulary change, 3
the review of `@gsd/core`'s kinds and fields, 5 the conversion, 6 the vendoring, 7 the swap.

| Finding | What it said | Outcome | Delivered in |
|---|---|---|---|
| WR-11 | The allowance and the total budget were fixed from the spike's measurement, while the blob is vendored after changes that move its size | Fixed in D3: the allowance and the total are a rule measured at the vendoring commit, and the spike's figures are what the rule yields at the spike | phase 6 |
| WR-12 | The byte-equality test was not specified for a different rustc, and the toolchain floats | Fixed in D4: the test fails, and never skips, in CI on a rustc mismatch, and the vendoring change pins the rustc and names the job | phase 6 |
| WR-13 | The file table left out files that break once the row exists | Fixed in Context, the table "What the spike changed": four rows, for `evaluation/gsd-ab/seed.py` with `OPERATOR.md`, `gsd/core/README.md`, `gsd/README.md` and `gsd/parity/ACCEPTANCE.md`, and what a binary built before the row does | phase 6 |
| WR-14 | The cited `steps.rs` lines were not where the wasm build is | Fixed in the same table: `steps.rs` lines 118 and 199 | this ADR |
| WR-15 | The decision shipped a breach of a critical constraint and left the spec false | Fixed in Consequences: the cached load is re-measured on the final blob, and if it is still over 50 ms the constraint is amended in the same change or the exception is recorded with an owner and a trigger | phase 6 |
| WR-19 | gsd ADR 0001's status paragraph did not cover everything this ADR supersedes | Fixed in gsd ADR 0001's status paragraph, in the same change as this row: it names D7's `specforge add` sentence and the Consequences' release wasm clause | this ADR |
| IN-01 | The ADRs leaned on an unnamed earlier study and on unlinked decisions named by a letter | Fixed. ADRs 0364, 0365, 0366 and 0368 name a decision by its ADR number with a link, state an owner's decision of 2026-11-16 in words and never by a label, and state each figure with how it was measured, with no comparison against an unnamed earlier study | this ADR |
| IN-05 | With the vendored blob the gsd black-box suite reads the last vendored component, not the working tree | Fixed in Consequences: an edit to `gsd/core` reaches the suite only after a revendor and a CLI rebuild, and no local run loads a fresh build under the builtin's name | phase 6 |
