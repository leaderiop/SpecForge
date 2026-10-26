# ref

> **Structural, not from an extension.** `ref` is one of the five keywords the core grammar knows (`spec`, `ref`,
> `use`, `define`, `defaults`); it exists in every project, whatever extensions it loads.

## Purpose

A `ref` names a resource outside the graph: an issue, a ticket, a design, a standard operating procedure, a
document. It answers: **"What outside resource is this connected to?"**

Refs carry no contract. They are anchors: any entity lists them in its `refs` field, and prose can cite them
inline as `[gh.issue:42]`.

## Syntax

A ref's id is `scheme.kind:identifier`: the scheme names the source (`gh`, `jira`, `sop`), the kind the resource
type (`issue`, `epic`, `doc`), the identifier the locator. The title is required.

```spec
ref gh.issue:42 "Track login timeout bug"
ref jira.epic:PROJ-123 "Q2 auth overhaul"
```

A ref can take a block of metadata:

```spec
ref sop.doc:QA-017 "Batch release SOP" {
  labels ["gxp"]
}
```

Entities name refs in `refs`, and prose can cite them in brackets:

```spec
invariant audit_integrity "Audit Trail Integrity" {
  guarantee "The audit log is append-only. See [gh.issue:42]."
  refs      [gh.issue:42, jira.epic:PROJ-123]
}
```

## Providers

A scheme can be registered by a provider: an extension a `specforge.json` `providers` entry names, with its own
settings. `specforge providers` lists each entry and whether its scheme is registered. Today a provider only
registers its scheme: **no provider reads the outside system yet**, so a ref is not checked against the issue
tracker or the document store it names. Once a project configures providers, a ref whose scheme none registered is
I005.

## Validation rules

| Code | Rule |
|------|------|
| E002 | Two refs share an id |
| E076 | A ref has no title |
| W012 | A ref nothing references |
| I005 | A scheme no configured provider registered (only once a project configures providers) |

## Design guidance

Use refs for resources people act on (issues, tickets, designs, procedures, RFCs), not arbitrary URLs. Prefer the
scheme-based id to a raw URL: it is one stable name the whole graph uses. For links between entities, use their ids
directly; refs are for the outside world.
