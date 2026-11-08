# tree-sitter is the host's one source reader

**Status:** accepted (2026-11-08, R2 of the Beyond Parity build plan). **Amends**
[ADR 0293](0293-a-pass-or-command-declares-the-project-files-it-reads.md) D4: its 64 KiB text cap stays for a `files`
input only; source the host parses is read whole. The guest face of the reader is ADR 0345.

## Context

- No part of SpecForge parses source. `@specforge/rust` and `@specforge/typescript` find items with string logic (a
  function inside a raw string is "public"), feeding `specforge infer gaps`; a pack that wants to name a source item,
  where it starts and what encloses it has nothing to ask.
- The only host read of source a guest gets is the `files` input: 64 KiB of text per file, 1 MiB per call, 2,000
  files (ADR 0293 D4). SpecForge's own tree has 2,461 `.rs` files, 22.5 MB.
- tree-sitter 0.26 is already a workspace dependency: its C runtime is compiled into every binary for the `.spec`
  grammar (`crates/kernel/tree-sitter`), and the workspace lints name its FFI as production's one `unsafe`.
- Bench (release, one thread, 2026-11-08): SpecForge's Rust, 2,461 files / 22.5 MB, parsed in 1.31 s with 0 error
  nodes, 36,856 items; with a tags query run over every tree, 2.19 s. A broken file (`fn third( { let x = ; }` among
  four good items) recovers all 5 items with 1 error. TS/JS: 77 files in 90 ms; a broken TS file recovers 3 of 4.
  Binary: the runtime is linked already, the rust grammar adds about 1.1 MB, typescript and tsx 2.9 MB.

## D1. One reader, in the kernel

`crates/kernel/source` (`specforge-source`) is the host's one source reader: tree-sitter with the grammars
`tree-sitter-rust@0.24.2`, `tree-sitter-typescript@0.23.2` (its `typescript` and `tsx` languages), pinned exactly in the
crate's own manifest (not in `[workspace.dependencies]`, which every blob inherits); a test holds the names it reports
to `Cargo.lock`. JavaScript is read with `typescript`, JSX with `tsx`. The crate knows grammars, not items: what a
"function" or an "item id" is lives in the packs' queries and code.

A pack hands it a **query** in the tags convention:

- every pattern has exactly one **role capture**, `definition.<kind>` or `reference.<kind>` (`<kind>` a word with `_`
  or `.`);
- every other capture not starting with `_` is answered with its text and span (`@name`, `@doc`, `@path`, `@test`,
  `@visibility`, …: the pack names them), anonymous nodes included (`"export" @attr`);
- the text predicates tree-sitter evaluates itself are honoured (`#eq?`, `#not-eq?`, `#match?`, `#not-match?`,
  `#any-of?`, `#not-any-of?` and their `any-` forms); a general predicate (`#set!`, `#strip!`, `#select-adjacent!`,
  any `…!`) or an unknown one is a fault, as is a pattern with no role capture or two. A fault names tree-sitter's
  row and column, or the convention's.

The answer per file is its matches sorted by (start byte, end byte descending), each with its role, kind, span, the
index of its **parent** (the innermost definition match whose span holds it, strictly, or with the same span and an
earlier pattern) and its captures; capture text is UTF-8 (lossy), cut at 4,096 bytes on a character boundary and
flagged `cut`. `errors` counts the tree's ERROR and MISSING nodes: the parse is error tolerant and what it recovered
is answered. A span is `{start, end}` of `{byte, line, col, byte_col}`: 0-based byte offset, 1-based line, 1-based
column in characters (SpecForge's convention for a person) and 1-based column in bytes (what a fact location stores).

## D2. A content-addressed cache, a parallel read, counted work

A parse is cached under `.specforge/cache/syntax/` (`<aa>/<hex>.json`, `hex = sha256(format, grammar, query digest,
file sha256)`), written atomically; a hit touches the entry. An entry untouched for 30 days is pruned at the end of a
collect, the time passed in. An unreadable entry is a miss and is rewritten; an outcome that did not parse is never
cached; a write that fails is counted (`unwritten`), never an error, and the answer is the same. Files are loaded,
digested, looked up and parsed on scoped threads (one parser per worker and language, at most one worker per file);
the answer keeps the order asked. Every read counts its work: `parsed`, `cached`, `bytes` parsed, `unwritten`.

## D3. A deadline per file, no size cap

Each file has a parse deadline, 5 s of wall clock in production, bounding its parse and its query run; tests use a
deadline counted in tree-sitter's progress callbacks, so no test reads a clock. A file past it is answered
`not_parsed(deadline)`. There is no size cap of the reader's own: a file over the fs adapter's 64 MiB ceiling
(ADR 0264) is `not_parsed(too_large)`.

## D4. The C-only-where-needed rule holds

The build plan's rule (S-D1) admits C only where no pure-Rust crate does the job. It holds here: the C runtime is
already compiled into every binary for `.spec`; each grammar is generated C tables plus one scanner; and no pure-Rust
crate is one error-tolerant reader with one query language across Rust and TypeScript.

| Reader | Languages | Broken Rust file (5 items, 1 error) | Notes |
|---|---|---|---|
| tree-sitter 0.26 | Rust, TS, TSX (JS read as TS) | 5 items, 1 error | one query language; C runtime already linked |
| ra_ap_syntax 0.0.360 | Rust only | 6 items, 4 errors | a weekly 0.0.x release; needed a `unicode-properties` pin |
| oxc_parser 0.157 | TS/JS only | (broken TS: 0 statements) | drops the file on one error |
| syn 2 | Rust only | 0 items | not error tolerant |

## Rejected

- **ra_ap_syntax for Rust and oxc for TS.** Two readers, two tree models, no shared query; one churns weekly, the
  other answers nothing for a file with one error.
- **tree-sitter-tags.** It answers a tag's name, doc and role only: no custom capture and no parent, and a pack needs
  both.
- **A per-file size cap.** A cap answers a large file with nothing for no reason the deadline does not already give;
  the deadline bounds the work and W196 says which files it stopped.
- **A guest-side parser.** Every pack would ship a grammar in its blob (megabytes each) and parse inside the fuel
  budget with no cache shared across packs.
