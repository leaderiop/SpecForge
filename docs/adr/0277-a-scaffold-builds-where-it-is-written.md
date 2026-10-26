# A scaffold builds where it is written, and an SDK it cannot resolve is named

**Status:** accepted (2026-10-26, architecture round 29, plan 01). Extends
[ADR 0238](0238-the-scaffold-names-the-sdk-this-build-speaks.md) (the scaffold names the SDK at the workspace version,
a release publishes it).

## Context

`specforge extension init` names `specforge-extension-sdk` at the workspace version, 0.2.0 (ADR 0238 D1). crates.io
holds only 0.1.0, which is the pre-cutover Extism SDK of protocol 1.0.0, and no release has been cut since 0.1.0-rc1,
so a scaffold made by any build since the bump does not resolve. `extension build` relayed cargo's line as E040 with no
remedy, though ADR 0238 promised a clear message. Cargo states the cause two ways: `error: failed to select a version for
the requirement` (with an index entry) and `error: no matching package named` (without one). And a scaffold inside any
cargo workspace (this repository's `examples/`, or an author's own) failed before resolution: "current package believes
it's in a workspace when it's not", because the scaffold's `Cargo.toml` had no `[workspace]` table, though its
`[profile.release]` and `.cargo/config.toml` already assume it is a root.

## D1. The scaffold is a workspace of its own

The scaffold's `Cargo.toml` has an empty `[workspace]` table. It builds unedited inside another cargo workspace; an author
who wants it as a member deletes the line (and its profile, which cargo ignores in a member).

## D2. An SDK that does not resolve is named, with its remedy

When cargo's first error is one of the two resolution lines for `specforge-extension-sdk`, `extension build` refuses with
E040 (`PreconditionFailed`), `specforge-extension-sdk does not resolve in <dir>: <cargo's line>`, and a suggestion naming
the `[patch.crates-io]` recipe in `docs/extension-sdk.md`, "Building against an unreleased SDK". Every other build
failure keeps the generic E040.

## D3. `init --dir` is the parent folder

`extension init --dir` names the folder the crate's folder (the last segment of `--name`) is created in, as today; the
help says so.

## D4. Publishing stays by hand

ADR 0238 D2 is unchanged: the user publishes the three guest crates before tagging, and the tag-time `scaffold` job
refuses a release whose scaffold does not build against crates.io.

## Rejected

- `init --sdk <version|path>`: a flag for what `[patch.crates-io]` already does without editing the scaffold.
- The newest published SDK: crates.io's 0.1.0 is the Extism SDK, which the host cannot load.
- A path baked in at build time: a build machine's path in a binary.
- `init --dir` as the crate's own folder: breaks scripts, tests and the release job for one meaning per flag name.
- A check on `main` that crates.io has the scaffold's version: red from every bump to its publish by construction.

## Consequences

No version changes and no re-vendor. The user publishes 0.2.0 and tags `v0.2.0` without a suffix, which also gives the
README's installer a latest release. After that, a build from `main` resolves the published 0.2.0 until the next bump.

## What would reopen it

A second published guest crate, or a release cadence that leaves `main` ahead of crates.io for long (then `init --sdk`).
