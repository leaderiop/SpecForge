# A failed MCP tool call is one McpError in its text block, worded once per fact

**Status:** accepted (2026-10-26, architecture round 28, plan 12). Amends [ADR 0252](0252-every-mcp-tool-refuses-with-an-mcp-error.md) D2 (a missing argument is every tool's sentence); extends [ADR 0033](0033-a-tools-arguments-are-one-typed-definition.md) D2 (it holds on an extension command's tool too); amends [ADR 0004](0004-architecture-migration-decisions.md) D4-c (a failed call carries no `structuredContent`); applies [ADR 0253](0253-a-source-code-path-a-request-names-has-one-reader.md) D2 and D3 to the infer prompt.

## Context

- One MCP server said a missing argument two ways: a core tool `Missing required parameter: entity_id` (ADR 0033 D2), an extension command's tool `missing required arg 'feature'` (the arg rule's words, which ADR 0252 D2 chose "with the message the CLI writes"). ADR 0252 never weighed ADR 0033 D2.
- The infer prompt worded its own refusal of an empty `file:` scope (`Empty file path in scope 'file:'`) although the reader it calls next refuses it in a sentence of its own (`'' names no file: a source file is named from the project root`).
- A failure was `structuredContent` on the 44 tools without an `outputSchema` and the text block alone on the 31 with one; the text block already carried the McpError on all of them.

```text
BEFORE  inspect {}            -> "Missing required parameter: entity_id"
        product.x {}          -> "missing required arg 'feature'"        failed validate -> text + structuredContent
        infer file: ""        -> "Empty file path in scope 'file:'"      failed inspect  -> text
AFTER   every tool            -> "Missing required parameter: <name>"    any failed tool -> text
        infer file: ""        -> the source path reader's sentence
```

## Decisions

- **D1.** A failed tool call is an `isError` result whose text block is its McpError, on every tool; it never carries `structuredContent`.
- **D2.** A missing required argument is `Missing required parameter: <name>` on every tool, an extension command's included. The CLI keeps the arg rule's sentence.
- **D3.** The infer prompt's empty file scope is the source path reader's refusal, on `scope`. No ops reader words an empty kind, so that check stays.

## Rejected

- The arg rule's `missing required arg '<name>'` on every tool: it rewords the core tools' ADR, 29 assertion and snapshot sites.
- Leaving both sentences (ADR 0252 D2 as it stood).
- Keeping `structuredContent` on a failed call of a tool without an `outputSchema`: two placements, told apart by a fact a client must know.

## Consequences

- Breaking: MCP, a command tool's missing-argument message, the infer prompt's empty-file message, and a failed untyped call's `structuredContent`. Rust API: `tool::envelope` loses a parameter, `ToolSpec::is_typed` goes.
- The CLI is unchanged: a command's missing argument reads differently on the two surfaces; every other refusal matches.
- No new code, no re-vendor.

## What would reopen it

- An MCP revision that types errors in `structuredContent`.
- A client that needs the command's own message verbatim over MCP.
