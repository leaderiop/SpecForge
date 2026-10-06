# The LSP reads a document through one module

**Status:** accepted (2026-10-06)

The LSP answered questions about a `.spec` buffer with seven scanners of its own: a bracket scanner for
completion that counted `[` inside strings, a brace scanner that respected strings but not `"""`, a
header guesser that skipped `requires`/`ensures`/`maintains` by name, a 330-line line classifier for
semantic tokens that ended an entity at the `}` of its first nested block, an ASCII word grabber for
hover, a second one for go-to-definition's bounds, and a `use`-line splitter. Navigation (ADR 0016) had a
string- and comment-aware lexer, private to ops, that split scheme ref IDs. Byte columns and UTF-16
columns were converted in eleven places: four functions (two of them twins), a span wrapper, a
pass-through that converted nothing, and five inline loops; three commits each fixed "UTF-16 at every site" (49985278,
833d815c, f85bc381). The consequences were visible: hover named one entity and go-to-definition jumped
to another on `gh.issue:42`; a `[` inside a string turned field completion into entity completion;
highlighting stopped after `requires { … }`; completion offered `define`, which ADR 0005 removed; field
completion code that tests proved was not the code production ran.

## Decision

**D1. One lexer, in the parser crate.** `specforge_parser::lex` reads identifiers, scheme ref IDs (one
lexeme), numbers, strings, comments and punctuation, without a parse, so half-typed text lexes. A test
checks it against tree-sitter on every spec file of the repository. Navigation's `SourceText` and the
LSP's document read text through it; neither scans text itself. A regular string ends at its line's end
(the grammar lets it run on; the repository has none), so an unclosed quote never swallows a document
being typed.

**D2. One document module in the LSP.** `specforge_lsp::document::Document` owns an open buffer, its
version, its `LineIndex` and its syntax (lexemes with roles, and the entity bodies, blocks and lists they
open). Its interface: `at(position) -> Cursor`, `tokens(view)`, `index()`, `apply_change`. It lives in
the LSP, not in ops: UTF-16 is the protocol's unit and no MCP tool asks about a position (ADR 0016's
"editor-only navigation need").

**D3. One line index.** `LineIndex` is the only conversion between byte offsets and UTF-16 positions,
both ways. A span of the graph or of a diagnostic is a position in the text the project was compiled
from (`ProjectSession::source_text`, the text the compile parsed: the resolver keeps it, ADR 0018 D5), so
it converts against that text through a per-request cache of indexes, reusing an open document's own index
when the document is that text; never against the buffer typed since, nor the disk now. Navigation reads
the same text. A file the compile holds no text of has no range: its location or symbol is left out, a
diagnostic stays at its file's start as one without a span does, a code action with such an edit, or over
a buffer typed since the compile, is not offered, and a rename over a stale buffer is refused as
`ContentModified` (-32801). There is no byte-column fallback.

**D4. Structure from the text, identity from navigation.** The graph lags the buffer while the user
types, and loses a block the parser rejects, so where the cursor is (entity body, field, list, string,
comment) is read from the buffer's syntax. Which entity a token names is navigation's occurrence, which
checks the token against the same text.

**D5. One entity per cursor.** `Cursor::target` answers, in order: the `use` statement's path; the
declaration or reference token under the cursor; an identifier or scheme ref ID at a reference
position (a header's name, a value or item of a field the registry does not type as a non-reference,
a `use` binding's imported name) that names an entity; the field whose name the cursor is on. Hover,
definition, references and rename ask it; prepareRename asks only for the token. Words in strings and
comments, and enum, boolean, string and number values, name nothing. The same reference positions are
what completion fills with entity IDs and what semantic tokens mark as references.

**D6. Completion sites.** `Cursor::completion` says what completes: keywords at a top-level statement
start (`use` and the registered kinds; never `define`), the kind's fields at a statement start in its
body, the kind's allowed verify kinds after `verify`, entity IDs in a reference list or a
single-reference field's value, an enum field's declared values, `true`/`false` for a boolean field,
nothing in strings, comments, nested blocks, define blocks and other values. Every item carries an
edit over the word under the cursor (the lexer's word, a scheme ref ID whole), an insert/replace edit
when the client supports one, so the client's own word rules never decide what is replaced.

## Consequences

- Hover names a scheme ref's ref; hover and definition no longer answer from comments and strings; field
  help answers only on a field's name.
- A ref's definition is a token (`precision: "token"` in MCP `find_definition`; the ID selected in the
  editor; prepareRename answers it).
- Completion stops offering `define`, field names inside strings, entity IDs inside nested blocks and
  string lists, and `"` as a trigger; it completes a single-reference field's value, enum and boolean
  values and verify kinds, and accepting an item replaces the whole word (a scheme ref ID included).
- Semantic tokens classify every field of a body, trailing comments, `pub`/`from`/`as`/`method`/`expr`
  as keywords, method names, strings and numbers wherever they are, enum values as `enumMember` and
  boolean values as keywords; a reference takes the token type of the kind of the entity it names; a
  define block's name has no token and its W143 is published as unnecessary code (faded).
- A `use` binding's imported name navigates to its entity; the rest of the statement to the file.
- Diagnostic publication is a pure function of the LSP state (`publish::Publication::of`), tested without
  a client.

## Rejected

- **Structure from graph spans** (node spans, `FieldEntry::value_span`): stale while typing, absent for
  text the parser rejects, and fields have no key span.
- **Reparsing the buffer with tree-sitter per request**: its error recovery on half-typed text decides
  the structure unpredictably.
- **The document module in ops**: it would carry the LSP's protocol unit into the operations crate.
- **Keeping words in prose as entity names**: rename already refuses them; the answers disagreed.

## What would reopen it

A client that negotiates UTF-8 or UTF-32 positions (`general.positionEncodings`): the line index grows
an encoding, nothing else changes. A grammar change that the agreement test rejects: the lexer follows
the grammar.
