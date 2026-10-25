# An extension's text is read printable where the host reads it

**Status:** accepted (2026-10-25, architecture round 27, plan 02). Extends [ADR 0231](0231-a-search-hit-is-a-package-name-at-the-wire.md) D4 (its "what would reopen it": extension-supplied text gets the same function at its own entry points) and [ADR 0037](0037-the-sandbox-is-limits-the-host-enforces.md) D1.

## Context

- The sandbox grants a component no capability (ADR 0037 D1), but the terminal becomes one: an extension's command title
  or argument help reached `specforge greet hello --help` raw on a terminal, and `ESC ] 52 ; c ; … BEL` sets the user's
  clipboard. A rule's `message_template` reached `check` raw even in a pipe, a pass's message reached `analyze`, E030
  quoted a declared short name, E028 a guest's error text, and JSON and MCP carried the bidirectional and C1 controls
  raw.
- Every sink printed what it was given; one `printable` call per sink (clap ×3, ariadne, E030, W138, W153, LSP, MCP,
  verdicts, E028, the lock writer) was the per-site pattern three rounds had already fixed one entry point at a time.

```text
BEFORE  guest answer -> from_slice::<O> -> text as sent -> clap / ariadne / LSP / MCP / JSON
AFTER   guest answer -> reads_text? -> Value -> remedy::hold_printable -> O -> the same sinks, printable
        registry manifest -> hold_printable -> declaration
```

## Decisions

- **D1.** `ExtensionCalls` (ADR 0013), where an extension's bytes become typed answers, reads a handshake, describe,
  pass or validator answer through `remedy::hold_printable`: every string and object key written printable, before the
  typed decode, before `Sandbox::of` and before W138's key walk. A registry's manifest is held the same way in
  `read_declaration`, so the peers `add` records equal the loaded binary's (`printable` is idempotent).
- **D2.** A handshake's `starter_template` is a file's text and keeps its newlines and tabs (`remedy::DECLARED_LINES`);
  every other control is escaped in it.
- **D3.** A command's stdout and stderr, an MCP tool's and resource's answer, a collector's and a scanner's answer are
  read as answered: the first three are the output the user or agent asked for, the last two are data matched against
  the project's own text (`Operation::reads_text`).
- **D4.** A call failure's text (a trap message, a decode reason) is written printable whole, newlines included, in E028.
- **D5.** A declared text holding a control or bidirectional character is escaped and the extension loads; no new
  diagnostic code.

## Rejected

- A check in `ExtensionDeclaration::from_wire` (a blob input, misses W153, pass, verdict and trap text); a sink rule in
  the CLI's terminal writer (CLI only, cannot tell a file's bytes from a message); W163 on every escaped load; refusing
  the extension; holding a command's output; per-sink `printable` calls.

## Consequences

- Hostile text shows as `\u{1b}` in help, diagnostics, listings, JSON and MCP; no builtin declares one, so no snapshot
  changes and no re-vendor.
- A declared identifier holding a control character registers as its escape text and can match no `.spec` keyword.
- An installed extension whose peer text held a control character differs from its older lock entry: E070, reinstall.

## What would reopen it

- A protocol carrying rich text (Markdown descriptions).
- A command's output reaching a terminal raw judged a channel to close (a terminal-only write of `printable`).
- A project's own `.spec` text holding a control character (it enters at the parse, a follow-up of its own).
