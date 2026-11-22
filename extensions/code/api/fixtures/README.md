# API snapshot reader fixtures

Neither `cargo public-api` nor api-extractor is installed where these were written, so each fixture is hand-written
to the grammar or model named below. Phase 11's packs re-record them with the real tools.

| Fixture | Source |
|---------|--------|
| `rust.public-api.txt` | hand-written to the cargo-public-api snapshot grammar (the lines of the tool's own committed snapshots `comprehensive_api.txt` and `example_api-v0.3.0.txt`): crate `example_api` with a module, a `#[non_exhaustive]` struct and its field, an inherent `impl` with `new`, `impl Debug` and `impl Display` each with a `fmt`, a marker `impl` with no member, a `pub extern crate`, a `pub use`, an enum with a tuple and a unit variant, an enum with a discriminant variant, a `pub const`, a `#[repr(C)]` struct and its field, a `const fn`, a function no source item holds (`from_macro`) and a `pub(crate)` function |
| `widget.api.json` | hand-written to the api-extractor .api.json doc model (`@microsoft/api-extractor-model`'s `ApiPackage` serialization, schema version 1011): Package `@demo/widget` > EntryPoint > Class `Widget` with Method `render` and Property `size`, Interface `Options`, Function `create`, Function `parse` overloaded twice, Enum `Mode` with EnumMember `Fast`, TypeAlias `Id`, Variable `VERSION`, Namespace `util` |
| `items.json` | hand-written served `item` facts (ids only). Rust, of the package `example-api`: most entries of `rust.public-api.txt`, not the re-export, the extern crate, the variants nor `from_macro`, and one item of a binary target of that package, which no entry may cite. JavaScript, of `@demo/widget`: the entries of `widget.api.json` in their source files, `VERSION` in two files (so neither is cited) and no `util` (only another package's) |
