---
name: spec-features
description: "Author feature specification files in a spec's features/ directory. Each file describes a user-facing capability with FEAT-XX-NNN IDs, Problem/Solution framing, constituent behaviors table, acceptance criteria, and detailed contract. Use when creating new feature specs, organizing behaviors into features, or documenting the 'why' behind a group of related behaviors."
---

# Spec Features

Rules and conventions for authoring **feature specification files** in a spec's `features/` directory. Feature files bridge the gap between high-level product goals and low-level behavioral contracts — they describe *what* a capability delivers and *why*, while linking to the `BEH-XX-NNN` entries that define *how*.

## When to Use

- Creating a new feature specification for a user-facing capability
- Organizing related behaviors into a cohesive feature narrative
- Documenting the problem/solution framing for a group of behaviors
- Defining acceptance criteria for a feature that spans multiple behavior files

## Directory Structure

```
features/
  index.yaml                    # Manifest of all feature files
  FEAT-XX-001-<name>.md         # One file per feature
  FEAT-XX-002-<name>.md
  ...
```

### index.yaml Schema

```yaml
kind: features
package: "@hex-di/<name>"
infix: XX                       # 2-3 char package infix
entries:
  - id: FEAT-XX-001
    file: FEAT-XX-001-graph-composition.md
    title: Graph Composition
    status: active              # active | draft | deprecated
    behaviors: ["BEH-XX-001", "BEH-XX-002", "BEH-XX-003"]
  - id: FEAT-XX-002
    file: FEAT-XX-002-session-lifecycle.md
    title: Session Lifecycle
    status: active
    behaviors: ["BEH-XX-009", "BEH-XX-010"]
```

**Rules:**
- Every `.md` file in `features/` MUST have a corresponding entry
- Every entry MUST have a corresponding file on disk
- No duplicate `id` values across entries
- `behaviors` lists the BEH IDs that this feature encompasses

## File Naming

- ID-prefixed: `FEAT-XX-NNN-<name>.md`
- Kebab-case name describing the feature
- Examples: `FEAT-SF-001-graph-composition.md`, `FEAT-GD-003-policy-evaluation.md`

## File Template

```markdown
---
id: FEAT-XX-NNN
kind: feature
title: "<Feature Name>"
status: active
adrs: [ADR-NNN]
behaviors: [BEH-XX-NNN]
---

# FEAT-XX-NNN: <Feature Name>

## Problem

<What user problem or system need does this feature address?
What happens without it? What pain point exists?>

## Solution

<How does this feature solve the problem?
High-level approach without implementation details.>

## When to Use

<Under what conditions should a user/developer reach for this feature?
When is it NOT the right choice?>

---

## Constituent Behaviors

| BEH ID | Title | Behavior File |
|--------|-------|---------------|
| BEH-XX-NNN | <Title> | [BEH-XX-NNN-<name>.md](../behaviors/BEH-XX-NNN-<name>.md) |
| BEH-XX-NNN+1 | <Title> | [BEH-XX-NNN-<name>.md](../behaviors/BEH-XX-NNN-<name>.md) |
...

## Acceptance Criteria

- [ ] AC-1: <Measurable criterion that proves the feature works end-to-end>
- [ ] AC-2: <Criterion covering the primary success path>
- [ ] AC-3: <Criterion covering key error/edge cases>

## Detailed Contract

<Optional deeper prose expanding on the feature's contract.
Include TypeScript signatures, diagrams, or input/output tables
when they clarify the feature's boundaries. Link to individual
BEH entries for the formal MUST/SHALL statements.>
```

## Content Rules

1. **YAML frontmatter** — Every feature file MUST start with `---` frontmatter containing `id`, `kind: feature`, `title`, `status`, `adrs`, `behaviors`. The `**Status:**` and `**ADRs:**` lines are REMOVED from prose — that metadata lives in frontmatter.
2. **Unique IDs** — Every feature has a unique `FEAT-XX-NNN` ID. No duplicates across the entire `features/` directory.
2. **Problem before Solution** — Always frame the problem first. Features exist to solve problems.
3. **Behavior linkage** — The Constituent Behaviors table MUST reference actual `BEH-XX-NNN` entries from `behaviors/`. Every listed BEH must exist.
4. **Acceptance criteria are measurable** — Criteria should be testable assertions, not vague goals.
5. **No duplicate contracts** — The feature file describes the *what/why*; the behavior files contain the formal MUST/SHALL contracts. Don't duplicate.
6. **ADR links** — Reference ADRs that explain design decisions behind the feature.

## Cross-References

```markdown
# From feature to behaviors:
| BEH-XX-001 | Graph Node Creation | [BEH-XX-001-graph-ops.md](../behaviors/BEH-XX-001-graph-ops.md) |

# From feature to ADRs:
**ADRs:** [ADR-001](../decisions/ADR-001-closures-over-classes.md)

# From other documents linking to a feature:
See [FEAT-XX-001](features/FEAT-XX-001-graph-composition.md).

# Libraries that implement this feature (via library frontmatter features[]):
See [LIB-XX-001](libraries/core/LIB-XX-001-di-kernel.md).
```
