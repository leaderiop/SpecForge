# The call target takes its own arguments

**Status:** accepted (2026-10-23, architecture round 26, plan 10). Extends [ADR 0014](0014-the-session-keeps-itself-current.md)
("The MCP call target") and [ADR 0033](0033-a-tools-arguments-are-one-typed-definition.md) D3 to every handler; amends
[ADR 0024](0024-one-mcp-request-pipeline.md) D5 (handlers are handed arguments without the target's) and
[ADR 0017](0017-one-extension-surface-table.md) (I017's reasons, and the SDK's refusal).

## Context

An MCP call's arguments were read by two readers that disagreed about `path`: the call target read it (every entry but
`Unscoped` accepts it), and the handler read the same object again. Core tools and prompts subtracted the target's
names before refusing unknowns; the extension adapters did not. All 40 extension tools of a software + product +
testing project accepted `{"path": "."}` for containment and then refused it (`INVALID_INPUT unknown argument
'path'`), `{"path": null}` included, and an explicit tool's export was handed the target's `path` as if it were its
own argument (or refused it as an undeclared key when its schema is closed).

```text
BEFORE  {"path": "."} -> target: own root, Served -> core tool runs
                                                 -> command rule: unknown 'path'   (refused)
                                                 -> mcp__ export handed path        (forwarded)
AFTER   {"path": "."} -> target takes path -> every handler is handed {}
```

## Decision

- **D1.** The request pipeline hands every handler (core tool, prompt, extension command, explicit extension tool)
  the call's arguments without the names its target reads (`TargetSpec::handler_arguments`); `args::undeclared`
  knows only the entry's own names.
- **D2.** Events record the arguments as sent.
- **D3.** An explicit extension tool whose input schema declares a target name, as a property or a required name, is
  refused by one rule the host and the SDK both run (`specforge_protocol_types::tool_input::refusal`, beside
  `command_args`): the SDK's builder panics when the extension is built, and the host does not serve such a tool from
  any guest (I017), as for a command declaring an arg named `path` (ADR 0017). The rule's list equals the target's
  (`TargetSpec::accepted`), pinned by a host test.

## Rejected

- Each adapter subtracting `accepted()`: a fourth and fifth reader of the target's names.
- `resolve` returning the handed arguments: the target would own a copy of the call.
- Unifying the target's and the command's refusal shapes: reopens ADR 0017 D5.
- Listing `path` on extension tools.
- A compile-time check in `#[extension]`: the macro cannot see a schema built at run time.
- A host-only rule, or a second list in the SDK: the two would drift.
- Serving such a tool and never showing it `path`, or passing `path` through to it.

## Consequences

Over MCP, every extension tool accepts its own project's `path` (and a null one) and still refuses another
project's as the target; an explicit tool's export never receives `path`; such a tool declaring `path` is not served.
For an SDK author, it panics when the extension is built (its first test). Rust API: `specforge_mcp::args::undeclared`
takes two arguments; new `specforge_protocol_types::tool_input`. Every SDK blob is re-vendored (hashes only). No
listing, no snapshot, no CLI change.

## What would reopen it

A target argument an entry must also read (a handler that needs to know which `path` was sent), or extension tools
reaching another project (`AnyProject`).
