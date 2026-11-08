# Item and package facts, and ids a SCIP symbol maps into

**Status:** accepted (2026-11-08, R2 of the Beyond Parity build plan). **Amends**
[ADR 0329](0329-code-core-holds-test-facts-and-their-ids.md) D2 (the `<file>` escape; Rust tests), D3 (`test` takes
`rust` ids and `binary`; `file` is project-relative as the pack gives it; `names` a reference list), D5 (a reference is
judged by the kinds its field admits; `FactStore.held`) and D7 (the minting moves to `extensions/code/facts`).
Protocol 2.0.0, unreleased: `UnheldReference.kinds` and `FactStore.held` go into the 2.0.0 row, no bump. No code: the
`code/` book keeps E101, W102, W401 and I402, made precise.

## Context

- **A scheme is not a kind.** R1 left one collected kind per scheme (`test` in `js`, `test_run` in `run`), and
  `@code/core`'s `refs` called a citation E101 whenever a served store held facts of its scheme. R2's packs write Rust
  tests, Rust and JavaScript items and packages: a store of `rust` items is no evidence that nextest ran, so
  `proof ["rust:toy#tests::adds"]` beside it would be E101 for a test nobody looked for.
- **Packs share code by linking.** The wire has no guest-to-guest call (ADR 0329). Five packs (`@javascript/node-test`,
  `@rust/test`, `@rust/source`, `@rust/cargo`, `@javascript/source`) must mint identical ids without calling each
  other, and `@code/core` must parse them.
- **SCIP's two grammars** (read from the indexers' sources on 2026-11-08). rust-analyzer
  (`crates/rust-analyzer/src/cli/scip.rs`): `rust-analyzer cargo <package> <version|.> <descriptors>`, the first
  descriptor the target (`lib/`, `bin/<name>/`, `test/<name>/`, `example/<name>/`, `bench/<name>/`, `proc-macro/`,
  `custom-build/`); a trait method `lib/m/MyTrait#func().`; a trait impl member `lib/m/impl#[MyStruct][MyTrait]func().`
  (generic arguments kept in the trait, backticked); an inherent member `impl#[S]new().`; a field `lib/St#a.`; items
  in a function body flatten to the module; locals are `local N`. scip-typescript (`src/Descriptor.ts`,
  `src/FileIndexer.ts`): `scip-typescript npm <name> <version|HEAD>` from the nearest `package.json`, one namespace
  descriptor per directory and one for the file (`` `index.ts`/ ``), a class, interface, type alias or enum `Name#`, a
  function or method `name().`, a constructor `` `<constructor>`(). ``, accessors `` `<get>x`(). ``, a property,
  variable or enum member `name.`, a TS namespace `Name/`.
- **nextest's binary ids**: `<package>` for the lib, `<package>::<test target>` for an integration test,
  `<package>::bin/<name>`, `::example/<name>`, `::bench/<name>`; the test path is libtest's (`mod::tests::name`).
- **Purls**: `pkg:type/namespace/name@version?qualifiers#subpath`; an npm scope is the namespace, its `@`
  percent-encoded (`pkg:npm/%40angular/core`).

## D1. One id library, linked

`extensions/code/facts` (`specforge-code-facts`, an extension library, ADR 0328 D3) holds every id and every minting
function: `TestId::{js, rust}`, `ItemId::{rust, js}`, `Owner`, `PackageId::{cargo, cargo_in, npm}`, `crate_segment`,
`location`, `TestCase`, `test_facts`, `Definition`, `ItemKind`, `Visibility`, `item_facts`, `Package::fact`. Each pack
links it, and `@code/core` links it to parse an id. `extensions/code/junit` keeps only the JUnit reader (`cases`, `Case`,
`relative`); `code-facts` depends on it for `TestCase::of`, never the reverse.

## D2. The crate segment is the package name; another target is qualified as rust-analyzer does

A Rust item's crate segment is the cargo **package** name (with `-`) for a library target (every library crate type:
`lib`, `rlib`, `dylib`, `cdylib`, `staticlib`, `proc-macro`; `TargetKind::cargo` reads cargo's kind), and
`<package>/<kind>/<target>` for a `bin`, `test`, `example` or `bench` target (`<package>/custom-build` for a build
script): rust-analyzer's package and target descriptor, nextest's binary id and the purl all use the package name, so
no manifest is needed to map, and a lib and a bin of one package cannot collide. A second package of one name (a
`cargo_in` package) appends `@<dir>` (`rust:<name>@<dir>::…`): no item id holds `#`, and `@` is in no package, target
or kind name.

## D3. One fact per path

Every definition one path reaches is one fact listing every location: cfg twins, two impls of one trait
(`From<A>`, `From<B>`), a module and a function of one name, a TS `type` and `const` of one name. `item.kind` is
therefore a list. Trait and self types keep their last path segment with generic arguments dropped
(`crate::a::Vec<T>` → `Vec`, `From<io::Error>` → `From`): tree-sitter's source text and rust-analyzer's display of a
type differ (`io::Error` against `Error`), and the last segment without arguments is the one spelling both produce.
Any other type (`&T`, `[T]`, `(A, B)`, `dyn Tr`) is its text with whitespace collapsed. A raw identifier is written
without `r#`.

## D4. The grammar

| Fact | Id | Example |
|---|---|---|
| Rust item, lib | `rust:<package>::<mod>::…::<name>` | `rust:specforge-ops::read::navigate::anchors::find` |
| Rust inherent member | `rust:<crate>::<mod>::<Type>::<name>` | `rust:toy::Board::add` |
| Rust trait item, default method | `rust:<crate>::<mod>::<Trait>::<name>` | `rust:foo::module::MyTrait::func` |
| Rust trait impl member | `rust:<crate>::<mod>::<Type as Trait>::<name>` | `rust:foo::module::<MyStruct as MyTrait>::func` |
| Rust item, other target | `rust:<package>/<kind>/<target>::…` | `rust:specforge-cli/bin/specforge::main` |
| Rust test | `rust:<nextest binary id>#<test path>` | `rust:specforge-ops::golden#reads_a_store` |
| JS item | `js:<package>/<file>::<name>[.<member>…]` | `js:@acme/core/src/board.ts::Board.add` |
| JS test | `js:<package>/<file>#<s1 > … > name>` | `js:app/test/a.test.js#board > adds` |
| Package | `pkg:cargo/<name>`, `pkg:cargo/<name>#<dir>`, `pkg:npm/<name>` | `pkg:npm/%40acme/core` |

- **The ids are disjoint by construction**: a test id holds `#` before its path, an item id never does, a package id
  is in scheme `pkg`. JavaScript items use `::` between file and name and `.` between members (JavaScript's own), so a
  top-level test and an item of one file cannot clash.
- **Escapes**: in a `js` id's `<file>`, `%`, `#` and `:` are percent-encoded; in a JS name segment, `%`, `#`, `.` and
  `:`; `crate_segment`'s `<dir>` encodes `%`, `#`, `:` and `@`. Every id parses back to itself (`FromStr`).
- **An item's test id** (`ItemId::test_id`) is what nextest names that function: the lib's `<package>`, another
  target's `<package>::<target>` or `<package>::<kind>/<target>`, then `#` and the module path without the crate.

**rust-analyzer → id** (total; version and `.` dropped; backtick escapes undone):

| SCIP | Id |
|---|---|
| `rust-analyzer cargo foo 0.1.0 lib/a/b/f().` | `rust:foo::a::b::f` |
| `… proc-macro/S#` | `rust:foo::S` |
| `… bin/main/S#`, `test/t/S#`, `example/e/S#`, `bench/b/S#` | `rust:foo/bin/main::S`, `rust:foo/test/t::S`, … |
| `… custom-build/S#` | `rust:foo/custom-build::S` |
| `… lib/m/impl#[S]new().` | `rust:foo::m::S::new` |
| `… lib/m/impl#[S][Tr]f().`, ``impl#[S][`From<Error>`]from().`` | `rust:foo::m::<S as Tr>::f`, `rust:foo::<S as From>::from` |
| `… lib/m/Tr#f().`, `Tr#C.`, `Tr#Ty#` | `rust:foo::m::Tr::f`, `…::Tr::C`, `…::Tr::Ty` |
| `… lib/m/` (module), `lib/m!` (macro), `lib/C.` (const) | `rust:foo::m`, `rust:foo::m`, `rust:foo::C` |
| `… lib/St#a.` (field), `lib/E#V.` (variant), a parameter or type parameter | the owner's id, `rust:foo::St` |
| `local N` | none |

The SCIP index is per workspace, so its root names the nearest package of a name; another maps to `rust:<name>@<dir>::…`.

**scip-typescript → id**: `scip-typescript npm <name> <ver> d1/…/`file.ts`/D…` → `js:<name>/d1/…/file.ts::<names>`.
The file is the first namespace descriptor ending in a source extension (a TS namespace identifier holds no `.`); the
remaining descriptors' names are joined by `.` with their suffixes dropped; `<constructor>` → `constructor`,
`<get>x` and `<set>x` → `x`; parameters, type parameters, meta and descriptors no fact is minted for map to the
nearest minted owner.

Where SCIP is finer than the ids, the map is many-to-one and the one fact lists every definition (D3): "maps into
exactly" holds, and a later `@rust/scip` or `@javascript/scip` pack adds references to the same ids. The two tables
are tests of the library (`rust_analyzer_symbols_map_into_ids`, `scip_typescript_symbols_map_into_ids`).

## D5. Package ids are purls without version

`pkg:cargo/<name>`, `pkg:npm/<name>` (scoped `pkg:npm/%40scope/name`). The version is a field. A second cargo package
of one name takes the purl subpath, `pkg:cargo/<name>#<dir>` (`PackageId::cargo_in`). A JavaScript package fact takes
the same kind: `engines` and `depends_on` to the project's own packages are its fields too.

## D6. Locations are ADR 0346's `Fact.at`, every one of them

A merged item lists every definition's place; goto offers each. `specforge_code_facts::location(file, start, end)`
turns the reader's `SyntaxPoint`s (1-based lines, 1-based byte columns) into a `FactLocation`: the one conversion every
source pack uses.

## D7. `item.test_id` points from the item to its test

One writer per edge: `@rust/source` knows the function and computes the id (`ItemId::test_id(edition)`); `@rust/test`
writes the test under the same id. No `test.item`. The item id writes a raw identifier without `r#` (D3), but libtest
names a test as rustc prints its path, a word the crate's edition reserves raw (`r#async::r#match`; `gen` only from
2024, `async`, `await`, `dyn` and `try` from 2018), so the test id takes the package's edition (`Edition::cargo`).

## D8. `test.file` is project-relative, as the pack gives it

ADR 0329 D3 said so and the JUnit library wrote the path under the package (`test/x.test.js` for `@acme/core` at
`packages/core/`). `TestCase` takes the file from the pack; the id keeps the package-relative file.

## D9. The pass reads the kinds from the wire

```rust
pub struct UnheldReference { field, id, span, kinds: Vec<String> }        // the collected kinds its field admits
pub struct FactStore { collector, key, held: BTreeMap<String, Vec<String>>, facts } // scheme → kinds it holds
```

- The kernel already decides an unheld reference as "its scheme's collected kinds include one its field admits"; it
  now sends those kinds (`Resolution::Unheld { kinds }`, `graph::Unheld.kinds`). A store's `held` replaces `schemes`:
  by scheme of its facts' ids, the kinds of the facts it holds.
- `@code/core`'s `refs`, for each unheld reference of an entity that is not pending and not of one of `@code/core`'s
  kinds (a fact's own references are its collector's, judged as ADR 0346 D4 says):
  1. nothing unless the reference's `kinds` meet `@code/core`'s kinds. Those kinds are narrowed to the ones the id
     parses as through the library (a test or run id holds `#`, an item id never does), all of them when it parses as
     none, so a malformed reference is still judged;
  2. **E101** when a served store's `held[scheme]` meets them. A test reference is told the closest held test of the
     same container (`TestId::container`); no item or package suggestion, since `refs` reads test facts only;
  3. else **W102**: "`<field>` names `<id>`, but no `<scheme>` `<kind>` facts were collected: run `specforge collect`
     with `<pack>`", the pack from `@code/core`'s table, by kind and by the scheme of a test's or item's id or the
     purl type of a package's: (test, js) `@javascript/node-test`, (test, rust) `@rust/test`, (item, rust)
     `@rust/source`, (item, js) `@javascript/source`, (package, cargo) `@rust/cargo`, (package, npm)
     `@javascript/source`.
- A judged `test(..)` input (ADR 0331 D7) reads an unheld reference by the same rule: `absent` (the test is gone) only
  when a served store's `held[scheme]` meets the reference's `kinds`, unrecorded otherwise.

## D10. The library normalises visibility

A pack hands over what it captured: `Visibility::rust(modifier)` (`pub` public, `pub(crate)` crate, `pub(self)`
private, `pub(super)` and `pub(in …)` restricted, none private) and `Visibility::js(exported)` (public or private).
`@rust/source` and `@javascript/source` cannot disagree, and no reader parses modifier text. A merged item takes the
widest.

## D11. `names` is a reference list on `test` and `item`

`test.names` and `item.names` are reference lists with no target kind, edge `names`: ADR 0346 D4 links them. `I402`
reports each dangling one of a `test` or an `item` (ADR 0346 D8).

## The kinds (`@code/core` 1.1.0)

| Kind | Schemes | Fields |
|---|---|---|
| `test` | `js`, `rust` | `package`, `file`, `suite`, `name`, `status` (result), `names`, `binary` (the nextest binary id) |
| `test_run` | `run` | `test` (edge `run_of`), `status`, `duration_ms` |
| `item` | `rust`, `js` | `package` (→ `package`, edge `in_package`), `file`, `kind` (list of `ItemKind`), `visibility` (`public\|crate\|restricted\|private`), `signature` (`sha256:` digest), `doc`, `attrs`, `names`, `test_id` (→ `test`, edge `runs_as`) |
| `package` | `pkg` | `name`, `version`, `edition`, `rust_version`, `engines`, `features`, `targets` (`<kind>:<name>:<root>`), `path`, `depends_on` (→ `package`, edge `uses_package`) |

An item's title is its own unqualified name (`add` for `…::Board::add`, `fmt` for `…::<Foo as Display>::fmt`). Its
`signature` digests the definitions' signature texts, each whitespace-collapsed, sorted, deduplicated and joined by a
newline. No `depends_on` edge label: `@specforge/product` reads one for features.

## Rejected

- **The `use` crate spelling (`specforge_ops`).** A lib's `[lib] name` would need the manifest to map, and a lib and a
  bin of one package would share it.
- **Namespace suffixes** (rustdoc's `f()`, `m!`, SCIP's `/ # . ().`). Ids would no longer read as Rust and TS write
  paths, and cfg twins need the merge anyway.
- **`#` or a `ts:` scheme for JS items.** `#` clashes with a top-level test of the same file; a `ts:` scheme splits
  one package's facts.
- **SCIP symbols as ids.** Version-bound and spaced.
- **A versioned purl.** The version is a field that changes on every release.
- **`@code/core` hard-coding which of its kinds share a scheme.** It breaks the first time another extension declares a
  kind in `rust`.
- **A `test.item` edge beside `item.test_id`.** Two writers of one relation.

## Consequences

- `proof ["rust:toy#tests::adds"]` beside a store of `rust` items only is W102 naming `@rust/test`, not E101.
- A nested package's test fact files its project-relative path.
- `@code/core` 1.1.0 still matches a peer on `^1.0`.

## What would reopen it

- A SCIP display that differs from the normalised owner text for a non-path self type (`impl Tr for &T`): the SCIP pack
  would need a normalisation table.
- A project that needs two npm packages of one name.
- A pack that needs a namespace-exact id.
