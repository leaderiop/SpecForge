# A prompt argument may be a free-text tail, a picked subject, or named like a host option

**Status:** accepted (2026-10-31, kernel round K20b, plan 14). Extends [ADR 0291](0291-an-extension-contributes-mcp-prompts.md). Takes extension protocol 1.13.0.

## Context

Upstream commands take a bare task description, `[path]`, and an optional phase. A prompt argument was named, quoted, never `path` or `format`, and a subject was required.

## Decision

- **D1. `rest`.** `PromptArgDescriptor.rest: bool`. One string arg per prompt, never the subject. A word no positional slot takes goes to it, joined by one space. Given by name and by words is "given twice". An unknown `--option` is refused. After `--` every word is a tail word.
- **D2. `path`, `format`.** Reserved prompt names are `help`, `settings`, `secrets`, `host`, `argv`. The host reads `--path`, `--format` and `--host` only before the first `--`, and leaves `--path`/`--format` to a prompt that declares them.
- **D3. Optional subject.** An optional subject is positional. When none is given the host calls the export first; the answer may carry `subject` (`PromptOutput::with_subject`), which must be the path of an entity of the subject's kinds (else E028). The gate then runs for the picked subject. A given subject wins.

## Rejected

An implicit `$ARGUMENTS` arg; renaming gsd's args; the host picks the subject.

## Limits

An optional subject and a tail together: the first word goes to the subject. The export runs before the gate when the subject is absent; prompts are read-only (ADR 0291).

## Consequences

Protocol 1.13.0.
