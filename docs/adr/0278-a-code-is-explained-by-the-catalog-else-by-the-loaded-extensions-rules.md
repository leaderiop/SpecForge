# A code is explained by the catalog, else by the loaded extensions' rules

**Status:** accepted (2026-10-26, architecture round 29, plan 02). Extends
[ADR 0026](0026-typed-diagnostic-codes.md) D4, amends [ADR 0012](0012-one-extension-declaration.md) (a validation
rule may carry a `description`), [ADR 0024](0024-one-mcp-request-pipeline.md) and
[ADR 0049](0049-a-core-tool-is-one-module.md) (no core tool is `Unscoped`).

`specforge check` reports a third-party extension's own codes (E900–E998, W900–W998, I900–I998), and its hint sends
the user to `specforge explain`. Explain read the host catalog alone, which by ADR 0026 D4 never holds those codes, so
`specforge explain E903` in the marketing example answered `unknown_code` (exit 1) and `specforge.explain` was
`invalid_input`, though the project's loaded rules declare E903 (`schema --validation-rules` lists it). A rule
declared no sentence saying what its code means; every other descriptor has a description.

## D1. Explain takes the loaded declarations

`specforge_ops::read::explain::explain(code, declarations)` answers in this order: a retired code (the catalog's), a
catalogued code (the catalog's entry, whoever declares a rule with it), a declared code, else `unknown_code`. The
declared answer is `{code, level, rules: [{extension, target_kind?, check, message, description?}], retired: false}`,
a third reply shape told apart by `rules`.

## D2. Only the codes the host takes answer

A rule answers only when `specforge_registry::declared_level(extension, rule)` is `Ok`: the verdict W150 reports
otherwise. Every such rule of every extension is listed, in load order (W023 keeps both rules of a repeated code);
their level is the first one's, which every accepted rule shares. Explain never describes a code differently from
how `check` attributed its diagnostics.

## D3. Each surface reads its project

The CLI reads the project at `--path` (default `.`; a loose root loads no extension, so outside a project the catalog
alone answers), and does not print the environment's diagnostics. MCP `specforge.explain` is a `View` tool: the served
project, the empty session's (no declarations) when nothing is served; the `Unscoped` input, target and call target
are gone. The LSP hover reads the session's declarations.

## D4. A rule may say what its code means

`ValidationRuleDescriptor.description: Option<String>` (skipped on the wire when absent), set by the SDK's
`RuleBuilder::description`. Explain prints it, the hover appends the reporting extension's accepted rule's description,
and `schema --validation-rules` lists it. No docs link is declared for a declared code.

## Rejected

- A code table as an eleventh describe category (`codes: [...]`), which would cover a pass's codes too: a protocol
  category, an SDK method and a duplicate of what rules already say, out of proportion for the defect.
- Docs only ("see the extension that reports it"): the `check` hint keeps sending users to a command that cannot
  answer.
- An `AnyProject` MCP target with a `path` argument: a client serving the project asks it.

## Consequences

- `specforge explain` is a project command: it loads the project's extensions (as `check` and `schema` do).
- A blob built before the field loads as before; its codes are explained without a description. The builtins and
  fixtures need a re-vendor (protocol-types and the SDK are blob inputs), with no declaration snapshot change.
- A third-party compiler pass's codes stay documented by their extension. The diagnostics JSON `title` and doctor's
  remediation stay the catalog's.

## What would reopen it

A third-party pass whose codes users need explained (a code table), or a rule that needs a docs link.
