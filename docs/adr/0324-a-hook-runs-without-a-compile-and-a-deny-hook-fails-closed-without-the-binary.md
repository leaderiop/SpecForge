# A hook runs without a compile and a deny hook fails closed without the binary

**Status:** accepted (2026-10-31, kernel round K20c, plan 15). Extends [ADR
0292](0292-an-extension-contributes-agents-skills-commands-and-hooks.md). Takes extension protocol 1.14.0.

## Context

`specforge hook` compiled the whole project for every event, though `hook::run` reads only the registries, settings,
state and, when declared, the graph and revisions. And a line `specforge hook …` exits 127 when `specforge` is not on
the host's PATH; all three hosts treat any non-zero exit but 2 as a non-blocking failure, so the spec guard was
silently skipped.

## Decision

- **D-K20c-8.** `specforge hook` loads the environment, looks the route up in its registries, and builds the view over
  `CompiledProject::unread` when the hook declares neither `reads_graph` nor `writes` and is not `core.spec_guard`;
  otherwise it compiles in that same load. An unreadable event uses `unread` too.
- **D-K20c-9.** A rendered hook whose policy is `deny` (`on_failure: deny`, and `core.spec_guard`) on `pre_tool`,
  `pre_commit` or `user_prompt` fails closed when `specforge` is missing, rendered per host because the hosts do not
  all run `sh`:
  - **Claude Code:** the plain line, and `"onFailure": "block"` on the handler (shell-neutral; it also covers a crash
    and a timeout).
  - **Codex:** `command` is `command -v specforge >/dev/null 2>&1 || { echo '…' >&2; exit 2; }; specforge hook …`,
    and `commandWindows` is the plain line.
  - **Cursor:** the same guarded line, and the plain line when install runs on Windows (PowerShell has no `||` or
    `command`).
  - A hook whose policy is `allow`, and any `stop`, `subagent_stop` or other event, keeps the plain line and fails
    open: exit 2 on a stop event keeps the agent working, and Claude Code ignores `onFailure` there.
  - Install recognises the guarded and the plain line as its own and removes either, so an old install is replaced.
- **D-K20c-9a.** `specforge hook` never exits 2 on its own refusals (an unknown host or event, a usage error): it
  exits 1, which every host reads as a non-blocking failure. Exit 2 is the hook's own deny.

## Limits

- A `specforge` on the host's PATH older than the install may exit 2 (clap's usage error) and block; an old binary
  cannot be fixed.
- Git Bash and PowerShell on Claude Code, and fish as a login shell on Cursor or Codex, never see the sh line; the
  Windows rows above cover PowerShell, and a fish user has the plain line's fail-open behaviour at worst.
- A Cursor install made on Unix and run on Windows keeps the sh line; re-install there.

## Rejected

Refusing install without `specforge` on PATH (install cannot know the host's PATH); the binary's absolute path
(goldens become machine-specific); Cursor's `failClosed: true`.

## Consequences

Protocol 1.14.0 is unaffected (no wire change). The guarded line is the only golden change for deny hooks.
