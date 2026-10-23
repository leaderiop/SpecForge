# A lock source is checked once, and a remedy's command quotes its values

**Status:** accepted (2026-10-23, architecture round 26, plan 01). Extends [ADR 0036](0036-one-package-name.md) D1 and
[ADR 0135](0135-a-remedy-names-a-command-or-the-requests-words-never-a-flag.md) D1-D2.

## Context

- A lock entry's `local:` source was any text, and the reinstall command pasted it: a cloned repository's specforge.lock
  wrote `specforge add tool.wasm && curl … | sh #` into the E028, E070 and W149 suggestions and doctor's remediation,
  which check, doctor, specforge.validate and specforge://diagnostics hand to a user or an agent as the fix. A newline
  forged a second help line; ESC reached the terminal.
- 26 remedies named a command with a pasted value; four values come from a repository or an extension (the recorded
  path, a .wasm file entry, a registry alias, a declared peer). A real path with a space, a leading `-` or `@` gave a
  command that does not run.

```text
BEFORE  "source": text ─► Local(any) | Other(any)        format!("specforge add {specifier}")  ×26 sites, no rule
AFTER   "source": text ─► Registry | Local(LocalPath)    else E033      remedy::command(template, values)  one writer
```

## Decisions

- **D1.** A lock entry's source is read where the lock is read: `registry` or `local:<path>`, the path not empty and
  holding no control character (`LocalPath`). Anything else makes the lock unreadable (E033, its one remedy), the text
  shown escaped. `add` refuses a local file whose path the lock could not record (E054) before anything is written.
- **D2.** Spaces, shell metacharacters and absolute paths are a path's own: kept, and quoted where a command names them.
- **D3.** One writer, `specforge_common::remedy::command(template, values)`: the template as written, each value one
  POSIX shell word (bare when only `A-Za-z0-9@%+=:,./_-`, else single-quoted, `'` as `'\''`), a control character as
  its escape (`remedy::printable`).
- **D4.** A reinstall names a relative recorded path `./<path>`, so `add` reads it as a path whatever its first character.
- **D5.** ADR 0135's guard also refuses a literal that writes a value into a `specforge <subcommand>` clause below the
  surfaces; such a command is a `remedy::command` template.

## Rejected

- Refusing metacharacters, spaces or absolute paths at the read: `add` writes them for real files.
- Quoting at each surface: the CLI and MCP read one suggestion text.
- A per-entry refusal: an unreadable lock is the whole lock's (ADR 0036's name precedent, one remedy).
- A `shlex`/`shell-words` dependency; a `--` before values; a new code (E082).

## Consequences

- An existing lock whose source is unknown, empty or holds a control character: E033 on check, doctor, the LSP and MCP
  until it is fixed or deleted and the extensions added again. One whose path holds a space or metacharacter loads as
  before; its suggestion is quoted.
- Every local reinstall suggestion reads `specforge add ./<path>` (was `<path>`); a safe value is written as before.
- The quoting is POSIX (sh, bash, zsh; PowerShell reads `'…'` literally but doubles `'`); Windows cmd is not served.

## What would reopen it

- A surface that must print commands for a shell POSIX quoting does not serve (Windows cmd): a rendering per surface,
  one writer still.
- A text an extension writes itself (a pass diagnostic's suggestion) that names a command: the host cannot quote prose;
  that is attribution, not this writer.
