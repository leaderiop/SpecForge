---
name: spec-capabilities
description: "Author capability specification files in a spec's capabilities/ directory. Each file describes what a user can do (UX-XX-NNN IDs) with interaction flows, embedded Mermaid diagrams, and full traceability to features and behaviors. Use when creating new capability specs, documenting user-facing interaction paths, or bridging the gap between features (why) and behaviors (how)."
---

# Spec Capabilities

Rules and conventions for authoring **capability specification files** in a spec's `capabilities/` directory. Capability files describe *what the user can do* — they bridge features (the *why*) and behaviors (the *how*) by documenting concrete interaction flows from the user's perspective.

## When to Use

- Creating a new capability specification for a user-facing interaction
- Documenting the step-by-step flow a persona follows on a specific surface
- Adding Mermaid diagrams (sequence, state, flowchart) to capability files
- Organizing capabilities into groups and maintaining `index.yaml`
- Tracing capabilities back to features and behaviors

## Directory Structure

```
capabilities/
  index.yaml                    # Manifest of all capability files, grouped
  UX-XX-001-<name>.md           # One file per capability
  UX-XX-002-<name>.md
  ...
```

For large specs (>30 capabilities), group sub-folders are optional:

```
capabilities/
  index.yaml
  flow-operations/              # Optional group sub-folder
    UX-XX-001-<name>.md
    UX-XX-002-<name>.md
  graph-queries/                # Optional group sub-folder
    UX-XX-009-<name>.md
```

### index.yaml Schema

```yaml
# Capabilities Index — UX-XX-001 through UX-XX-NNN
# "What the user can do" layer bridging features (why) and behaviors (how)

groups:
  - name: <Group Name>
    capabilities:
      - id: UX-XX-001
        file: UX-XX-001-run-predefined-flow.md
        title: Run a Predefined Flow
      - id: UX-XX-002
        file: UX-XX-002-run-flow-with-preset.md
        title: Run a Flow with Preset
  - name: <Another Group>
    capabilities:
      - id: UX-XX-009
        file: UX-XX-009-query-graph-natural-language.md
        title: Query Graph Using Natural Language
```

When using group sub-folders, `file:` paths include the sub-folder prefix:

```yaml
groups:
  - name: Flow Operations
    capabilities:
      - id: UX-XX-001
        file: flow-operations/UX-XX-001-run-predefined-flow.md
        title: Run a Predefined Flow
```

**Rules:**
- Every `.md` file in `capabilities/` MUST have a corresponding entry in `index.yaml`
- Every entry MUST have a corresponding file on disk
- No duplicate `id` values across entries
- Group names describe functional areas (e.g., "Flow Operations", "Graph & Knowledge Queries")
- Capabilities are listed in sequential ID order within each group

## File Naming

- ID-prefixed: `UX-XX-NNN-<name>.md`
- `UX` = user experience prefix (always literal `UX`)
- `XX` = 2-3 character package infix (e.g., `SF` for specforge, `GD` for guard)
- `NNN` = zero-padded sequential number
- Kebab-case name describing the capability verb phrase
- Examples: `UX-SF-001-run-predefined-flow.md`, `UX-GD-003-evaluate-policy.md`

## File Template

```markdown
---
id: UX-XX-NNN
kind: capability
title: "<Verb phrase — what user does>"
status: active
features: [FEAT-XX-NNN, ...]
behaviors: [BEH-XX-NNN, ...]
persona: [developer, team-lead]      # YAML array
surface: [cli, dashboard]             # YAML array
group: "<Group Name>"                 # Optional — matches group sub-folder if used
---

# <Title>

## Use Case

<1-2 paragraphs from user perspective. WHO wants WHAT and WHY.>

## Interaction Flow

` ``text
┌──────────┐    ┌──────────┐    ┌───────────────────┐
│ <Persona> │    │ <Surface> │    │ <Backend Component>│
└────┬─────┘    └────┬─────┘    └────────┬──────────┘
     │  <action>     │                   │
     │──────────────►│  <operation>      │
     │               │──────────────────►│
     │               │  <result>         │
     │               │◄──────────────────│
     │  <feedback>   │                   │
     │◄──────────────│                   │
` ``

` ``mermaid
sequenceDiagram
    actor <Persona>
    participant <Surface> as <Surface Label>
    participant <Backend> as <Backend Component>
    <Persona>->>+<Surface>: <action>
    <Surface>->>+<Backend>: <operation>
    <Backend>-->>-<Surface>: <result>
    <Surface>-->>-<Persona>: <feedback>
` ``

## Steps

1. <Step referencing BEH-XX-NNN where applicable>
2. ...

## State Model

` ``text
         start       pause        resume
[*] ───► Idle ───► Running ───► Paused ───► Running
                     │                │
                     │  finish        │  cancel
                     ▼                ▼
                  Completed      Cancelled
                     │                │
                     ▼                ▼
                    [*]              [*]
` ``

` ``mermaid
stateDiagram-v2
    direction LR
    [*] --> Idle
    Idle --> Running: start
    Running --> Paused: pause
    Paused --> Running: resume
    Running --> Completed: finish
    Completed --> [*]
` ``

## Decision Paths

` ``text
┌─────────────────┐
│ Review outputs   │
└────────┬────────┘
         ▼
   ┌──────────┐
  ╱  Approve?  ╲
 ╱              ╲
Yes              No
 │                │
 ▼                ▼
┌──────────┐  ┌──────────────────┐
│ Flow     │  │ Provide feedback │
│ proceeds │  └────────┬─────────┘
└──────────┘           ▼
              ┌──────────────────┐
              │ Re-enter phase   │
              └──────────────────┘
` ``

` ``mermaid
flowchart TD
    A[Review outputs] --> B{Approve?}
    B -->|Yes| C([Flow proceeds])
    B -->|No| D[Provide feedback]
    D --> E([Re-enter phase])
` ``

## Related Capabilities

| Capability | Relationship |
|------------|-------------|
| [UX-XX-NNN](./UX-XX-NNN-slug.md) | Precedes / Follows / Enables |

## Traceability

| Behavior | Feature | Role in this capability |
|----------|---------|------------------------|
| BEH-XX-NNN | FEAT-XX-NNN | <what this behavior contributes> |
```

**Section inclusion rules:**

| Section | Required | Condition |
|---------|----------|-----------|
| Use Case | Always | Every capability |
| Interaction Flow | Always | Every capability gets a sequence diagram |
| Steps | Always | Every capability |
| State Model | Conditional | Only for stateful capabilities (pause/resume/cancel, activate/deactivate) |
| Decision Paths | Conditional | Only for branching capabilities (approve/reject, if/then, alternative paths) |
| Related Capabilities | Conditional | Only when the capability chains to or depends on other capabilities |
| Traceability | Always | Every capability |

## Diagram Rules

### Which Diagrams to Include

| Capability Pattern | Diagrams | Detection Heuristic |
|---|---|---|
| Simple CLI command | Sequence only | Single linear path, no branching, no state changes |
| Dashboard / monitoring | Sequence only | Read-only views, no user decisions |
| Stateful lifecycle | Sequence + State | Steps mention pause/resume/cancel, activate/deactivate, or multiple statuses |
| Branching decision | Sequence + Decision Flowchart | Steps contain approve/reject, if/then, or alternative paths |
| Multi-step chain | Sequence + Related Capabilities | Capability references or depends on other capabilities |

### ASCII Diagram Rules

Every Mermaid diagram (`mermaid` code block) MUST be preceded by an equivalent ASCII diagram in a `text` code block. This ensures interaction flows are readable in plain-text contexts (terminals, git diffs, non-rendering editors).

**Placement:** The `text` block goes immediately before its corresponding `mermaid` block.

**Characters:** Use Unicode box-drawing characters:
- Boxes: `┌ ┐ └ ┘ ─ │`
- Arrows: `►` (request), `◄` (response), `▼` `▲` (vertical flow)
- Lifelines: `│` descending from participant boxes
- Solid request arrows: `│────────────►│`
- Response arrows: `│◄────────────│`
- Dotted (async): `│· · · · · · ·│`
- Decisions: `╱ ╲` diamond approximation
- Alt/else labels: `[if approved]` / `[if rejected]`
- States: `State1 ──label──► State2`
- Terminal nodes: `[*]` or boxed text

**Width:** Keep under 80 characters where possible.

**Sequence diagrams:** Participants as boxed headers with lifeline `│` descending.
**State diagrams:** States connected by labeled arrows; `[*]` for initial/final.
**Flowcharts:** Top-down with `╱ ╲` diamonds for decisions, boxed actions/outcomes.

### Sequence Diagram Conventions

- Always start with `actor <Persona>` using the primary persona from frontmatter
- Participant naming: surface name (`CLI`, `Dashboard`, `Desktop`), then backend components (`FlowEngine`, `GraphStore`, `AgentBackend`, etc.)
- Use `alt`/`else` for branching paths
- Use `opt` for optional steps
- Use `loop` for convergence/retry patterns
- Use activation (`+`/`-`) for processing spans

### State Diagram Conventions

- Use `stateDiagram-v2` with `direction LR`
- States match the status values used in the Steps section
- Include `[*]` initial and final transitions
- Add notes for guard conditions

### Decision Flowchart Conventions

- Use `flowchart TD` (top-down)
- Diamond `{Decision}` nodes for user choices
- Terminal nodes use `([Outcome])` shape (stadium/rounded)
- Rectangular `[Action]` nodes for intermediate steps

See [Diagram Selection Reference](./references/diagram-selection.md) for detailed Mermaid syntax examples.

## Content Rules

1. **YAML frontmatter** — Every capability file MUST start with `---` frontmatter containing `id`, `kind: capability`, `title`, `status`, `features`, `behaviors`, `persona`, `surface`.
2. **Persona and surface as arrays** — Always use YAML arrays (`[developer, team-lead]`), not comma-separated strings.
3. **Verb-phrase titles** — Titles describe what the user does: "Run a Predefined Flow", "Approve or Reject a Phase Transition". Start with a verb.
4. **User perspective in Use Case** — Write from the user's point of view. Use WHO/WHAT/WHY framing.
5. **BEH references in Steps** — Steps should reference `BEH-XX-NNN` IDs in parentheses where the behavior is exercised.
6. **No duplicate contracts** — The capability describes the *interaction flow*; the behavior files contain formal MUST/SHALL contracts. Don't duplicate.
7. **Unique IDs** — Every capability has a unique `UX-XX-NNN` ID. No duplicates across the entire `capabilities/` directory.
8. **Sequential numbering** — New capabilities get the next available number in the sequence.
9. **Diagrams are substantive** — Only include a diagram if it adds clarity beyond the Steps list. A 2-step sequence diagram with no branching adds no value.
10. **Diagram participant alignment** — Sequence diagram participants must match the `surface` and backend components referenced in Steps.
11. **ASCII diagram pairing** — Every `mermaid` code block MUST be immediately preceded by a `text` code block containing an equivalent ASCII diagram using box-drawing characters (─, │, ┌, ┐, └, ┘, ►, ◄, ▼, ╱, ╲).
12. **Group sub-folders are optional** — For large specs (>30 capabilities), capabilities MAY be organized into group sub-folders (e.g., `flow-operations/`, `graph-queries/`). Flat layout remains the default. When sub-folders are used, the `group` frontmatter field and `index.yaml` `file:` paths must include the sub-folder prefix.

## Cross-References

```markdown
# From capability to behaviors:
| BEH-SF-049 | FEAT-SF-004 | Resolves predefined flow definition from registry |

# From capability to features (in frontmatter):
features: [FEAT-SF-004, FEAT-SF-009]

# From capability to other capabilities:
| [UX-SF-004](./UX-SF-004-monitor-running-flow.md) | Follows |

# Deliverables that bundle this capability (via deliverable frontmatter capabilities[]):
See [DLV-SF-001](../deliverables/DLV-SF-001-desktop-app.md).

# From other documents linking to a capability:
See [UX-SF-001](capabilities/UX-SF-001-run-predefined-flow.md).

# From sub-documents linking up:
See [UX-SF-001](../capabilities/UX-SF-001-run-predefined-flow.md).
```

## Quick Reference

- [Diagram Selection](./references/diagram-selection.md) — Diagram type matrix + Mermaid patterns
- [Example: Simple CLI](./examples/simple-cli.md) — Single-path CLI command capability
- [Example: Stateful Lifecycle](./examples/stateful-lifecycle.md) — Pause/resume/cancel pattern
- [Example: Branching Decision](./examples/branching-decision.md) — Approve/reject pattern
