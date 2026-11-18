# `@gsd/core` is vendored on a pinned rustc, with its load and size measured

**Status:** accepted (2026-11-17). **Amends**
[ADR 0135](0135-a-remedy-names-a-command-or-the-requests-words-never-a-flag.md) D5 (an argument hint is the command's
own syntax: D6) and
[ADR 0369](0369-the-builtin-vocabulary-maps-a-kind-keyword-to-every-builtin-that-declares-it.md) D2 (the order and the
filter of a kind suggestion: D4). **Carries out**
[ADR 0366](0366-gsd-core-is-vendored-as-a-builtin-with-a-size-allowance-of-its-own.md) D3 (the constants, measured at
the vendoring commit: D3), D4 (the pinned rustc and the job that runs the equality test: D1, D2), D5 (the readers that
take the blob by name) and its Consequences' rule for the cached load (D3). Builds on
[ADR 0214](0214-a-builtins-name-is-refused-where-extensions-load.md) (E028 stays whole: a path entry for `@gsd/core` is
refused), [ADR 0122](0122-the-builtins-vocabulary-is-generated-beside-them.md) (the generated vocabulary the suggestions
read) and [ADR 0292](0292-an-extension-contributes-agents-skills-commands-and-hooks.md) (what a builtin contributes to a
host: D7). No protocol, SDK or host-rule change comes with this decision.

## Context

**What ADR 0366 left to the vendoring change.** It decided that `@gsd/core` is a committed builtin with a size
allowance of its own, and left four things to the change that vendors it: the two constants, measured on the final
blob (its D3); the rustc that builds the blobs and the CI job that runs the equality test, which it named neither of
(its D4); the readers that take the blob by name (its D5); and the answer for the cached load, which the spike put 2 ms
over the 50 ms bound (its Consequences). This ADR is that change's record.

**The measured inputs.** These are the research's figures, measured on the tree at `eaa5d1d12` with rustc 1.99.1,
before the decisions. The figures measured at the vendoring commit are under [As built](#as-built); where the two
differ, the as-built figure is the record.

- **The blob.** The stager (`build-blobs --install`, run on a scratch copy of the tree with the `@gsd/core` row) builds
  `gsd_core.wasm` at 5,470,682 B, sha256 starting `5c958ac0`, with 567 recorded inputs; a build staged by hand with the
  same flags gave the same bytes. The spike's blob was 5,663,081 B: the kinds and fields the later changes removed took
  192,399 B with them. The `gsd/` workspace's own release build of the same source is 5,471,191 B, about 500 B more,
  because the stage builds through the root's `[profile.blob]` and not `gsd/`'s release profile: the two builds are
  never compared.
- **The total.** The fourteen old builtins, rebuilt by the stager on rustc 1.99.1, total 12,166,212 B (17,925 B less
  than the 12,184,137 B committed from rustc 1.98.1, the spike's figure to the byte: the build is deterministic). The
  fifteen total 17,636,894 B. The vocabulary the revendor writes holds 85 kind keywords, three of them declared by
  `@gsd/core` and an old builtin both: `decision`, `deliverable` and `milestone`.
- **The cached load, three ways.** (a) In process: a `ComponentRuntime` with a compile cache that already holds the
  blob loads it in 19 to 27 ms, then the handshake takes 1.7 to 2.4 ms and the nine `describe` calls that declare the
  extension 8.4 to 12.6 ms, 30 to 42 ms in all. (b) The spike's method, a CLI check of a project loading the blob minus
  the same check with `@specforge/checks` alone, both cached, median of fifteen or more runs: 41 to 52 ms across
  batches. The CLI's wall time moves in 10 ms steps, because the runtime joins its epoch-ticker thread, which sleeps
  10 ms between ticks, when it is dropped (measured in process at 8.5 to 10.5 ms), so a difference of two such times is
  good to about 5 ms either way and flips across 50 ms with the tick's phase. (c) The same differential in CPU time,
  which the tick does not quantise but which counts every thread's work: 50.3 ms. The CLI figures also pay the host's
  registry build over the 581,821 B declaration and the rule run, which are not the extension's load.
- **Compression.** The assets are about 2.53 MB of the blob (an emptied-asset blob is 2,942,520 B). Removing every
  asset cuts the in-process load by 3 to 5 ms and the CLI differential by 4 to 9 ms; compressed whole, the assets would
  leave a blob of 3.5 to 3.9 MB, still over the 2,500,000 B cap.

**What the row breaks, found by running it.** A revendor with the row on a scratch copy of the tree, then the whole
workspace's tests: 9,623 run, ten failed, each a consequence of the row. One of them is the bare-flag guard of
ADR 0135: it read `@gsd/core`'s declared surfaces for the first time and found 39 literals naming a bare flag. 25 are
upstream gsd-core's argument hints, the usage lines of its slash commands (`<phase> [--auto] [--text]`), carried
verbatim; 14 are SpecForge-authored texts in `gsd/core`: nine command-argument descriptions, two command descriptions
and three prompt step titles. The other failures are the old set's declaration tests (D7), the import table, the
registry snapshot and the VS Code pickers' test.

**A second constraint the blob contradicts.** Besides `wasm_compile_cache_budget` (50 ms on a compile-cache hit),
`wasm_binary_size_limit` (`spec/governance/constraints.spec`, priority high) says an extension's wasm binary should be
under 5MB. The blob is over it.

## D1. The committed blobs build on a pinned rustc, 1.99.1

One constant, `gate::blob_inputs::BLOB_RUSTC`, names the rustc every committed blob is built with: 1.99.1. The stager
and `revendor` apply it to every cargo and rustc they spawn for a blob, through `RUSTUP_TOOLCHAIN`, with any `RUSTC`
override removed, and `built_with` records that rustc's version. `build-blobs --verify` fails, never skips, on a blob
another rustc built, and the failure names the pin and the fix. The bootstrap job installs the pinned toolchain
explicitly (`rustup toolchain install`), and so does a developer who re-vendors. Changing the pin re-vendors every blob
in the same commit.

The reason: `rust-toolchain.toml` names the `stable` channel, every committed blob says it was built with 1.98.1, and
`--verify` skips a blob built by another rustc, so with a toolchain that floats it skips every blob on CI today and
compares nothing.

## D2. A test proves the committed `gsd_core.wasm` is the stager's build

`committed_gsd_core_blob_is_the_stagers_build` runs `build-blobs --verify --only gsd/core` and needs a `[same]` line
for `gsd/core`, so it cannot pass by skipping. The bootstrap job (`.github/workflows/bootstrap.yml`, job `bootstrap`)
runs it beside `cargo gate bootstrap`, which verifies every blob the same way. `blob_inputs` lists `gsd/core`, and the
job's `paths:` list `gsd/core/**`, `gsd/kit/**`, `gsd/Cargo.lock` and `gsd/.cargo/**`; a test holds that the paths
cover every input any `inputs.json` records, so an edit to a blob's input always runs the job.

The reason: ADR 0366 D4 asks for a test that compares the committed blob with the source's build through the same
stage. It is a job's test, since it builds a blob, not a check that runs when a project is checked.

## D3. The constants and the two constraints

ADR 0366 D3's rule is applied at the vendoring commit: the allowance of `gsd/core/wasm/gsd_core.wasm` and
`TOTAL_BUDGET` are the measured size and the measured total of the builtins, each rounded up to the next 100,000 B
(5,500,000 B and 17,700,000 B on the research's figures). The 2,500,000 B cap is unchanged.

`wasm_compile_cache_budget`'s `cached_extension_load` is read on the in-process load of a compile-cache hit plus the
handshake and the `describe` calls that declare the extension: what the host pays to have the extension. The
constraint's metric names that seam and records the measured figure, with the CLI differential beside it, which adds
the host's registry build and the runtime's 10 ms epoch-ticker join. The assets are not compressed: it saves 3 to 9 ms
and the blob stays over the cap. If the in-process figure is over 50 ms, the metric records the exception with its
owner and its trigger (the blob passing 6.5 MB, or a cached load over 60 ms).

`wasm_binary_size_limit` names the one vendored exception, held to its allowance by the `blob-size` check. Each
constraint changes its `description` and `metric` text only; its `verify` lines stay as written.

The reason: the constraint's own words, "load ... on a compile-cache hit", name the extension's load, and the host's
registry build over the declaration is the host's work, not the extension's. A reading that depends on the phase of a
10 ms tick is no reading. The spec is never left claiming what the blob contradicts (ADR 0366 Consequences).

## D4. A kind suggestion offers only the declarers that collide with nothing loaded

A declarer of a kind keyword is offered when no kind keyword it declares is already registered by a loaded extension,
since installing it would be E026. When every declarer collides, the full list is offered, and the sentence says, in
one clause, that each also declares a loaded kind, so the suggestion never goes silent. E024, W151, I004, E022 and I210
read one rule, `CandidateVocabulary`'s. This amends ADR 0369 D2's "in the vocabulary's order": the order stays the
vocabulary's, and the filter comes first.

The reason: a review of the vocabulary change found that a project loading `@specforge/product` and writing a
`decision` was told `specforge add @gsd/core` first, and following it gives E026 for `deliverable` and `milestone`,
which both declare. The filter leaves `@specforge/governance`, which collides with nothing loaded.

## D5. `@gsd/security` is not vendored

The gate's `gsd` step builds `@gsd/security`'s wasm from `gsd/`, and the suite reads it through `GSD_SECURITY_WASM`.
Nothing builds `@gsd/core`'s wasm to load it: every reader takes the builtin by name.

The reason: `@gsd/security` is tested beside `@gsd/core`, not loaded by the root's checks, and vendoring it is a size
and a review of its own for a later change.

## D6. An argument hint is the command's own syntax

The bare-flag guard skips the `argument_hint` key of a declared slash command, so upstream's usage strings stay
gsd-core's text, byte for byte. Every other declared text, `@gsd/core`'s SpecForge-authored ones included, names the
request's words, never a bare flag. This amends ADR 0135 D5, which read every string a builtin declares.

The reason: ADR 0135 D1 holds that "a flag inside a named command is fine", since it is typed whatever the surface. A
slash command's argument hint is that command's own usage line: it is shown beside the command's name and spells the
command's syntax, not a remedy for a request.

## D7. The old set's declaration rules hold for the old set

No builtin kind is scoped, declares settings, a K3 attribute or a writer or history flag, and none contributes to a
host (the case ADR 0292's Consequences name: "a project whose extensions contribute nothing to a host declares and runs
as it did"). These hold for the fourteen `@specforge/*` starter-set builtins (`Family::Old`). `@gsd/core` scopes its
kinds ([ADR 0370](0370-gsd-core-holds-exactly-gsd-cores-kinds-and-fields.md)), declares settings and contributes agents,
skills, slash commands and hooks, by design. The VS Code pickers list `@gsd/core` as they list every builtin that
declares authored kinds.

The reason: the tests that hold these rules were written for the old set and stay true of it; `@gsd/core` was never
meant to meet them.

## As built

Measured at the vendoring commit `6cd048a37` (2026-11-17), whose one revendor built the blob on rustc 1.99.1
(58cf8f9c3 2026-10-13); the constants and both constraints' texts are computed from its committed bytes.

- **The blob.** `gsd/core/wasm/gsd_core.wasm` is 5,470,859 B, sha256 starting `da1b8291`, with 567 recorded inputs,
  `gsd/Cargo.lock` and `gsd/.cargo/config.toml` among them. It is 177 B more than the research's 5,470,682 B: the
  rewordings of D6 and of six field docs landed between the two.
- **The allowance.** 5,500,000 B, the size rounded up to the next 100,000 B: 29,141 B of headroom. The 2,500,000 B cap
  is unchanged.
- **The total.** The fifteen builtins total 17,637,071 B, and `TOTAL_BUDGET` is 17,700,000 B: 62,929 B of headroom.
  The old fourteen, rebuilt on the pin, total 12,166,212 B, the research's figure to the byte (17,925 B less than on
  rustc 1.98.1); `@specforge/checks` is 2,448,525 B of it.
- **The cold compile.** In process: 359, 314 and 334 ms in three runs. The CLI differential's cold compile: 285 ms wall.
- **The cached load in process** (load of a compile-cache hit, the handshake and the `describe` calls), median of eight
  runs, three times: 33.15, 31.99 and 32.07 ms, of which the load is about 16 to 17 ms and the declaration about
  15.6 to 15.8 ms. The harness is the ignored test `gsd_core_loads_from_the_compile_cache_in_under_fifty_ms`.
- **The CLI differential**, n=20, a check loading the blob minus the same check without it, both cached: 51.2 ms
  wall and 46.7 ms CPU. The wall time of the check loading it is bimodal at 65 and 75 ms, in the epoch ticker's 10 ms
  steps.
- **The revendor** that vendored it took 5 min 17 s, against the research's estimate of about six minutes; the
  Consequences give the measured figure.
- **The equality test**, `committed_gsd_core_blob_is_the_stagers_build`, runs in about 22 s on a warm target
  (`[same]   gsd/core`).
- **The vocabulary** holds 85 kind keywords, 61 of them `@gsd/core`'s; exactly three are shared with an old builtin:
  `decision`, `deliverable` and `milestone`.

**D3's branch.** The in-process cached load, about 32 ms, is under 50 ms. So `wasm_compile_cache_budget`'s metric
records the figure with its date and the CLI differential beside it, no exception is written, and it names no owner
or trigger; the assets are not compressed. The CLI differential's 51 ms wall is the host's registry build and the
ticker's join on top of the load (D3), not the bound's reading.

## Rejected

- **Compressing the assets.** 3 to 9 ms saved, a decoder dependency and a changed `assets::get`, and the blob stays
  over the cap.
- **Pinning 1.98.1.** No churn in the old blobs, but the constants were not measured on it, and it is a second
  toolchain everywhere.
- **Pinning the whole repository in `rust-toolchain.toml`.** It pins every developer's and CI's compiler for
  everything, not the blobs.
- **Exempting `@gsd/core` from the guard by name.** One line, and 14 texts MCP callers read would keep flag spellings
  no MCP argument has.
- **A wall-clock gate check for the cached load.** The 10 ms tick and shared runners make it flake.
- **Comparing the committed blob with the `gsd/` release build.** The bytes differ.
- **Vendoring `@gsd/security` now.** D5.
- **Skipping `@gsd/core` in the VS Code pickers by name.** The pickers list every builtin that declares authored kinds.

## Consequences

- Every binary embedding `specforge-component` (the CLI, xtask, the test binaries) grows by the blob's bytes.
- A consent stored for a path-loaded `@gsd/core` was keyed by its digest and does not carry over to the builtin, which
  is keyed by name: a release-note line.
- Re-vendoring needs the pinned toolchain installed.
- `gsd/Cargo.lock` is hashed whole, so a dependency change that touches only `gsd-security` re-vendors `@gsd/core`.
- An edit to `gsd/core` reaches the black-box suite only after a revendor (about five minutes: 5 min 17 s
  measured) and a CLI rebuild.
- The 19 old blobs are rewritten once, on 1.99.1; bumping the pin re-vendors all twenty.

## Would reopen it

- The blob passing about 6.5 MB, or a second blob needing an allowance.
- The in-process cached load over 50 ms.
- A rustup that stops honouring `RUSTUP_TOOLCHAIN`.
- `gsd/` leaving the repository.
