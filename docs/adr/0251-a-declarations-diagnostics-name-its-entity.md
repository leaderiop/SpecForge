# A declaration's diagnostics name its entity

**Status:** accepted (2026-10-26, architecture round 27, plan 09). Amends [ADR 0016](0016-navigation-is-one-module.md) D7: `Subject { entity }` is also the host graph build's payload for a diagnostic about one declaration.

## Context

- The build drops a block that re-declares an id (E002 for the same kind, W060 for another) and reports it at the dropped block's span with no data. A dropped block is no node, so navigation's attribution (data, else the node whose block holds the span) found no subject: MCP `inspect`, the LSP hover and `suggest_fixes`' entity filter treated the diagnostic as about nobody.
- E076, E081 and I005 reported inside a dropped block were orphaned the same way.
- The build knows the subject when it reports; `DiagnosticData::Subject { entity }` already carries one entity name to attribution.

```text
BEFORE  E002 span=dropped block, data None -> subjects(): no node holds it -> inspect create_task []
AFTER   E002 span=dropped block, data Subject{create_task} -> node create_task (first decl.) -> [E002]
```

## Decisions

- **D1.** Every diagnostic the declaration sweep reports about one declaration (E076, E081, E002, W060, I005) carries `Subject { entity: <its id> }`, placed or not.
- **D2.** The subject is the id, whose node is the first declaration of any kind; the dropped block is never a subject. W060's subject is therefore the other kind's node.
- **D3.** W143 (a define block, which declares nothing, ADR 0005) names nothing.
- **D4.** Attribution, placement (ADR 0227: these have spans) and every surface are unchanged; they read the data they already read.

## Rejected

- A `Duplicate { entity, first }` variant: the first site is the id's node, and go-to-definition already reaches it.
- Attribution reading the parse for dropped blocks: ops would hold a second record of declarations beside the graph (ADR 0176).
- Data only on dropped blocks: a branch for the same answer.

## Consequences

- Breaking: the five codes' JSON gains `"data": {"kind": "subject", "entity": <id>}` in CLI `check`, `watch --format json`, MCP `validate`, `inspect`, `specforge://diagnostics`, `diagnosticsChanged` and LSP `Diagnostic.data`. MCP `inspect`, the LSP hover and `suggest_fixes`' entity filter now see them.
- No new code, no re-vendor.

## What would reopen it

- A fix that needs the first declaration's site as data.
- A diagnostic produced after the build about a block the graph does not hold.
