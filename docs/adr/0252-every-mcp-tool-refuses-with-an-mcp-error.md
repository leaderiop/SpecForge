# Every MCP tool refuses with an McpError

**Status:** accepted (2026-10-26, architecture round 27, plan 10). Amended by [ADR 0275](0275-a-failed-mcp-tool-call-is-one-mcp-error-in-its-text-block.md) (D2: a missing argument is every tool's sentence). Reopens [ADR 0017](0017-one-extension-surface-table.md) D5; amends [ADR 0011](0011-extension-command-contract-and-the-product-surface.md) B and O2 and the "Rejected" list of [ADR 0239](0239-the-call-target-takes-its-own-arguments.md); restores [ADR 0004](0004-architecture-migration-decisions.md) D4-a for extension commands.

## Context

- An auto-promoted command tool failed in three vocabularies: the call target's and the host's refusals were McpErrors (`invalid_input`, `argument`, `tool`); the arg rule's refusal and the command's own error were the command's object (`{"code":"INVALID_INPUT"|"ENTITY_NOT_FOUND", message, suggestion?}`, no `argument`, no `tool`); a run that failed without that object was bare text blocks.
- The invariant `mcp_structured_error_responses` says a failed tool call is an `isError` result whose content is an McpError.
- The split is older than ADR 0239: ADR 0011 B (2026-10-03) introduced the command object two days after D4-a; ADR 0017 D5 chose the command's side for argument errors but D14 and the trap path kept McpErrors, so the tool still had two.

```text
BEFORE  arg rule -> {"code":"INVALID_INPUT",message}   own object -> {"code":"ENTITY_NOT_FOUND",..}
        prose failure -> [text, text] isError           ToolOutcome = Done { is_error } | Refused
AFTER   arg rule -> McpError invalid_input + argument   own object -> McpError by its code
        prose failure -> internal_error + data          ToolOutcome = Done | Refused
```

## Decisions

- **D1.** Over MCP every failure of every tool is one McpError naming the tool. The CLI keeps writing the command's object on stderr.
- **D2.** The arg rule's refusal is `invalid_input` on the arg it names (the first missing one), with the message the CLI writes and the close arg or value as `data.suggestion` ("did you mean '<x>'?", the form a core tool uses).
- **D3.** A command's own error object (`CommandError`, one object on stderr, nothing on stdout) is the McpError its code names: `INVALID_INPUT` is `invalid_input`, `ENTITY_NOT_FOUND` is `entity_not_found`, any other code is `internal_error` keeping the code in `data.code`. Its `entity_id` is kept; it names no argument (the host does not guess).
- **D4.** A failed run without one error object is `internal_error`, "command <export>() of '<extension>' exited <n>: <stderr>", with `data.{exit_code, stdout, stderr}`.
- **D5.** `ToolOutcome::Done` loses `is_error`, `ToolOutcome::texts` its flag and `ToolOutcome::failed` goes: `Refused` is the only way a tool outcome fails.

## Rejected

- Writing every refusal of a command tool as a command object (target, no project, E028): still two shapes per server, and `argument`, `tool` and `diagnostic` lost.
- Mapping only the host's `ArgError`: the product's most common failure, `ENTITY_NOT_FOUND`, would stay uppercase.
- Exempting command tools from the invariant: the defect becomes the contract.
- Matching every `McpErrorCode` in a guest's code, ignoring case: widens the command contract by implication.
- Keeping text blocks for a failed run without an object.
- A `From<&ArgError>` in the core error module, or a method on `ArgError` in protocol-types (a re-vendor for a `match` the MCP crate can make).
- Naming the argument of a command's own error by its value.

## Consequences

- Breaking: MCP, every auto-promoted command tool's failure (code case, `argument`, `tool`, suggestion in `data`, unknown codes as `internal_error`, a failed run without an object as an McpError); Rust API `ToolOutcome::failed`, `Done::is_error` and the `texts` flag removed.
- `edit_unreadable` (kind `CompilationFailed`, ADR 0248) stays `compilation_failed` over MCP: this ADR changes no operation's mapping.
- The explicit `mcp__` tool keeps `data.violations` and gains no `argument`.
- No new code, no re-vendor.

## What would reopen it

- A command contract that names a third code.
- A client that needs the command's object verbatim over MCP.
