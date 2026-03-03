# Example: Simple CLI Capability

This example shows a **simple CLI command** capability — single linear path, sequence diagram only.

---

```markdown
---
id: UX-SF-001
kind: capability
title: "Run a Predefined Flow"
status: active
features: [FEAT-SF-004, FEAT-SF-009]
behaviors: [BEH-SF-049, BEH-SF-057, BEH-SF-113]
persona: [developer]
surface: [cli]
---

# Run a Predefined Flow

## Use Case

A developer wants to execute one of the built-in flows (e.g., `spec-verify`, `code-review`, `drift-check`) against their current project. They invoke the flow by name from the CLI, and the system resolves the flow definition, validates inputs, and begins execution with default parameters.

This is the most common entry point for day-to-day SpecForge usage — selecting a known flow and watching it run to completion.

## Interaction Flow

` ``text
┌───────────┐  ┌─────┐  ┌──────────────┐  ┌────────────┐
│ Developer │  │ CLI │  │ FlowRegistry │  │ FlowEngine │
└─────┬─────┘  └──┬──┘  └──────┬───────┘  └─────┬──────┘
      │  run cmd  │            │                 │
      │──────────►│  resolve   │                 │
      │           │───────────►│                 │
      │           │  FlowDef   │                 │
      │           │◄───────────│                 │
      │           │  validate  │                 │
      │           │───┐        │                 │
      │           │◄──┘        │                 │
      │           │  execute   │                 │
      │           │─────────────────────────────►│
      │           │  progress (phase 1)          │
      │           │◄─────────────────────────────│
      │           │  progress (phase 2)          │
      │           │◄─────────────────────────────│
      │           │  FlowResult{completed}       │
      │  summary  │◄─────────────────────────────│
      │◄──────────│            │                 │
` ``

` ``mermaid
sequenceDiagram
    actor Dev as Developer
    participant CLI
    participant Registry as FlowRegistry
    participant Engine as FlowEngine

    Dev->>+CLI: specforge run spec-verify
    CLI->>+Registry: resolveFlow("spec-verify")
    Registry-->>-CLI: FlowDefinition
    CLI->>CLI: Validate project context
    CLI->>+Engine: execute(flow, context)
    Engine-->>CLI: ProgressEvent{phase: 1, status: running}
    Engine-->>CLI: ProgressEvent{phase: 2, status: running}
    Engine-->>-CLI: FlowResult{status: completed}
    CLI-->>-Dev: Summary + exit code 0
` ``

## Steps

1. List available predefined flows via `specforge flows list`
2. Select a flow by name (e.g., `specforge run spec-verify`)
3. System resolves the flow definition from the registry (BEH-SF-049)
4. System validates project context and required inputs
5. Flow execution begins, streaming progress to the terminal (BEH-SF-057)
6. CLI displays phase transitions and convergence status (BEH-SF-113)
7. Flow completes and outputs summary with exit code

## Traceability

| Behavior | Feature | Role in this capability |
|----------|---------|------------------------|
| BEH-SF-049 | FEAT-SF-004 | Resolves predefined flow definition from registry |
| BEH-SF-057 | FEAT-SF-004 | Executes flow phases and manages convergence |
| BEH-SF-113 | FEAT-SF-009 | CLI command parsing and progress output |
```

---

## Why This Pattern

- **Sequence diagram only** — The flow is a single linear path: invoke command, resolve, execute, return. No branching, no state changes.
- **No State Model** — The user doesn't pause, resume, or manage lifecycle states. The flow runs to completion.
- **No Decision Paths** — There are no approve/reject or if/then branch points.
- **No Related Capabilities** — This capability stands alone; it doesn't chain to other capabilities.
