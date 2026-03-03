---
name: spec-decisions
description: "Author Architecture Decision Records (ADRs) in a spec's decisions/ directory. Each file documents design rationale with ADR-NNN IDs, Context/Decision/Consequences structure, and concrete code examples. Use when capturing design rationale, creating new ADRs, or auditing decision records for completeness."
---

# Spec Decisions

Rules and conventions for authoring **Architecture Decision Records** (ADRs) in a spec's `decisions/` directory. ADRs document design rationale — the *why* behind design choices.

## When to Use

- Capturing a non-obvious design decision
- Creating a formal ADR for an architectural choice
- Auditing ADRs for completeness (missing consequences, missing code examples)
- Superseding or deprecating an existing ADR

## Directory Structure

```
decisions/
  index.yaml                    # Manifest of all ADR files
  ADR-001-<topic>.md            # One file per decision
  ADR-002-<topic>.md
  ...
```

### index.yaml Schema

```yaml
kind: decisions
package: "@hex-di/<name>"
entries:
  - id: ADR-001
    file: ADR-001-closures-over-classes.md
    title: Closures Over Classes
    status: Accepted             # Accepted | Superseded by ADR-NNN | Deprecated
  - id: ADR-002
    file: ADR-002-graph-first-architecture.md
    title: Graph-First Architecture
    status: Accepted
```

**Rules:**
- Every `.md` file in `decisions/` MUST have a corresponding entry
- Every entry MUST have a corresponding file on disk
- No duplicate `id` values across entries
- `status` tracks the lifecycle of the decision

## File Naming

- ID-prefixed: `ADR-NNN-<topic>.md`
- Three-digit prefix: `001-`, `002-`, etc. assigned sequentially
- Kebab-case name describing the decision topic
- Examples: `ADR-001-closures-over-classes.md`, `ADR-005-graph-first-architecture.md`

## File Template

```markdown
---
id: ADR-NNN
kind: decision
title: "<Decision Title>"
status: Accepted
date: "YYYY-MM-DD"
supersedes: []
invariants: []
---

# ADR-NNN: <Decision Title>

## Context

<Problem statement. What design question needed answering? What constraints exist?>

## Decision

<What was decided. Include code snippets showing the chosen approach.>

## Consequences

**Positive**:
- <benefit 1>
- <benefit 2>

**Negative**:
- <trade-off 1>
- <trade-off 2>

**Trade-off accepted**: <one-sentence justification for why negatives are acceptable>
```

## Content Rules

1. **YAML frontmatter** — Every ADR file MUST start with `---` frontmatter containing `id`, `kind: decision`, `title`, `status`, `date`, `supersedes`, `invariants`. The `## Status` section and `**Status:**`/`**Date:**`/`**Supersedes:**` lines are REMOVED from prose — that metadata lives in frontmatter.
2. **Focus on the why** — Behavior specs describe *what*; ADRs explain *why*.
2. **Context is mandatory** — Always include a `## Context` section that explains the problem space.
3. **Code examples required** — Always include concrete code showing the chosen approach.
4. **Both sides of consequences** — Always list both positive and negative consequences. Never write an ADR with only positive consequences.
5. **Trade-off justification** — Include an explicit `**Trade-off accepted**` statement.
6. **Edge case notes** — Include edge case notes where behavior is non-obvious.

## Status Lifecycle

```
Accepted
  └─► Superseded by ADR-NNN    (new decision replaces this one)
  └─► Deprecated                (decision no longer relevant)
```

- When superseding, update the old ADR's status AND create the new ADR
- ADR numbers are never reused — even deprecated ADRs keep their number

## Cross-References

```markdown
# From ADR to behaviors it affects:
Referenced by: [BEH-XX-001-graph-ops.md](../behaviors/BEH-XX-001-graph-ops.md)

# From behaviors to ADRs (in header):
**ADRs:** [ADR-001](../decisions/ADR-001-closures-over-classes.md), ...

# From traceability to ADRs:
Maps each ADR to the invariants and behavior files it affects
```
