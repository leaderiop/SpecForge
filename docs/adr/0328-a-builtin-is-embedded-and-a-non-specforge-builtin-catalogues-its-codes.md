# A builtin is embedded, a non-@specforge builtin catalogues its codes, and builtins live in family dirs

**Status:** accepted (2026-11-02, build-plan R1, plan 02). Amends [ADR 0286](0286-a-diagnostic-code-is-namespaced-by-its-extension.md)
D2 and D3 ("first party" is the `@specforge/` scope, not "a builtin") and [ADR 0070](0070-every-committed-blob-is-listed-once-and-built-one-way.md)
D2 (the layout: a builtin's crate may sit one level deeper, and a blob is keyed by its full name or crate dir).

## Context

Every builtin was `@specforge/<dir>`, built from `extensions/<dir>`, so three things that are different read as one:
a builtin (a component the binary embeds), a first-party extension (whose codes are bare, in the static catalogue)
and a crate one level under `extensions/`. R1 adds `@code/core` (the neutral home of test facts) and, after it,
`@javascript/node-test`; R2 adds `@rust/*` and `@javascript/*` packs. Each is a builtin, none is `@specforge/`.

The tools then broke on three assumptions. The builtins table test required `@specforge/{dir}`, and the codes test
required every builtin to declare no `codes` (ADR 0286 D2 read as "builtin"). The gate's globs, rustfmt's crate list
and the blob inputs read `extensions/*` one level deep. And blobs were keyed by the last segment of their name (the
declaration and registry-build snapshots) or of their crate dir (the build stage), so `@rust/source` and
`@javascript/source` would share a key.

## Decisions

- **D1. A builtin is not first party.** A builtin is a component the binary embeds: a row of `BUILTINS`, loaded by
  name, whose name no `.wasm` file entry may take (ADR 0214), whatever its scope. First party is the `@specforge/`
  scope only (`FIRST_PARTY` in `specforge-diagnostics`): its codes are bare and live in the static catalogue (ADR
  0286 D2).
- **D2. A builtin outside `@specforge` that reports catalogues its codes.** It declares the `codes` category as a
  third party does (ADR 0286 D3), and its findings print namespaced: `@code/core`'s E101 prints `code/E101`. Its rules
  are checked against the static catalogue plus its own. A `@specforge/` builtin still declares none. It reports
  through validation rules or passes (`specforge_registry::reports`); one that only collects (`@javascript/node-test`)
  has no code to catalogue.
- **D3. One naming rule, two shapes.**

  | Crate dir | Name | Artifact |
  |---|---|---|
  | `extensions/<dir>` | `@specforge/<dir>` | `specforge_ext_<dir>.wasm` |
  | `extensions/<family>/<pack>` | `@<family>/<pack>` | `specforge_ext_<family>_<pack>.wasm` |

  In an artifact name, `-` becomes `_`. A crate with a `wasm/` dir is a builtin. A family crate without one is an
  **extension library**: `crate-type = ["rlib"]`, package `specforge-<family>-<dir>`, never embedded, linked by the
  builtins that need it (the precedent is `gsd/kit`). A library needs a family: a depth-1 crate (`extensions/<dir>`)
  without `wasm/` is an error. A directory under `extensions/` is a crate or a family, never
  both, and the family `specforge` is refused. `extensions/rust/` holds a `Cargo.toml`, so it is the depth-1 crate
  `@specforge/rust`; while it exists, no `extensions/rust/<pack>` may be added. R2 retires `@specforge/rust` by deleting
  the crate, and the rule then reads `extensions/rust/` as the `@rust` family. Nothing moves in R1.
- **D4. A blob is keyed by its full name or crate dir.** The declaration and registry-build snapshots use
  `CommittedBlob::key()`: the name without `@`, with `/` turned into `_` (`specforge_product`, `code_core`,
  `sdk_greet`). The build stage is `target/blob-stage/<crate_dir>`. Both are unique by construction, and a test
  asserts it. The gate's patterns take any number of whole-segment `*`, and every list of guest crates
  (rustfmt, the guests' tests and clippy, the blob inputs) reads `extensions/*` and `extensions/*/*`.
- **D5. The blob budget.** R1 raises the total budget of the embedded blobs by 1 MB, to 5,900,000 B, for its two new
  builtins; the 1 MB per-blob cap stays. Ports ADR 0338 later replaces both with a 2.5 MB per-blob gate cap and
  re-measures the total.

## Rejected

- Moving the nine `@specforge/*` crates to `extensions/specforge/<pack>`: churn for crates R2 retires, and a second
  spelling of one name.
- Keeping builtins bare whatever their scope: `@code/core`'s E101 would collide with core's E101 in the static
  catalogue, and ADR 0286 D1 forbids a builtin pack from reporting a kernel code.
- Namespacing the `@specforge/*` builtins: ADR 0286 rejected it, and nothing here changes that.
- Keeping last-segment keys with a uniqueness test: the first R2 pack pair (`@rust/source`, `@javascript/source`)
  breaks it.
- A nested snapshot layout (`declarations/code/core/`): the revendor sweep and every reader of the snapshots walk one
  level; a flat key keeps their shape.

## Consequences

- `Builtin.dir` is gone; a builtin's crate is `Builtin.crate_dir`.
- The declaration snapshot dirs and the registry-build `alone_*` snapshots are renamed to their keys.
- `build-blobs` stages under `target/blob-stage/extensions/<dir>`; a stage's bytes are unchanged (the stage is
  remapped to `/specforge`).
- A builtin may link an extension library; its inputs then include the library's sources.
- `@code/core` and later family packs show namespaced codes in every surface, and `explain code/E101` reads their
  catalogue.

## What would reopen it

- A family pack that must report a bare code: it would have to be first party, so it moves under `@specforge/`.
