# The scaffold names the SDK this build speaks

**Status:** accepted (2026-10-23, architecture round 26, plan 09). Extends
[ADR 0091](0091-the-crate-map-is-generated-from-the-workspace.md) D1 (a crate's manifest is the one source of what
crates.io shows) to its version.

## Context

`specforge extension init` wrote `specforge-extension-sdk = "0.1"`, which resolves on crates.io to the only published
SDK, 0.1.0: the Extism SDK of protocol 1.0.0, published before the component-model cutover. The workspace said
`0.1.0` too, so one number named two SDKs, and 89 SDK and guest-wire commits later a fresh scaffold failed `cargo
check` with four errors. The only test that built a scaffold edited its `Cargo.toml` first, and the SDK docs showed
dependency lines that do not resolve (`"1.0.0"`, `specforge-wasm`, `specforge-test`).

```text
BEFORE  scaffold "0.1" ──▶ crates.io 0.1.0 (extism)      test: edits the scaffold, then builds
AFTER   scaffold "{CARGO_PKG_VERSION}" ──▶ this SDK      test: builds the scaffold's bytes as written
```

## D1. The SDK version is the workspace version

`[workspace.package] version` is the SDK's, its macros', the guest wire's and the CLI's. `extension init` names it
(`env!("CARGO_PKG_VERSION")`), so no literal is left to drift. The workspace is 0.2.0, a version never published.

## D2. A release publishes the guest crates before its tag

A release publishes `specforge-protocol-types`, `specforge-extension-sdk-macros` and `specforge-extension-sdk` at that
version from the commit it tags. The release workflow makes no GitHub Release unless a fresh scaffold builds and
validates against crates.io.

## D3. In the repository the scaffold's files are built unedited

The SDK the build compiles comes from a cargo configuration beside the crate, offline. A version mismatch fails to
resolve, so the test also pins the version.

## D4. The docs show only published crates

An author is shown the published guest crates at that version, or path dependencies inside the repository. A test holds
the three author-facing files to it.

## D5. `extension validate` reports the protocol

It states the protocol version the component's handshake speaks (`protocol`), which is not the SDK version.

## Rejected

- A git dependency at the build's commit: a scaffold would depend on an unreleased commit.
- Path-or-refuse: no scaffold for an author outside the repository.
- `0.1.1`: it would still match `"0.1"` of the old SDK.
- A `[patch]` written into the scaffold's `Cargo.toml`: the author would ship it.
- Publishing from CI: a credential in the pipeline for three crates.

## Consequences

One user step per release (`cargo publish`), and a re-vendor at each version bump. A scaffold made before the publish
lands fails to resolve with a clear message.
