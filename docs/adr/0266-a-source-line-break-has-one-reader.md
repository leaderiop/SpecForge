# A source line break has one reader

**Status:** accepted (2026-10-26, architecture round 28, plan 03). Reopens [ADR 0247](0247-the-formatter-reads-a-values-text-from-the-parse.md) (a raw CR LF was a second fact of a value that format read apart from the parse). Amends [ADR 0021](0021-formatting-is-one-operation.md) D8 (its LF is formatting's), [ADR 0095](0095-the-format-header-is-read-and-rewritten-by-one-module.md) D2 (a new header ends in the file's line break) and [ADR 0235](0235-a-migration-lays-out-through-the-formatter.md) (a migration writes the file's line break around the blocks it lays out).

## Context

On a CRLF file the parse read a raw CR LF inside a `"..."` as CR LF (`string_text`), while a `"""..."""` (`dedent`, over `str::lines`) and the formatter (`lines_keeping_strings`) read it as LF. So `format`, which promises layout only, changed the stored text of a multi-line `"..."` value (a P0 by `formatting_semantic_preservation`), and every member edit of such a block was refused `undeclared_change`, "a defect of the typed edit". Nine places decided what a CR before a LF is, or which line break a written line gets:

| Site | Rule |
|---|---|
| `string_text` | CR kept (title, value, list item, annotation, verify text) |
| `dedent` | CR dropped (`str::lines`) |
| `header_line` | CR dropped |
| `lines_keeping_strings` | CR dropped, in field values only (titles, list items and verify texts are written whole: CR kept) |
| grammar, lexer | `\` + CR is an escape (before a LF there is none) |
| `with_format_version` | the first line's ending |
| typed edit, migration | LF, from `format_range` (ADR 0021 D8), in a CRLF file: a mixed file, and a diff naming every line of the block |

## Decisions

1. A line break is a LF, with one CR directly before it part of it (`str::lines`' rule). `specforge_parser::line_break::read` reads each CR LF as LF; a lone CR is text, and so is an escaped `\r`. The parse reads every `"..."` through it, before the escapes (`string_text`), so a CRLF file stores what its LF reading stores. `dedent` and `header_line` keep `str::lines`, the same rule by definition.
2. The formatter formats `read(source)`: every CR LF is LF, a title's, a list item's and a verify text's too, and its own CR strip is deleted.
3. `LineBreak::of(text)` answers which line break a writer writes: CRLF when the text holds a break and every one is CR LF, LF otherwise (none, all LF, or mixed). A writer that changes part of a file reads it with `lf` and writes it with `write`: the typed-edit planner (`Planner::text`, `finish`), the migration (`transform_content`) and the header writer. A CRLF file stays CRLF byte for byte outside what changes; a mixed file is written as before (LF where a writer changes it), because a mixed text cannot round-trip without a line diff in every writer. A new file is LF.
4. Whole-file `format` and the editor's document formatting keep writing LF, and so does the range format (ADR 0021 D8; VS Code keeps a document's own ending). Only the writers of part of a file write the file's line break.
5. The grammar's escape is `\` and any character but a CR or a LF, and the lexer agrees: a backslash before a CR LF leaves a string unclosed, as before a LF (E001, no new code). A backslash before a lone CR is unclosed too: no file writes one.

Rejected: normalising the source before tree-sitter (every byte offset after the first CR moves, so LSP positions and format edits of a CRLF document break); a line-break argument on `format_range` and every function that writes `\n` (about twelve sites, and the CR would sit inside `Block`'s lines); normalising a CRLF file to LF on every write (every line changes on the first edit, against `typed_edit_canonical_form`).

Breaking: a CRLF project's `"..."` values, titles, annotation values and verify texts read LF; a CRLF file with a backslash at a line's end inside a `"..."` reports E001; a typed edit and a migration write a CRLF file in CRLF.

Would reopen: a reader of `.spec` text that splits lines by its own rule; a writer of part of a file that does not read and write through `LineBreak`.
