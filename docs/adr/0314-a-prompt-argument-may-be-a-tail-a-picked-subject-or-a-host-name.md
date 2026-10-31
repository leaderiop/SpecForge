# A prompt argument may be a free-text tail, a picked subject, or named like a host option

**Status:** accepted (2026-10-31, kernel round K20b, plan 14). Extends [ADR 0291](0291-an-extension-contributes-mcp-prompts.md). Takes extension protocol 1.13.0.

## Context

Upstream commands take a bare task description, `[path]`, and an optional phase. A prompt argument was named, quoted, never `path` or `format`, and a subject was required.

## Decision

- **D1. `rest`.** `PromptArgDescriptor.rest: bool`. One string arg per prompt, never the subject. A word no positional slot takes goes to it, joined by one space. Given by name and by words is "given twice". An unknown `--option` is refused. After a `--` in the words the prompt reads, every word is a tail word.
- **D2. `path`, `format`.** Reserved prompt names are `help`, `settings`, `secrets`, `host`, `argv`. Two boundaries on the command line (`specforge prompt …` and `specforge <ext> <prompt> …`). The first `--` is the host's: `--path`, `--format` and `--host` are read before it only, and it is removed. Everything after it is the prompt's input and can never set a host option, so an installed skill's line `specforge prompt gsd.quick --host claude-code -- <text>` is safe whatever the text. A second `--` is the prompt's own boundary, after which every word is plain text (`-- fix -- --dry-run` has the task `fix --dry-run`). A prompt's flags before the second `--` still work (`-- --full fix`), and a prompt's own `path` or `format` argument is given after the first `--` (`-- docs/adr`). A host option in the text without the second `--` is the prompt's to refuse ("no such option").
- **D3. Optional subject.** An optional subject is positional. When none is given the host calls the export first; the answer may carry `subject` (`PromptOutput::with_subject`), which must be the path of an entity of the subject's kinds (else E028). The gate then runs for the picked subject. A given subject wins.

## Rejected

An implicit `$ARGUMENTS` arg; renaming gsd's args; the host picks the subject.

## Limits

A `--path` before the first `--` always names the project, from any current directory; a prompt that declares `path` takes it after the `--`. A free-text tail is rendered where its argument stands in the prompt's table (quick's first), so a subcommand word still opens `$ARGUMENTS`. An optional subject and a tail together: the first word goes to the subject. The export runs before the gate when the subject is absent; prompts are read-only (ADR 0291).

## Consequences

Protocol 1.13.0.
