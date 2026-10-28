# An extension contributes MCP prompts, and a command or prompt may be gated

**Status:** accepted (2026-10-28, architecture round 33, plan K8). Extends [ADR 0008](0008-extension-commands-run-over-the-graph.md) and
[ADR 0011](0011-extension-command-contract-and-the-product-surface.md) (a command may be gated),
[ADR 0012](0012-one-extension-declaration.md) (a surface item kind), [ADR 0013](0013-typed-extension-calls.md) (a typed
call), [ADR 0017](0017-one-extension-surface-table.md) and [ADR 0024](0024-one-mcp-request-pipeline.md) (the prompts
adapter serves extension prompts), [ADR 0058](0058-every-mcp-message-is-a-spec-type.md) (`specforge.prompt`'s reply is
a spec type) and [ADR 0097](0097-an-extension-commands-name-is-judged-by-its-route.md) (prompts share the commands'
routes). Takes extension protocol 1.10.0.

## Context

MCP prompts were core-only. GSD's orchestration is 89 top-level workflow prompts (181 files with their fragments,
gsd-core v1.18.0) the host agent follows: spawn the researcher, then the planner, loop the checker up to three times,
then commit. For any agent to run GSD through SpecForge those prompts must be served by the extension that owns them.
MCP prompts are user-controlled: a client lists them as slash commands and no tool reaches them, so an agent mid-workflow
needs another way in.

No command had preconditions: a command ran its export unconditionally.

## Decision

- **D1. An extension may declare MCP prompts**, served as `specforge.<short>.<id>`, with typed arguments (field forms,
  never a path); the guest renders the messages through a `prompt__<id>` export. Prompts share the commands' routes,
  `(short name, id with _ spelled -)`: commands are routed first, then prompts in load order, and a prompt whose name is
  taken is E039 and runs on no surface. The core prompts keep their URI names (`specforge://prompts/<name>`). A prompt
  whose `when` setting is false is not served. The caller's words are read by each argument's field type and admitted by
  the form every field of that type is admitted by; a reference is resolved by `Graph::find` and the export is sent its
  full path.
- **D2. A command, and a prompt's entry, may declare gates**: local codes of the extension that must not be errors in
  the scope of a `subject` argument's entity (the subject and what it scopes; the project with no subject), and off when
  a setting gates them off. The host evaluates them over the project's report (the settings' raises applied) and the
  guard passes that declare the code in `reports`, asked once with a `trigger` saying why (`gate`, the command or
  prompt, the subject), and refuses the call `gate_failed` (kind `precondition_failed`, `data.gates` and
  `data.failing`) before the export runs. Enforcement lives at the command call, at the prompt's entry and, for a write,
  in the judge of ADR 0289. Prompt text itself is never enforced: an agent may ignore a step, and a step that only
  spawns an agent is checked when that agent writes through a command.
- **D3. Prompt text is the extension's own source**, not project state.
- **D4. One render behind four doors**: MCP `prompts/get`, the core tool `specforge.prompt {name, arguments?, argv?,
  host?}` (so an agent can fetch a workflow), the CLI `specforge prompt <short>.<id>` and `specforge <short> <id>` when
  no command of the extension has the name. The host appends one message after the guest's: the prompt as
  `@ext@version:id`, a digest of the guest's texts, the attribution, the subject and each step with the tool, command
  line and state of its gate now. The host the prompt renders for is `clientInfo.name` of `initialize`, or `--host` and
  `host`.
- **D5. Versioning and attribution.** A prompt's version is its extension's. Its attribution (upstream, path, version,
  SPDX licence, copyright) is declared and rendered with every prompt and every host file derived from it.
- **D6. The registry fails closed.** A prompt or gated command it cannot serve as declared (an unknown gated code, a
  subject that is no reference argument, an argument type a prompt does not take, a reserved name, a `when` that is no
  bool setting, a step that names nothing) is dropped whole with one W021; only a step's `agent` and an empty attribution
  are dropped alone. A gate it could not evaluate would otherwise be ignored.

## Rejected

Shipping the prompts as host skills only: an MCP-only client would get no orchestration, and nothing would refuse a
skipped step. Gates as a new rule kind: the report and the guard passes already decide; a gate names their codes. A new
diagnostic for a failed gate: it is an operation's refusal, not a finding about the project.

## Consequences

A new declaration (prompts, and gates and a subject on commands, a pass's `reports`, `PassInput.trigger`), a prompt
export, MCP prompts and one core tool (38), one CLI command, a `Host`, `Scopes::within` and a protocol minor. The
existing core prompts are unchanged. Following a prompt is checked where the agent acts (commands, writes), not where it
reads. An older host ignores the new keys (W138), so a gated command runs ungated there.
