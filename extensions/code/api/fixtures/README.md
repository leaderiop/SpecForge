# API snapshot reader fixtures

`cargo public-api` is not installed where these were written (it needs a nightly toolchain), so its fixture is
hand-written to the tool's grammar. The api-extractor fixtures are hand-written to its model, beside doc models
recorded with the real tool.

| Fixture | Source |
|---------|--------|
| `rust.public-api.txt` | hand-written to the cargo-public-api snapshot grammar (the lines of the tool's own committed snapshots `comprehensive_api.txt` and `example_api-v0.3.0.txt`): crate `example_api` with a module, a `#[non_exhaustive]` struct and its field, an inherent `impl` with `new`, `impl Debug` and `impl Display` each with a `fmt`, a marker `impl` with no member, a `pub extern crate`, a `pub use`, an enum with a tuple and a unit variant, an enum with a discriminant variant, a `pub const`, a `#[repr(C)]` struct and its field, a `const fn`, a function no source item holds (`from_macro`) and a `pub(crate)` function |
| `widget.api.json` | hand-written to the api-extractor .api.json doc model (`@microsoft/api-extractor-model`'s `ApiPackage` serialization, schema version 1011): Package `@demo/widget` > EntryPoint > Class `Widget` with Method `render` and Property `size`, Interface `Options`, Function `create`, Function `parse` overloaded twice, Enum `Mode` with EnumMember `Fast`, TypeAlias `Id`, Variable `VERSION`, Namespace `util` |
| `api-extractor-7.60.1.widget.api.json` | recorded by the Phase 11 research with api-extractor 7.60.1 (`api-extractor run --local`, doc model only, the `@javascript/api` config shape) from the `.d.ts` below, package `@demo/widget`; committed unchanged. It names no absolute path. Members are alphabetical, as the tool writes them |
| `api-extractor-7.60.1.widget.d.ts` | the input that model was recorded from: the package's built `index.d.ts`, unchanged |
| `api-extractor-7.60.1.rich.api.json` | recorded by the Phase 12 research with api-extractor 7.60.1 (`api-extractor run --local`, doc model and report enabled) from the two `.d.ts` below, package `@demo/rich`; committed unchanged, no absolute path. It holds a protected constructor and an index signature (the two members the tool writes with no `name`), an accessor pair (one `Property`), a namespace with a variable, an overloaded function, a multi-line type alias and an `@alpha` const enum |
| `api-extractor-7.60.1.rich.d.ts` | the input `index.d.ts` of `@demo/rich`, unchanged; it re-exports `Other` from the file below |
| `api-extractor-7.60.1.rich-other.d.ts` | the input `other.d.ts` of `@demo/rich`, unchanged |
| `items.json` | hand-written served `item` facts (ids only). Rust, of the package `example-api`: most entries of `rust.public-api.txt`, not the re-export, the extern crate, the variants nor `from_macro`, and one item of a binary target of that package, which no entry may cite. JavaScript, of `@demo/widget`: the entries of `widget.api.json` in their source files, `VERSION` in two files (so neither is cited) and no `util` (only another package's) |
