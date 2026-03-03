---
name: spec-behaviors
description: "Author behavioral contract files in a spec's behaviors/ directory. Each file groups related behaviors under a capability heading with BEH-XX-NNN IDs, Contract sections using RFC 2119 keywords, and Verification sections. Use when creating new behavior files, adding behaviors to existing files, or auditing behavior specs for completeness."
---

# Spec Behaviors

Rules and conventions for authoring **behavioral contract files** in a spec's `behaviors/` directory. Behavior files are the core of the spec — every testable contract lives here.

## When to Use

- Creating a new behavior file for a capability domain
- Adding new `BEH-XX-NNN` entries to an existing behavior file
- Auditing behavior files for completeness (missing Contract/Verification sections, broken cross-references)
- Splitting an oversized behavior file into multiple files
- Assigning BEH ID ranges to a new behavior file

## Directory Structure

```
behaviors/
  index.yaml                    # Manifest of all behavior files
  BEH-XX-001-<capability>.md   # One file per capability domain
  BEH-XX-009-<capability>.md
  ...
```

### index.yaml Schema

```yaml
kind: behaviors
package: "@hex-di/<name>"
infix: XX                       # 2-3 char package infix
entries:
  - id: BEH-XX-001
    file: BEH-XX-001-graph-operations.md
    title: Graph Operations
    status: active              # active | draft | deprecated
    id_range: "001--008"        # BEH ID allocation range
  - id: BEH-XX-009
    file: BEH-XX-009-session-materialization.md
    title: Session Materialization
    status: active
    id_range: "009--016"
```

**Rules:**
- Every `.md` file in `behaviors/` MUST have a corresponding entry
- Every entry MUST have a corresponding file on disk
- No duplicate `id` values across entries
- `id_range` documents the BEH-XX-NNN allocation range for that file

## File Naming

- ID-prefixed: `BEH-XX-NNN-<capability>.md` where NNN is the first BEH ID in the file
- Kebab-case name describing the capability domain
- Examples: `BEH-SF-001-graph-operations.md`, `BEH-FL-017-flow-definitions.md`

## File Template

```markdown
---
id: BEH-XX-NNN
kind: behavior
title: "<Capability Name>"
status: active
id_range: "NNN--NNN"
invariants: [INV-XX-N]
adrs: [ADR-NNN]
types: [<domain>]
ports: [<PortName>]
---

# BEH-XX-NNN -- <Capability Name>

## BEH-XX-NNN: <Descriptive Title>

> **Invariant:** [INV-XX-N](../invariants/INV-XX-N-<name>.md) -- <Invariant Name>

<Prose description. What the system does, under what conditions, and why.
Include TypeScript signatures and input/output tables inline when they
clarify the contract.>

### Contract

REQUIREMENT (BEH-XX-NNN): <Formal MUST/SHALL statement using RFC 2119
keywords. This is the testable assertion.>

### Verification

- Unit test: <What the unit test checks.>
- Integration test: <Cross-component verification.>
- Edge case test: <Boundary or failure scenario.>

---

## BEH-XX-NNN+1: <Next Title>
...
```

## Content Rules

1. **YAML frontmatter** — Every behavior file MUST start with `---` frontmatter containing `id`, `kind: behavior`, `title`, `status`, `id_range`, `invariants`, `adrs`, `types`, `ports`. The `**Invariants:**`, `**ADRs:**`, `**Types:**` header lines and `## Ports Used`, `## Referenced Invariants`, `## Referenced ADRs` footer sections are REMOVED from prose — that metadata lives in frontmatter.
2. **Unique IDs** — Every behavior has a unique `BEH-XX-NNN` ID. No duplicates across the entire `behaviors/` directory.
2. **Invariant blockquote** — The `> **Invariant:**` blockquote is present when the behavior enforces or is governed by an invariant. Omit it when the behavior is standalone.
3. **RFC 2119 keywords** — The `### Contract` section uses MUST, SHALL, SHOULD, MAY per RFC 2119.
4. **Verification describes how** — The `### Verification` section describes *how* to test, not the test code itself.
5. **TypeScript signatures** — Include type signatures for functions when they clarify the contract.
6. **Input/output examples** — Show concrete examples for transformations.
7. **Edge cases** — Document edge cases explicitly (null, undefined, empty, NaN, nested types).
8. **ADR links** — Link to the ADR when a design choice is non-obvious.
9. **Operational tag** — Tag procedural/organizational requirements that cannot be verified by automated tests with `[OPERATIONAL]`. Format: `REQUIREMENT (BEH-XX-NNN) [OPERATIONAL]: <text>`. These are excluded from automated test coverage calculations.

## ID Assignment

- IDs are sequential within a file's allocation range
- IDs are never reused — deleted requirements keep their number reserved
- New behaviors append to the end of the allocation range or start a new file
- Document allocation ranges in `process/requirement-id-scheme.md`

## Cross-References

```markdown
# From behavior file header:
**Invariants:** [INV-XX-N](../invariants/INV-XX-N-<name>.md), ...
**ADRs:** [ADR-NNN](../decisions/ADR-NNN-<topic>.md), ...
**Types:** [types/<domain>.md](../types/<domain>.md)

# From invariant blockquote within a behavior:
> **Invariant:** [INV-XX-N](../invariants/INV-XX-N-<name>.md) -- <Invariant Name>

# From other documents linking to a behavior:
See [BEH-XX-001](behaviors/BEH-XX-001-<name>.md#beh-xx-001-descriptive-slug).
```
