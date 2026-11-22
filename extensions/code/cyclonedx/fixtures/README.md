# CycloneDX reader fixtures

Neither cargo-cyclonedx nor cyclonedx-npm is installed where these were written, so each fixture is hand-written to
the CycloneDX 1.6 JSON reference and the tool shape named below. Which `specVersion` cargo-cyclonedx writes by default
is unverified: Phase 11's packs ask the tool for one the reader accepts (1.4, 1.5 or 1.6) and re-record these with the
real tools.

| Fixture | Source |
|---------|--------|
| `cargo.cdx.json` | hand-written to the CycloneDX 1.6 JSON reference and the cargo-cyclonedx shape: specVersion `1.5`, root `pkg:cargo/demo@0.1.0?download_url=file://.` with a `bom-ref` and a nested target component (the root's own nesting is not read as components), the components `serde`, `serde_derive` and `itoa` with purls and registry bom-refs, and a `dependencies` graph where the root depends on `serde` only |
| `npm.cdx.json` | hand-written to the cyclonedx-npm shape: specVersion `1.6`, root `pkg:npm/%40demo/board@1.0.0` (group `@demo`), a scoped `@babel/runtime`, `fsevents` of scope `optional`, `loose-envify` and `js-tokens` nested inside `react`, and `js-tokens` listed a second time at the top with scope `optional`; the graph has the root depend on `@babel/runtime`, `fsevents` and `react` |
| `no-purl.cdx.json` | hand-written to CycloneDX 1.4, no dependency graph: root `pkg:cargo/demo@0.1.0`, `anyhow` with a purl, and `madler/zlib` 1.3.1 of scope `excluded` with a name, group and version but no purl |
| `packages.json` | hand-written served facts: the `package` facts an SBOM collector receives, `pkg:cargo/demo` at the project root and `pkg:npm/%40demo/board` at `web` |
