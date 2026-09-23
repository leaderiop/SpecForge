# AGENTS.md

## Agent skills

Matt Pocock engineering skills are installed at `.agents/skills/` (project scope):
`to-spec`, `implement-spec`, `to-tickets`, `triage`, `grill-me`, `grilling`, `grill-with-docs`,
`domain-modeling`, `wayfinder`, `tdd`, `code-review`, `improve-codebase-architecture`,
`diagnosing-bugs`, `handoff`, `pr`, `retro`, and others.

Configuration (created by `/setup-matt-pocock-skills`):

- **Issue tracker**: GitHub Issues on `leaderiop/SpecForge` via the `gh` CLI — see `docs/agents/issue-tracker.md` (includes "Wayfinding operations" for `/wayfinder`)
- **Triage labels**: default vocabulary (`needs-triage`, `needs-info`, `ready-for-agent`, `ready-for-human`, `wontfix`) — see `docs/agents/triage-labels.md`
- **Domain docs**: single-context layout (`CONTEXT.md` + `docs/adr/` at repo root, created lazily by `/domain-modeling`) — see `docs/agents/domain.md`

Run engineering skills before assuming workflow; they read this block and the files above.
