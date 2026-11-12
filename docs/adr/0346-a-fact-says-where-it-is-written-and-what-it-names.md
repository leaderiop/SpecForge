# A fact says where it is written and what it names

**Status:** accepted (2026-11-08, R2 of the Beyond Parity build plan). **Amends**
[ADR 0327](0327-a-collectors-facts-are-read-only-entities-recorded-by-digest.md) D2 (store version 2 with `sources`),
D5 (a served fact is `located`, its names spanned where they are written), D6 (a fact's own references: dangling,
not a kernel error; a unique suffix) and D7 (a rename of an entity a fact names rewrites the source; a delete is still
refused), and [ADR 0329](0329-code-core-holds-test-facts-and-their-ids.md) D3 (`names` is a fact-held reference).
Protocol 2.0.0, unreleased: its additions go into the 2.0.0 row, no bump. Codes E117, E120, W197.
Amended by [ADR 0353](0353-a-pass-reads-the-facts-it-declares.md) (D4: `dangling` rides on the facts a pass is sent, which are those its `facts` input names).
D7 amended 2026-11-11: renames compose (a served store's places are shifted by what the renames wrote; one stale line
per name held, replaced by a later rename; `source_rewritten` is gone).

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
- **Every file a store places a fact in has its digest.** The collect writes it for every place it admits (above), so
  only another writer leaves one out (a hand-made store, another tool). The one reader (`facts::store`) refuses such
  a store, W192 ("its fact '<id>' is placed in <file>, of which it records no digest"), and so does the derivation of
  what renames wrote (D7): no served store places a fact in a file without its `facts::Placed`, so every judge of a
  file (W198, the language server's placing, a rename) and the shift of D7 hold for every place. *Rejected:* serving
  it with its places in that file unshifted and unjudged, and a diagnostic (its findings would be placed at guessed
  positions, a rename of its names refused for a reason the store's writer caused, and every judge would carry the
  case of a file it cannot judge, when `specforge collect` makes the store whole); serving its facts with those
  places dropped (a third shape of a store, for a store no collect writes).
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
  │    (a name an earlier rename rewrote is the name it wrote, at the place it left it)
  │    the ledger renames a name the store holds two ways (W202) ─▶ refused `stale_conflict`
  │    no place ─────────────────────▶ refused `collected` ("records no place it is written")
  │    F (its held text, else the disk's) is not the text the store's places are in, or the text at a place
  │    ≠ the name served ────────────▶ refused `source_moved`
  │    another fact of the store holds the name in that field for something else ─▶ refused `name_shared`
  │    a write would change a name any fact holds in F that the rename does not rename ─▶ refused `name_shared`
  │    two places it writes cross one another ─▶ refused `places_cross`
  │    else every place the store writes the name: its spelling there → the same suffix of the new path
  │    (places nested one in another write one text, the outermost each)
  └─ one planned change: spec files + source files + ledger (moves, and for each name rewritten
       `<collector> stale:<key>:<field>:<name held>:<new spelling> <digest of the store bytes>`,
       replacing the line of that store, field and name held; dropped when it writes the name held back)
serve a store whose stale lines the ledger holds ──▶ those names read as now written, every place in a file
                                                     they rewrote shifted by what they wrote, + W197
next collect of that collector ──────────────────▶ replaces its facts: and stale: lines; no W197
```

- **Stale is a ledger line per name held, not a store rewrite.** The store stays the collector's bytes; a stale
  line records one name renames rewrote in the source (the field, the spelling the store holds, the spelling written
  last) and digests the store's bytes, so "stale" is exact in time and in what it follows: only a store collected
  before the rename, and only the names renames wrote. A later rename of the name replaces the line (its key is the
  store, the field and the name held), so the ledger says where each name is now whatever the order its lines are
  read in, and two branches each renaming a different name merge (`merge=union`) into both lines. Two branches that
  renamed the same name merge into two lines for it, and the ledger no longer says where it is: the store serves
  none of its renames (its names and places as collected), **W202** "fact store '<key>': specforge-harness holds two
  renames of 'T1' ('T2', 'T3'); none of its renames is served, until `specforge collect`", and a rename of a name it
  holds is refused `stale_conflict` until the next collect. A fresh store that
  still names the old id is dangling, with `did you mean` the new one: a session's check takes it from the ledger's
  moves (the old path written as a fact writes a name), the kernel's close match otherwise. It is union-merge safe
  (`stale:<key>` holds a namespace). *Rejected:* rewriting the store's values (a second writer of facts; the LSP would
  have to land a store write); following the moves for every store (a reverted source edit would link silently
  forever); following the ledger's moves by suffix for a stale store (an older move of `a/T1` would re-point a `T1`
  that names the root `T1`); a line per rename from the spelling it read (`T1→T2`, `T2→T3`: a set of lines has no
  order, and a rename back makes a cycle).
- **What renames wrote is derived, so the places are exact by construction.** Every place a fact of a stale store
  names one of its stale lines' held names in that field (`named_at`) is a place the renames rewrote, the held
  spelling replaced by the spelling written, a span of the text that store collected (`sources[F]`). The edits of a
  text are derived once, from every stale store the ledger serves that collected it, and every store placed in the
  file that collected that same text is served shifted by all of them, one that holds none of the renamed names
  included: its places are exact in the text the renames left, and a rename of its own names applies. The serve
  applies them to every place the store records in that file (`at` and every `named_at`): a place before an edit
  stays, one after it moves by the length the edit changed (on its line; by the lines an edit spanning lines
  removed, after it), and an edit's own place becomes what it wrote. Places nested one in another are said exactly
  (`facts::Writes`, the one owner of how a text's places move): two facts may hold names one inside the other
  (`v1/T1` read whole by one collector, its `T1` by another), and a rename moving what both name writes both. The
  outermost place is what is written; a nested one is read inside what it writes, found by the text after it (a
  respelled path keeps what follows the segment renamed), else by the text before it. A place of something no write
  is (a fact's `at`, a name not renamed) inside a write is carried by the same rule; one whose text a write changes
  cannot be, and its store is served as collected in that file, which holds there only as the text collected. The
  derivation does not depend on the order of the stale lines: the places are taken by span. What it cannot say
  exactly is text no rename writes: one place two stores say was written two ways (stores at one span hold one text,
  and a rename writes every store naming what it moves; merged branches renaming one held name give W202, not this),
  places that cross (a collector's names are tokens, which nest or lie apart; a rename refuses them,
  `places_cross`), and a name a write changed that its store does not rename (a rename refuses it, `name_shared`).
  Their stores' places are served as collected there, and the file is not the text they are in (W198, a rename
  `source_moved`). A store that collected another text of the
  file (an older collect) is not shifted by these edits: they are not spans of its text. A rename plans its
  source edits from the same derivation: for each name it rewrites, every place the served store writes it, in the
  served coordinates, and a stale line for every name held that is served as it (two when renames made two held
  names read as one, so each place written is one a line derives). A rename to a name a fact holds for no entity
  would make two such names, and is refused `name_captured` (the store's names are written tokens the capture
  check reads). A rename writes only names of what it moves: one whose write would change a name any fact holds in
  the file that it does not rename (at the place written, holding it, or inside it in a part the name written does
  not keep) is refused `name_shared`, as that fact would read another name there. *Rejected:* recording each rewrite's digest and edits in the ledger (a second record of what
  the stale lines already say, ordered by rename, which a union merge of two branches breaks).
- **A source file is judged against what the store says of it** (`facts::Placed`, the one judge: the anchors read
  view's W198, the language server's placing, a rename): a text is the one the served places are in exactly when
  each place a rename rewrote spells the name written now and putting back the name held at each gives the bytes
  whose digest the collect recorded (`sources[F]`). Anything else edited in the file, a rename reverted by hand
  included, fails it: W198, and a rename refused `source_moved`.
- **The renames reach the serve on the served text.** A session reads a store once (`facts::store` is its one
  reader); the text it serves is the collector's bytes, then what the ledger says of it as one more JSON value: the
  renames its stale lines record, and what every stale store's renames wrote in source files. The session compares
  served texts, so a ledger change re-serves the store, and the on-disk `Store` has no host-only field. What a store
  serves depends on the other stale stores' bytes, so a session that reads one store again reads them all again
  (one that reads as it holds it changes nothing). *Rejected:* a `moved` field on the store struct set by the host (a field no collector writes, and
  the store parsed, re-serialized and parsed again).
- **W197** "fact store '<key>' is stale: a rename moved what its facts name ('T1' → 'T2'[, and N more]) after the
  collect; its places in <files> are served as the rename left them, until `specforge collect`".
- **Renames compose on every surface.** A second (third, …) rename of a name an earlier rename rewrote reads the store
  as served: the name written, at the place it was left; it writes the file again and replaces the stale line. A
  source file is read as the view holds it (`ProjectView::holding`), else from the disk: within one plan, a later
  rename reads the file as the plan's earlier renames rewrote it and the stores through their stale lines
  (`CompiledProject::with_facts_after`), and the plan writes the file once, from the disk's text to the last; the
  language server holds its open source buffers, so a rename composes with one applied to the buffers and not yet
  saved. The editor's renames of an entity and of whatever their moves touch are one history, whichever files declare
  them: a rename whose moves start from, or sit under or above, a path a history's renames moved joins it, and one
  that touches several histories merges them into one, its renames in the order answered. Each rename hangs under the
  rename that gave the entity it renames (its path traced back through the moves in effect since: two scoped entities
  may share a name), the first under the entity as it was, so an undo then another rename branches and a redo walks
  back down; each also knows the renames in effect, in every history, when it was planned, whose paths its moves are
  written after. A history spans every file its renames edit: it ends when all of them close and lands when all are
  saved or closed. An editor undoes per buffer, so each file goes back apart; what a history shows and lands is
  therefore never kept as a phase, but read from the texts each time. A rename is in effect while the spec files it
  edited hold it, with the renames it came from (of two renames of one entity from one node, the later): its declaring
  file and each file it wrote the name in (a reference, or a path through it that places what it scopes: `milestone
  v1.1` in a phase's file, `phase v1.1/04` in a test's), each read on its own, since an editor undoes per buffer. A
  file holds a rename while it reads the name the rename wrote at the places it wrote it (carried as a source file's
  places are, below), or holds a later rename of the same entity made over it; it went back while it reads instead the
  name the rename read at the places it read it. Whether the declaring file holds a rename is counted (the new name
  declared, and one entity fewer with the old name than when it was planned or one more with the new), not asked by
  name, so an entity declared by hand under the old name beside the renamed one leaves the rename held and no move
  back lands onto it; it went back while its names count as when the rename was planned. A rename some of its files
  hold while others went back (its declaring file undone and saved alone while the files that place what it scopes
  still hold it, or the reverse) is in no state of the history, below; a file whose places say neither (edited
  otherwise, or deleted) decides nothing. Source files are not asked: each stale line reads them apart. Each stale
  line (store, field, name held) holds a rename: the latest every source file it is about holds or came after, a file
  holding a rename while the name it wrote for the line reads at the places it wrote it, carried through whatever else
  changed in the file since (another entity renamed, typing, a re-indent or a formatter run, a block pasted beside it,
  lines swapped): its line kept (anchored on the lines no other line of either text repeats, shared at the edge of the
  changed stretch it is in, or moved whole, the only line of its text in either) reads; inside a changed stretch, the
  name is counted where it reads whole, neither byte beside it one that would extend it (a letter, a digit, `_`: `//
  covers T2, T5` still reads `T2`, `T2x` does not), in the stretch of each text, and when the counts match the place
  is the occurrence of the same ordinal, which reads while it is beside what the place was beside: the text before it
  on its line or the text after it (up to the line's edge or the next whole name), whitespace collapsed, a side blank
  in the earlier text keeping nothing; else it reads as gone (typed over, grown, written once more in the same
  stretch, so that which of two the rename wrote is not said and neither is taken, or found on a line of its own or
  among other text). Two cases read alike by construction and are taken as kept: one name typed over and another added
  in the same stretch beside what the first was beside, and a copy of a whole line pasted elsewhere with the original
  then edited (read as the line moved); the stale line then claims a name the store's place no longer says, and W198
  says so at the next collect; else the last rename marking it in effect when the first whose name read still reads at
  its places was planned (an undo); else none (the name typed over with one no rename wrote). So the editor keeps a
  line wherever the CLI and MCP rename keep it, and drops it only where the name itself went back or changed. With one
  store placed in two files, the line is that of the earliest rename either holds: one line names one name for both,
  so a file undone alone leaves the other W198 until the next collect. What is shown and landed is the change from
  what the ledger holds of the history (nothing, until a landing) to what the texts hold, as one change: the moves of
  the renames left undone, each undone and the last made first, and of those taken up, in the order made, composed
  (one change re-keys a line by the nearest moved path above it, ADR 0287 D5); each stale line whose rename moved, as
  its rename wrote it, or at no rename as the ledger held it before the history. Undone in the order made, as an
  editor's undo goes, each step lands. Texts in which a rename is in effect without one it was planned after that
  moves related paths (an earlier rename undone by hand before a later one it led to), or in which a rename is held in
  some of the files it edited and gone back in others, are no state of the history: they show and land nothing,
  neither a move nor a stale line, until the texts are a state again; so no move lands that no rename made, and no
  write drops a harness value from the path the hand edit left (the ledger keeps it, as after any hand rename; with
  the files closed so, the history ends there, landing nothing, and the editor is told in its log which files hold
  each rename and which went back, and that `specforge rename` finishes it once the files agree). The shown overlay is
  what the buffers say (a file no buffer holds, as on disk), read again when a source buffer the history edits
  changes; once no file of it holds a buffer the disk does not (saved, or closed), what the disk says lands. A reading
  parses each declaring file once for every history it reads, reads a file no buffer holds from disk once (again only
  after an event names it), and reads a source file's names again only when its text changed. Carrying a place is
  bounded by counted work, never by a clock, and is linear in the texts' size with no size cut off: a pass over each
  text finds the anchor lines (the longest run of them in the same order in both), the lines shared at the edges of
  the stretch between two anchors are trimmed, and each stretch left holding a place is scanned once in each text for
  every place in it, so the work stays linear however many places a file holds. A line cut and pasted far away follows
  when it moved whole; edited as well, it reads as gone, never as another place. A history is kept while any of its
  files is open: a redo or an undo saved later lands the rest, as the ledger takes any later rename (a rename back
  carries each move record, as the CLI's does); with every file closed it is over. It is kept only while the ledger
  reads as the history last left it (as it found it, until a landing): a write the editor did not make (the CLI, MCP,
  another client, a pull) that changes what the ledger says of its entities (any path its renames moved) or of its
  stale lines ends it, so an outside rename is never read from the texts as an undo and landed over. The editor's own
  landing of one history is what every other history's ledger reads as next. So a history grows with the renames made
  while its files stay open, as an editor's undo stack does, and ends when they close; no node is pruned before, since
  an undo can return to any. A node keeps only what reading and landing need: the declaring file's text it was planned
  on, its moves, its marked lines and the source texts it read and wrote; the history keeps one project, the one the
  ledger was last written over. A rename answered and never applied waits while its declaring file is the text it was
  planned on, and is dropped at the first other text without its edit, so a name typed by hand later moves nothing.
  Rejected: landing when the declaring file alone is saved (the stale line would claim a source text the disk does not
  hold yet); ending the history at its first save (a redo would land nothing); whole-text comparison of source files
  (any other edit in a file would drop its lines); one tree per declaring file (a scope and what it scopes declared
  apart would each land against the last project, dropping the path between them and the harness values on it, and
  each claim the stale line both rewrote); and following a hand undo out of order by rebasing the later renames' moves
  (a guess at what the hand meant).
- The LSP rename's edit carries the source edits; the pending ledger change carries each stale line with the names
  it read and wrote and their places in the texts the rename read and wrote.
- **A delete of an entity a fact names stays refused.** It would remove what a test says it proves, and there is no
  text to delete in an attribute without guessing. Change the source, collect, then delete.

## D8. `names` is a fact-held reference

ADR 0329 D3's `names` ("the entities a test says it proves") is a fact-held reference: the rule of D4 links it. Its
declaration as a reference list and its proof diagnostic over the dangling names come with the change that declares
the item kind.

## Consequences

- A finding about a fact is placed where the fact is written, with a remedy addressed to its collector.
- A rename of an entity a test attribute names works from the editor and MCP, as often as needed before the next
  collect, and leaves the store stale until then.
- Every existing store is W192 until the next collect (pre-release).
- A stale store's places are exact in the text the renames left; W197 tells that the store is stale.

## What would reopen it

- A pack that cannot give a name's place: its names link and are judged, but their entities cannot be renamed.
- A reader that needs to tell a fact's place in a file the fact store no longer digests (a moved source tree).
