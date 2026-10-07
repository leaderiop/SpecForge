# One reader of the language's lexemes and one CST vocabulary

**Status:** accepted (2026-10-07)

ADR 0023 made `specforge_parser::lex` the one lexer of the `.spec` language beside tree-sitter's
grammar, and cleanup round B kept the expression tokenizer apart on purpose. Four scanners still copied
parts of the grammar's lexical rules. The parser's recovery from unclosed strings had its own string,
comment, identifier and scheme-ref-ID readers, plus a hand copy of the top-level block shapes; the LSP
cursor re-derived whether a string was closed and re-implemented the ref-ID byte rule; the formatter
joined text onto one line with a string scanner that did not know comments. They disagreed, visibly: a
union block after an unclosed string was dropped from the graph; a `\` before a line break lost every
entity of the file; a valid string spanning lines was highlighted as fields, and hover and
go-to-definition answered from words inside it (the lexer ended every regular string at its line's end,
ADR 0023 D1); the formatter deleted comments written between a statement's tokens and turned a
multi-line import into a comment. Separately, the parser's AST walk and the formatter named tree-sitter
node kinds and fields with about 110 string literals and no shared definition.

## Decision

**D1. Strings are read as the grammar reads them.** `lex` reads `"…"` and `"""…"""` across lines.
`LexemeKind::Str { closed }` marks one the grammar would not close. For a `"…"` that is no closing quote,
a `\` before a line break (no escape, as the grammar's `seq("\\", /./)`), or, for one spanning lines, a
closing quote that runs straight into text, the pairing an unclosed quote shifts (`lex::runs_into_text`:
anything but whitespace, a bracket, `,`, `|`, `/` or the text's end). Such a string ends at its first
line's end, so an unclosed quote still never swallows a document being typed. A `"""…"""` is closed by
the first `"""`; with none, it runs to the end of the text. A test checks the lexer against the grammar
over the repository's spec, its integrations and examples, and a fixture of the forms they do not write.

**D2. The parser's recovery reads through the lexer.** The culprit is the first string the lexer marks
unclosed, or a multi-line `"""…"""` whose closing quotes run into text. It is ended before the next line
whose lexemes start a top-level form of the grammar: `use`, `pub use`, `spec "T" {`, `ref id "T"`,
`kind name ["T"] {` or `kind name =`. A test reads the grammar's own top-level nodes in the corpora and
requires each to start a resume line.

**D3. The LSP asks the lexer.** Whether a string is closed is the lexeme's, and whether a word is a
scheme ref ID being typed is `lex::is_ref_id_prefix`. The scheme-ref-ID byte rule exists once.

**D4. One CST vocabulary, in the grammar crate.** `tree_sitter_specforge::kind` (the 42 named node
kinds) and `tree_sitter_specforge::field` (the 20 field names) are the only spellings the parser and
the formatter use; a test checks them against the compiled grammar both ways. Anonymous nodes are their
own text; an error node is `Node::is_error()`.

**D5. A statement holding a comment is kept as written.** An import, an inline ref, a union block, a
field, a verify statement or a method that holds a comment between its tokens is not rebuilt on one
line: its first line takes the body's indentation and the rest stays as written. Other one-line
rebuilds join the lexer's lexemes with single spaces.

## Consequences

- A valid string spanning lines is one string in semantic tokens, and words inside it name no entity.
- A half-typed quote reads as before, except when the next quote in the text is followed by whitespace,
  a bracket, `,`, `|`, `/` or the end of the text (a following string starting with a space or a
  bracket): it then runs on to that quote, as the grammar reads it.
- Recovery keeps a union block after an unclosed string, and keeps the blocks after a string broken by
  `\` before a line break.
- A grammar change that adds, removes or renames a rule or field fails the vocabulary test, and one that
  adds a top-level form fails the recovery test, until the code follows.
- The formatter no longer drops or moves comments inside statements; such statements are not
  normalized.

## Rejected

- **A lexer mode per reader** (grammar strings for recovery, line-ended strings for the editor): two
  readings again, and the editor's misreading stays.
- **The grammar's strings with no fallback**: an unclosed quote being typed swallows the document.
- **Resume lines found by tree-sitter**: a header line alone never parses; ADR 0023 rejects reading
  half-typed structure through tree-sitter's error recovery.
- **A `build.rs` generating the vocabulary from `node-types.json`**, or **an enum of node kinds**: no
  more complete than constants checked against the compiled grammar, and heavier.
- **The lexer in the grammar crate**: saves one dependency edge (formatter → parser) at the cost of every
  reader's import path.

## What would reopen it

A spec in the repository that writes a string spanning lines whose closing quote is followed by `@` or
another character outside the follow set: the follow set grows, or the grammar gives strings a follow
rule. A grammar that makes a top-level form span lines before its first token that decides it: the
resume rule reads more than one line.
