# A project's own text is read printable at the parse

**Status:** accepted (2026-10-26, architecture round 28, plan 13). Extends [ADR 0244](0244-an-extensions-text-is-read-printable-where-the-host-reads-it.md) (answers its third "what would reopen it": a project's own `.spec` text) and [ADR 0231](0231-a-search-hit-is-a-package-name-at-the-wire.md) D4 (the term covers the project's text, at the parse). Amends [ADR 0266](0266-a-source-line-break-has-one-reader.md) D1 (a lone CR is no longer text).

## Context

- A `.spec` file was the one text SpecForge prints that no check held printable. An ESC, BEL, C1 or bidirectional control in a title, a value, a comment or between tokens reached the terminal raw through eleven sinks: the E061, E001 and E019 messages, the echoed source line, markdown and DOT export, JSON (serde escapes C0 only, so C1 and bidirectional controls were raw), MCP, LSP diagnostics and hover, `format` writing it back, and `set` writing one and printing its diff.
- A per-sink `printable` call is what ADR 0244 rejected: eleven calls, CI never notices a missed one, JSON needs a custom serializer, and `format` and `set` still write the character.
- No `.spec` string escape spells a code point, so a value could hold such a character only raw. None of the 237 tracked `.spec` files does.

```text
BEFORE  .spec ─▶ Cst::read ─▶ AST (raw) ─▶ 11 sinks, each raw
AFTER   .spec ─▶ Cst::read ─ unprintable_in_lines? ─▶ one E001 at it, no tree
        diagnostic echo ─▶ remedy::printable_lines (the refusal's own line shows \u{1b})
```

## Decisions

1. The parse refuses the file. `Cst::read` checks first (`CstError::Unprintable`, as it refuses brackets nested too deep): the file's one E001 at the first such character, naming its code point, no tree, no entity, no header check. The formatter reads through the same `Cst::read`, so `format` leaves the file as written; a typed edit probes its value through the parse, so `set` of such a value is refused (`not_editable`) and writes nothing.
2. One rule for a file's text, `specforge_common::remedy` (`unprintable_in_lines`, `printable_lines`, `printable_lines_at`): a tab and a LF are kept, and a CR only directly before a LF (a line break, as `line_break::read` reads it). A lone CR is a control character and is refused; an escaped `\r` in a quoted text is not a raw character and stays a CR. A starter template's CR LF is kept and its lone CR escaped.
3. The echo is printable. `render_diagnostics` hands ariadne `printable_lines` of each source and moves the byte range with `printable_lines_at`; the refusal's line shows `\u{1b}`. Messages are not re-escaped: after the parse refuses, none holds such project text.
4. E001, no new code. A U+FEFF and a U+00A0 are not in the list and stay legal.

Rejected: escaping at every sink; reporting each character at its span while parsing on, with every AST text held printable (a missed entry is a leak, and the formatter writes the raw token back); a `\u{..}` escape in `.spec` strings (a grammar change, a feature of its own).

## Consequences

- A `.spec` holding such a character fails `check` (exit 1) with the E001 naming line, column and code point; `migrate`, `format` and typed edits leave it as written. Its entities are gone until the character is deleted, so references to them are E003 elsewhere (as for a file nested too deep). Pre-release: no shim.
- Breaking: `specforge_parser::CstError` gains `Unprintable`; `specforge_common::remedy` gains three functions; the corpus check counts a file read into no tree as broken.

## What would reopen it

- A `.spec` string escape for code points (it would let a value hold one without writing it raw).
- A right-to-left project needing U+200E or U+200F in a title.
