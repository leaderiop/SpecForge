# A fact says where it is written and what it names

**Status:** accepted (2026-11-08, R2 of the Beyond Parity build plan). **Amends**
[ADR 0327](0327-a-collectors-facts-are-read-only-entities-recorded-by-digest.md) D2 (store version 2 with `sources`),
D5 (a served fact is `located`, its names spanned where they are written), D6 (a fact's own references: dangling,
not a kernel error; a unique suffix) and D7 (a rename of an entity a fact names rewrites the source; a delete is still
refused), and [ADR 0329](0329-code-core-holds-test-facts-and-their-ids.md) D3 (`names` is a fact-held reference).
Protocol 2.0.0, unreleased: its additions go into the 2.0.0 row, no bump. Codes E117, E120, W197.

## Context

- A collected fact (ADR 0327) has no place in source. Every fact entity is spanned at the first character of its store
  JSON, so a finding about a fact points at `{` of `.specforge/facts/<key>/<collector>.json` with a remedy addressed
  to a spec author, and the editor cannot go to it.
- A fact's reference is linked by the one linker as a block's is: one that names no entity, the wrong kind or several
  is a kernel E003, E022 or ambiguous E003 at the store's brace. Nobody can fix it in the store: the collector writes
  it again on the next collect.
- The store span is a fact's identity: E115, the store listing a pass is sent and every `filed(record.span.file)` read
  it. It cannot become the source location.
- A fact id two stores hold is E002 "rename one of the entities to avoid the collision". There is no entity to rename:
  two collectors mint one id.
- `notes [js:p1]` (a reference holding `:` written without quotes) is three E001 and a spurious E003 on the scheme
  word.
- A rename of an entity a fact names is refused, "the fact has no text to rewrite". A source pack (`@rust/source`,
  `@javascript/source`) reads the name from a test attribute or a meta tag: there is text, in a source file.

## D1. `at` is a list, `named_at` a map field → id → places

```rust
pub struct FactLocation { file, start_line, start_col, end_line, end_col }   // project-relative, `/`
pub struct Fact { kind, id, title, fields,
                  at: Vec<FactLocation>,                                         // where the fact is written
                  named_at: BTreeMap<String, BTreeMap<String, Vec<FactLocation>>> } // field → id → name spans
```

- A location is the host `SourceSpan`'s convention: a project-relative path with `/`, 1-based lines, 1-based **byte**
  columns, end exclusive. A pack converts the reader's points (`SyntaxPoint.byte_col` is already 1-based).
- **A cfg twin is one fact at two places**, so `at` is a list; `Fact::at(place)` pushes one. A rename must rewrite
  every place a name is written, so `named_at` lists every place of every id; `Fact::name(field, id, place)` appends
  `id` to the field's array once and records the place.
- Both are omitted when empty: a fact with no place costs nothing on the wire or in the store.
- *Rejected:* a parallel array beside `fields` (breaks when an id repeats); an object form of a reference item
  (`{"id", "at"}`), which every reader of `fields` would have to unwrap.

## D2. The store is version 2 and records the digest of every file its places name

```json
{"store": 2, "collector": "rust-source", "extension": "@rust/source", "key": "<key>",
 "sources": {"crates/a/src/lib.rs": "sha256:<64 hex>"},
 "facts": [{"kind": "item", "id": "rust:a::adds", "at": [{"file": "crates/a/src/lib.rs", "start_line": 12, ...}],
            "fields": {"names": ["add_numbers"]},
            "named_at": {"names": {"add_numbers": [{"file": "crates/a/src/lib.rs", "start_line": 11, ...}]}}}]}
```

- **The host keys and digests the places at admission.** Every location's `file` is keyed from the project root by
  the recorded-path reader (ADR 0271): the key replaces the written spelling. A path that is no key (outside the root,
  `..` above it, absolute elsewhere) or names no regular file is an E116 reason ("fact '<id>' is placed at '<file>',
  which is no file under the project root"), and none of the collector's facts is stored. Each distinct file is read
  once and its sha256 recorded in `sources`.
- A version 1 store is W192 ("it is store version 1, and this specforge reads version 2") until the next collect.
- *Rejected:* a guest-supplied digest (a guest could lie or forget); hashing sources on every compile (cost, and only
  a rename needs it). Cost: one read and one sha256 per located file per collect.

## D3. A served fact keeps its store span and gains `located`

- The entity's span stays the store's first character: it is what identifies a fact. Its places are a second range,
  `located` (`Entity.located`, `Node.located`, `EntityRecord.located`); `Node::place()` and
  `EntityRecord::place()` are its first place, else the store span.
- A reference value's span is its first `named_at` place, else the fact's first `at`, else the store span. So every
  finding about a fact-held reference, the kernel's or a pass's, lands at the name.
- A pass finding with no span is placed at `place()`, not at the store's brace.
- Export and query nodes carry `located` (`[{file, start_line, start_col, end_line, end_col}]`, omitted when empty);
  MCP inspect serves it.
- *Rejected:* re-spanning the entity at its source: every `filed(record.span.file)` would break.

## D4. A fact's reference that names no entity is dangling, judged by its kind's extension

A holder whose span is a fact store is a **fact holder**. It resolves each reference from the project, a unique
suffix taken, as a surface input does (a test attribute writes the short name its author sees: `TR1` names
`v1/02/TR1` when that is the one). Then:

| Resolution | Outcome |
|---|---|
| linked | an edge (unchanged) |
| unheld (a fact scheme no store served) | `unheld` (unchanged), now at the name span |
| unavailable (target kind not loaded) | kernel I004 (a project fix, true for a fact too), at the name span |
| unresolved, ambiguous, mistyped | **no kernel diagnostic**: a `dangling` entry on the node, with its reason, `did_you_mean`, `candidates` and the actual kind; a mistyped one keeps its edge, as an authored one does |

- `dangling` rides on `PassEntity` (`DanglingReference { field, id, span, reason, actual, did_you_mean, candidates }`)
  for every pass: the pass input is encoded once. The extension that declares the holder's kind is the one that
  judges it. An extension that declares a collected kind with a reference field and no pass leaves them unreported.
- An authored entity's references are linked exactly as before.

## D5. A fact id two stores hold is E117

Two declarations of one `(kind, path)` that are both facts are **E117** at the second store: "fact '<id>' is collected
by both '<c1>' (<store1>) and '<c2>' (<store2>); the first is served", suggestion "two collectors mint one id: load
only one of them, or fix the pack that mints it, then run `specforge collect`". Any other duplicate stays E002.

## D6. A bare colon reference is one E120

In a value (a list item or a single value), a word directly followed by `:` and more id bytes is a reference written
without its quotes: one **E120** spanning `word:rest`, "a reference holding ':' is written quoted: \"js:p1\"",
suggestion `write "js:p1"`. No E001 for it, and no E003 for the scheme word, which leaves the recovered value. The
file still counts as unparsed: it cannot round-trip. Whatever the scheme, the fix is the same quotes. A bare
`scheme.kind:id` stays a provider ref.

## D7. A rename of an entity a fact names rewrites the source where the fact read it

```text
rename T1 → T2
  ├─ spec occurrences ──────────────▶ edits in compiled text (unchanged)
  ├─ each fact naming T1 (its name resolved as the linker resolves it), read from its store as served
  │    (a name an earlier rename rewrote is the name it wrote): places = named_at[field][old spelling]
  │    no place ─────────────────────▶ refused `collected` ("records no place it is written")
  │    file F changed since the collect (digest ≠ sources[F]) or the text at a place ≠ the old spelling
  │                                   ──▶ refused `source_moved`
  │    else F edited at every place: the written suffix of the old path → the same suffix of the new path
  └─ one planned change: spec files + source files + ledger (moves, and for each name rewritten
       `<collector> stale:<key>:<field>:<old spelling>:<new spelling> <digest of the store bytes>`,
       written even with no harness field)
serve a store whose stale lines the ledger holds ──▶ exactly those names read as the rename wrote them + W197
next collect of that collector ──────────────────▶ replaces its facts: and stale: lines; no W197
```

- **Stale is a ledger line per rewritten name, not a store rewrite.** The store stays the collector's bytes; a stale
  line records one name the rename rewrote in the source (the field, the spelling the store holds, the spelling
  written) and digests the store's bytes, so "stale" is exact in time and in what it follows: only a store collected
  before the rename, and only the names that rename wrote. A fresh store that still names the old id is dangling,
  with `did you mean` the new one: a session's check takes it from the ledger's moves (the old path written as a
  fact writes a name), the kernel's close match otherwise. It is union-merge safe (`stale:<key>` holds a namespace). *Rejected:* rewriting
  the store's values (a second writer of facts; the LSP would have to land a store write); following the moves for
  every store (a reverted source edit would link silently forever); following the ledger's moves by suffix for a
  stale store (an older move of `a/T1` would re-point a `T1` that names the root `T1`).
- **The renames reach the serve on the served text.** A session reads a store once (`facts::store` is its one
  reader); the text it serves is the collector's bytes, then the renames its stale lines record as one more JSON
  value. The session compares served texts, so a ledger change re-serves the store, and the on-disk `Store` has no
  host-only field. *Rejected:* a `moved` field on the store struct set by the host (a field no collector writes, and
  the store parsed, re-serialized and parsed again).
- **W197** "fact store '<key>' is stale: a rename moved what its facts name ('T1' → 'T2'[, and N more]) after the
  collect; its places in <files> may be off until `specforge collect`".
- A second rename needing a place in a file the first rewrote is refused `source_moved` (its digest moved): one
  rewrite per file per collect, exact by construction. It finds the place because it reads the store as served: the
  name the first rename wrote, at the place it wrote it. Within one plan, a rename that needs a file an earlier
  rename of the plan rewrites is refused `source_rewritten`.
- The LSP rename's edit carries the source edits; the pending ledger change carries the stale line and lands as the
  moves do.
- **A delete of an entity a fact names stays refused.** It would remove what a test says it proves, and there is no
  text to delete in an attribute without guessing. Change the source, collect, then delete.

## D8. `names` is a fact-held reference

ADR 0329 D3's `names` ("the entities a test says it proves") is a fact-held reference: the rule of D4 links it. Its
declaration as a reference list and its proof diagnostic over the dangling names come with the change that declares
the item kind.

## Consequences

- A finding about a fact is placed where the fact is written, with a remedy addressed to its collector.
- A rename of an entity a test attribute names works from the editor and MCP, and leaves the store stale until the
  next collect.
- Every existing store is W192 until the next collect (pre-release).
- W197 tells that a stale store's other places may be off by the rename's column shift.

## What would reopen it

- A pack that cannot give a name's place: its names link and are judged, but their entities cannot be renamed.
- A reader that needs to tell a fact's place in a file the fact store no longer digests (a moved source tree).
