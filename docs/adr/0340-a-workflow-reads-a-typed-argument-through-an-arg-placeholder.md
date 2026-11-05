# A workflow reads a typed argument through an `{{arg:}}` placeholder

**Status:** accepted (2026-11-05, S1 round). Extends
[ADR 0314](0314-a-prompt-argument-may-be-a-tail-a-picked-subject-or-a-host-name.md) (the typed arguments of a prompt).
No protocol change: the render is `@gsd/*`'s.

## Context

- The host parses a prompt's words with the real parser and refuses an option the prompt does not declare.
  `gsd-kit` then turned the typed values back into text, spliced it for `$ARGUMENTS`, and 64 quoted heredocs in 37
  ported files captured it so `sed`, `grep`, `awk`, `xargs` and `[[ =~ ]]` could parse it a second time.
- That second parser held the defects: `pr-branch`'s `--target` ran as shell (`echo "$ARGUMENTS"` around a value in
  double quotes that escape no `$`), `--ws "alpha"` matched no regex, `(no arguments)` read as a milestone name,
  `xargs` failed on an apostrophe, a quick-batch "parse" copied its text to a file and back, research-only mode never
  turned on, and dispatches forwarded options their target prompt refuses.
- The capture heredocs were also the only reason for the per-render terminator stamp (#5637).

## D1. The grammar

`{{arg:<name>|<fallback>}}`. `<name>` is an argument the prompt declares (snake_case). `<fallback>` is literal author
text up to the closing `}}`; it may hold shell (`{{arg:milestone|$MILESTONE}}`), since it is template, never typed
text.

## D2. The value by kind

| kind | value |
|---|---|
| reference, subject, optional reference | the entity's full path (`v1.0/02`) |
| bound | the last segment, as the person wrote it (`02`) |
| flag | `true` / `false` |
| integer | decimal |
| text, one-of, rest | the string; an empty string counts as absent |

## D3. One scan, after the host conversion

The fill runs where `$ARGUMENTS` was substituted: after the re-pointing and the host conversion. One left-to-right
scan of the command file and the files it inlines replaces the earliest of `{{setting:`, `$ARGUMENTS` and `{{arg:`,
and never rescans what it wrote, so a typed or set `$ARGUMENTS` or `{{arg:x|y}}` stays data. The preamble (which
shows the settings block) and the prompt's tail are not scanned. A declared name with a value writes it escaped as a setting is (`"`, `\`, `$` and the
backtick). A declared name without one writes the fallback.

## D4. Where a placeholder stands

Only in a file a prompt inlines, and only inside double quotes in a `bash`/`sh` fence. An undeclared name leaves the
token as written (the render stays infallible); a test over every render fails on it.

## D5. A file read through `reference`

`specforge gsd reference` fills settings, not arguments. A file a prompt does not inline states a flag condition in
prose about the run's `<arguments>`, or reads a variable an inlined fence binds and the prose names.

## D6. No fence reads the argument text

No shell fence reads `$ARGUMENTS` or `$GSD_ARGS`. `$ARGUMENTS` stays in prose, as data the agent reads. The argument
heredocs and the #5637 terminator stamp are deleted.

## D7. A dispatch forwards only declared options

A `Skill(…, args=…)` dispatch forwards only options its target prompt declares. `plan_review_convergence` declares
`override_gate`: upstream reads it (#4600), and autonomous `--converge` cannot run without it. Upstream flags a prompt
does not declare (`--no-transition`, `--auto` on `execute_phase`, reviewer lanes on `autonomous`) are dropped from
the text, not declared.

## D8. `ws`

`ws` stays declared on the three prompts whose upstream hint has it (`new_milestone`, `plan_review_convergence`,
`verify_work`) and is forwarded nowhere.

## Rejected

- **B, a runtime parse** (`specforge prompt --parse`): about 130 more processes for a value the render already holds.
- **C, a second render** of the prompt with the parsed values.
- **Escaping `$` in the argument text alone**: closes the injection, keeps the second parser and its defects.
- **Declaring upstream's undeclared flags**: the declared tables match upstream's hints.

## Consequences

- Typed text never reaches a shell as code: each placeholder sits in double quotes and its value is escaped.
- About 240 ledger rows leave with no kernel change.
- An agent that reads an inlined workflow again through `reference` sees the placeholder unfilled; the rendered prompt
  it was given holds the filled text.
