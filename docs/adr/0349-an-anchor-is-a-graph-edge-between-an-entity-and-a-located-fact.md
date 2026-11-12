# An anchor is a graph edge between an entity and a located fact

**Status:** accepted (2026-11-10, R2 of the Beyond Parity build plan). **Supersedes**
[ADR 0140](0140-source-anchors-are-a-read-view.md) D1 (the manifest the read view read) and D3 (stale rows), and
[ADR 0199](0199-a-source-item-is-covered-by-the-anchor-that-names-it.md). **Amends**
[ADR 0249](0249-the-editor-client-is-an-adapter-over-the-language-server.md) D2 (the server answers code lenses on
source files too; D1 unchanged). Builds on [ADR 0346](0346-a-fact-says-where-it-is-written-and-what-it-names.md)
(`Fact.at`, `Fact.named_at`, fact-held references, a store's `sources` digests, rename rewriting source) and
[ADR 0347](0347-item-and-package-facts-and-ids-a-scip-symbol-maps-into.md) (item, test and package facts). Codes W198
and I212. No wire change, no re-vendor.

## Context

- **The manifest nothing writes.** The anchors read view parsed `<root>/specforge-anchors.json` (ADR 0140), a file no
  code in the workspace writes. `specforge.find_implementation` and `specforge.find_spec_for_source` answered
  `count: 0` on every project, SpecForge's own included, while 5,750 test attributes in 863 files name 665 of its
  entities (`#[specforge_test(behavior = "…")]`, `#[spec(…)]`, `#[specforge_test_macros::test(…)]`).
- **The graph holds what the manifest promised.** A source pack writes located facts (`Fact.at`): `@rust/source` an
  `item` per definition with the entities its attribute names in `names`, `@javascript/source` the same for
  JavaScript. A fact's `names` resolves against authored entities into an edge labelled `names`; a `.spec` field
  citing a fact (`proof ["rust:…"]`, a gsd task's `items`) is an edge the other way.
- **An empty answer said nothing.** "No anchor" and "nothing was collected" were one answer.
- **The editor stopped at the `.spec` file.** Go-to-definition on a fact reference landed on the store; no request
  answered an entity's implementations, and a source file had no lens.

```text
BEFORE  nothing writes ─▶ specforge-anchors.json ─▶ manifest reader ─▶ find_* / file scope / gaps
AFTER   .spec refs to facts ─┐
        facts naming entities ┴▶ graph edges ─▶ anchors_in ─▶ find_* · specforge anchors · gaps · LSP
```

## Decisions

- **D1. An anchor is an edge, either way, read in one walk.** An edge of the compiled graph whose one end is an
  authored entity (a kind no collector writes) and whose other end is a located fact (a node with a place) is an
  anchor: entity → fact by a reference a `.spec` writes, fact → entity by a fact-held reference. An edge between two
  facts (`test_run.test`) or two authored entities is none. `specforge_ops::read::navigate::anchors_in(view)` walks
  the edges once and indexes the anchors by entity and by fact; `anchors_of_entity`, `anchors_of_file`, covered items
  and the language server read it. The manifest, its reader and its rows are deleted.
- **D2. `via` is `<holder kind>.<field>`.** The edge's label is the declared field; the holder's kind says which side
  wrote it: `item.names` when a fact names the entity, `task.items` when the entity's field names the fact. One string,
  not an object: the same information with less schema.
- **D3. A row is the fact's place.** `{fact_id, file, line, end_line, item_kind, collector, via}`: the fact's first
  place (`Node::location()`), its collector read from its store's path, and per entity of a file query its id and
  kind. Rows are ordered by `(file, line, fact_id, via)`, one per edge. An id the graph does not declare is always
  E003: with anchors as edges there is no row without an entity; a fact naming a missing id is ADR 0346's dangling
  reference at its source span.
- **D4. `item_kind` reads the fact's field named `kind`.** Its values, joined by `, ` (an item's `kind` is a list: a
  module and a function of one path are one fact), when the fact's kind declares a field named `kind`; else the
  fact's kind name (`test`, `package`). One field-name convention in a domain-free view; a `located_kind` declaration
  in the SDK would change the wire for a display string.
- **D5. Staleness by the store's digests, noticed by the read view.** A store records the digest of every source file
  its places name, as the host read it at admission (ADR 0346 D2); the compiled project serves each store's, by file
  and store, `CompiledProject::fact_sources()`, and ops' `CollectedSources` is the one check against them. W198 ("A
  source anchor is stale"): a file an answer names no longer hashes to the digest the store of one of its rows
  recorded (two stores collected at different times keep two), or is gone; one per such file, its rows still listed. Only the answer's files are hashed. I212
  ("No located facts to anchor to"): the session serves no located fact; the remedy is to load a source pack and run
  `specforge collect`. Both are notices of the read view (ADR 0111 D8), never `check` findings: `check` hashes no
  source file.
- **D6. One CLI command, `specforge anchors <ENTITY> | --file <PATH>`.** Two questions of one read view; the two are
  exclusive. Human output is one line per row, `--format json` the MCP reply document (ADR 0052).
- **D7. Source documents are answered, never compiled.** The language server holds a `.rs`, `.ts`, `.tsx`, `.js` or
  `.jsx` buffer under the project root (for hashing and placing) and never sends it to the session: typing in a source
  file rebuilds nothing. A span in a source file is placed through its text (the open buffer, else the file read
  through the session's containment) only when the text's digest is the one its store recorded; a stale file places
  nothing, and a rename that would edit it is refused with W198's sentence.
- **D8. In a source file the server speaks only at facts.** Definition, hover, references, prepare-rename and rename
  answer on a token a fact names an entity by; code lenses sit at each located fact with an anchor, one per entity,
  running `specforge.findReferences` (ADR 0249 D2's one command); a stale file has one lens with W198's message.
  Nowhere else: a definition over every identifier would merge with the language's own server on each goto.
  `textDocument/implementation` on an entity in a `.spec` lists its anchors' source places.
- **D9. The attribute is read once, by `@rust/source`.** `check-links` stays a developer tool that also reads a test's
  `verify` text, which item facts do not carry.
- **Covered items.** A public item fact is covered when it has an anchor to an entity the graph declares: inference
  gaps are the rest. ADR 0199's manifest half is deleted with the manifest, its name rule already was.

## Rejected

- Only `.spec` references to facts: SpecForge's own exit has none; its tests name its entities.
- Only facts naming entities: loses gsd `task.items` and a `proof` citing a test.
- A digest per fact (`Fact.at`): 64 hex characters on each of ~37k facts; mtime: not content.
- Two CLI commands (`find-implementation`, `find-spec-for-source`): two commands for one view.
- Routing source buffers through the session: a rebuild per keystroke in a `.rs` file.

## Consequences

- Breaking (MCP): `find_implementation` rows are `{fact_id, file, line, end_line, item_kind, collector, via}` (no
  `symbol_name`, `scanner`); `find_spec_for_source` rows gain `fact_id`, `end_line`, `collector`, `via`, lose
  `symbol_name`, `confidence`, and always name the entity's kind. `specforge-anchors.json` is not read. An id the graph
  does not declare is always E003. The replies carry I212 and W198.
- Breaking (inference gaps): an item is covered only by an anchor; E071 is no longer said of the anchors.
- CLI: `specforge anchors`. `specforge init` enables `@code/core` with `@rust/cargo` and `@rust/source` for a
  `Cargo.toml`, with `@javascript/source` for a `package.json`.
- LSP: `implementationProvider`; source documents answered for code lenses, hover, definition, references,
  prepare-rename and rename. Breaking (VS Code): every open Rust, TypeScript and JavaScript buffer of a SpecForge
  workspace is sent to `specforge-lsp`.
- SpecForge's own `specforge.json` loads `@code/core`, `@rust/cargo` and `@rust/source`; a collect at its root writes
  only ignored files.

## What would reopen it

- SCIP references: a `@rust/scip` or `@javascript/scip` pack's references are more edges to the same ids, so more
  anchors with no change here; a reason to cache `anchors_in` on the compiled project if the walk shows in a profile.
- A kind that locates facts with a field named `kind` meaning something else than a definition's kind.
