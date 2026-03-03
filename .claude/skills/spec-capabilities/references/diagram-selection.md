# Diagram Selection Reference

Guide for choosing and authoring Mermaid diagrams in capability specification files.

## Diagram Type Selection Matrix

| Capability Pattern | Diagrams Required | Detection Heuristic | Examples |
|---|---|---|---|
| **Simple CLI command** | Sequence only | Single linear path, no branching, no state changes | Run a flow, list items, export data |
| **Dashboard / monitoring** | Sequence only | Read-only views, no user decisions | View history, monitor health, inspect logs |
| **Stateful lifecycle** | Sequence + State | Steps mention pause/resume/cancel, activate/deactivate, or multiple statuses | Pause/resume flow, enable/disable plugin, activate compliance mode |
| **Branching decision** | Sequence + Decision Flowchart | Steps contain approve/reject, if/then, or alternative paths | Approve phase transition, review and accept/reject, choose route |
| **Multi-step chain** | Sequence + Related Capabilities table | Capability references or depends on other capabilities | Onboarding wizard, multi-phase setup |

### Decision Tree

```
Does the capability involve state changes (pause/resume/cancel/activate)?
├── Yes → Include State Diagram
│         Does it also involve user decisions (approve/reject)?
│         ├── Yes → Sequence + State + Flowchart (rare, all three)
│         └── No  → Sequence + State
└── No  → Does it involve user decisions (approve/reject/if-then)?
          ├── Yes → Sequence + Flowchart
          └── No  → Sequence only
```

## Mermaid Syntax Patterns

### Sequence Diagram — Simple Linear Flow

```text
┌───────────┐    ┌─────┐    ┌────────────┐
│ Developer │    │ CLI │    │ FlowEngine │
└─────┬─────┘    └──┬──┘    └─────┬──────┘
      │  run cmd    │             │
      │────────────►│  resolve    │
      │             │────────────►│
      │             │  FlowDef    │
      │             │◄────────────│
      │             │  execute    │
      │             │────────────►│
      │             │  result     │
      │  summary    │◄────────────│
      │◄────────────│             │
```

```mermaid
sequenceDiagram
    actor Dev as Developer
    participant CLI
    participant Engine as FlowEngine

    Dev->>+CLI: specforge run spec-verify
    CLI->>+Engine: resolveFlow("spec-verify")
    Engine-->>-CLI: FlowDefinition
    CLI->>+Engine: execute(flow, context)
    Engine-->>-CLI: FlowResult{status: completed}
    CLI-->>-Dev: Summary + exit code 0
```

**Key patterns:**
- `actor` for the primary persona
- `participant X as Label` for aliased names
- `->>+` and `-->>-` for activation spans (shows processing time)
- Solid arrows (`->>`) for requests, dashed arrows (`-->>`) for responses

### Sequence Diagram — With Branching (alt/else)

```text
┌───────────┐    ┌───────────┐    ┌────────────┐
│ Team Lead │    │ Dashboard │    │ FlowEngine │
└─────┬─────┘    └─────┬─────┘    └─────┬──────┘
      │                │  Approval       │
      │                │◄────────────────│
      │  notification  │                 │
      │◄───────────────│                 │
      │  review        │                 │
      │───────────────►│                 │
      │  summary       │                 │
      │◄───────────────│                 │
      │                │                 │
      │  [if approved] │                 │
      │  approve       │                 │
      │───────────────►│  approve        │
      │                │────────────────►│
      │                │  PhaseAdvanced  │
      │  proceeding    │◄────────────────│
      │◄───────────────│                 │
      │                │                 │
      │  [if rejected] │                 │
      │  reject        │                 │
      │───────────────►│  reject         │
      │                │────────────────►│
      │                │  PhaseReentered │
      │  re-entered    │◄────────────────│
      │◄───────────────│                 │
```

```mermaid
sequenceDiagram
    actor Lead as Team Lead
    participant Dash as Dashboard
    participant Engine as FlowEngine

    Engine->>Dash: ApprovalRequired event
    Dash->>Lead: Notification: review needed

    Lead->>+Dash: Review phase outputs
    Dash-->>-Lead: Phase summary + metrics

    alt Approved
        Lead->>+Dash: Approve
        Dash->>+Engine: approve(runId)
        Engine-->>-Dash: PhaseAdvanced
        Dash-->>-Lead: Flow proceeding
    else Rejected
        Lead->>+Dash: Reject with feedback
        Dash->>+Engine: reject(runId, feedback)
        Engine-->>-Dash: PhaseReentered
        Dash-->>-Lead: Phase re-entered with feedback
    end
```

**Key patterns:**
- `alt`/`else`/`end` for mutually exclusive paths
- Each branch represents a user decision

### Sequence Diagram — With Optional Steps (opt)

```text
┌───────────┐    ┌─────┐    ┌────────────┐
│ Developer │    │ CLI │    │ FlowEngine │
└─────┬─────┘    └──┬──┘    └─────┬──────┘
      │  run cmd    │             │
      │────────────►│  execute    │
      │             │────────────►│
      │             │  result     │
      │             │◄────────────│
      │             │             │
      │  [opt: drift detected]    │
      │  warning     │            │
      │◄─────────────│            │
      │  run --fix   │            │
      │─────────────►│  execute   │
      │              │───────────►│
      │              │  FixResult │
      │  fixed 3     │◄───────────│
      │◄─────────────│            │
      │  [end opt]   │            │
      │              │            │
      │  summary     │            │
      │◄─────────────│            │
```

```mermaid
sequenceDiagram
    actor Dev as Developer
    participant CLI
    participant Engine as FlowEngine

    Dev->>+CLI: specforge run drift-check
    CLI->>+Engine: execute(flow)
    Engine-->>-CLI: FlowResult

    opt Drift detected
        CLI->>Dev: Warning: 3 drift items found
        Dev->>+CLI: specforge run drift-check --fix
        CLI->>+Engine: execute(flow, {autoFix: true})
        Engine-->>-CLI: FixResult
        CLI-->>-Dev: Fixed 3 items
    end

    CLI-->>-Dev: Summary
```

### Sequence Diagram — With Loops

```text
┌───────────┐    ┌─────┐    ┌────────────┐
│ Developer │    │ CLI │    │ FlowEngine │
└─────┬─────┘    └──┬──┘    └─────┬──────┘
      │  run cmd    │             │
      │────────────►│  execute    │
      │             │────────────►│
      │             │             │
      │  [loop: until convergence]│
      │             │  progress   │
      │  progress   │◄────────────│
      │◄────────────│             │
      │  [end loop] │             │
      │             │             │
      │             │  result     │
      │  complete   │◄────────────│
      │◄────────────│             │
```

```mermaid
sequenceDiagram
    actor Dev as Developer
    participant CLI
    participant Engine as FlowEngine

    Dev->>+CLI: specforge run code-review
    CLI->>+Engine: execute(flow)

    loop Until convergence or max iterations
        Engine->>Engine: Run agent iteration
        Engine-->>CLI: IterationProgress{n, convergence%}
        CLI-->>Dev: Progress: iteration N, convergence X%
    end

    Engine-->>-CLI: FlowResult
    CLI-->>-Dev: Review complete
```

### State Diagram — Lifecycle States

```text
                                  · Checkpoint saved
         start       pause       ·
[*] ───► Idle ───► Running ───► Paused
                     │  │           │
                     │  │  cancel   │ cancel
                     │  └─────┐     │
                     │ finish │     │
                     ▼        ▼     ▼
                 Completed  Cancelled
                     │          │  · Cleanup hooks run
                     ▼          ▼
                    [*]        [*]
```

```mermaid
stateDiagram-v2
    direction LR
    [*] --> Idle
    Idle --> Running: start
    Running --> Paused: pause
    Paused --> Running: resume
    Running --> Completed: finish
    Running --> Cancelled: cancel
    Paused --> Cancelled: cancel
    Completed --> [*]
    Cancelled --> [*]

    note right of Paused: Checkpoint saved
    note right of Cancelled: Cleanup hooks run
```

**Key patterns:**
- `direction LR` for left-to-right layout
- `[*]` for initial and final pseudo-states
- `note right of State: text` for guard conditions or clarifications
- State names match the statuses referenced in Steps

### State Diagram — Nested States

```text
                  enable
[*] ───► Disabled ─────► Enabled
                         │ disable
                         │◄──────┐
                         │       │
                    ┌────────────────┐
                    │  [*] ──► Active │
                    │    suspend ↓ ↑  │
                    │   Suspended ──┘ │
                    │    reactivate   │
                    └────────────────┘
```

```mermaid
stateDiagram-v2
    direction LR
    [*] --> Disabled

    state Enabled {
        [*] --> Active
        Active --> Suspended: suspend
        Suspended --> Active: reactivate
    }

    Disabled --> Enabled: enable
    Enabled --> Disabled: disable
```

### Decision Flowchart — Binary Choice

```text
┌───────────────────────┐
│ Review phase outputs  │
└───────────┬───────────┘
            ▼
      ┌──────────┐
     ╱  Approve?  ╲
    ╱              ╲
 Yes                No
  │                  │
  ▼                  ▼
┌──────────────┐  ┌────────────────────────┐
│ Flow proceeds│  │ Provide rejection      │
│ to next phase│  │ feedback               │
└──────────────┘  └───────────┬────────────┘
                              ▼
                  ┌────────────────────────┐
                  │ Phase re-entered       │
                  │ with feedback          │
                  └────────────────────────┘
```

```mermaid
flowchart TD
    A[Review phase outputs] --> B{Approve?}
    B -->|Yes| C([Flow proceeds to next phase])
    B -->|No| D[Provide rejection feedback]
    D --> E([Phase re-entered with feedback])
```

**Key patterns:**
- `flowchart TD` for top-down layout
- `{Decision}` diamond for user choices
- `([Outcome])` stadium shape for terminal outcomes
- `[Action]` rectangle for intermediate steps
- Edge labels with `|Label|` syntax

### Decision Flowchart — Multi-Branch

```text
┌──────────────────────────┐
│ Review plugin manifest   │
└────────────┬─────────────┘
             ▼
     ┌───────────────┐
    ╱  Plugin type?   ╲
   ╱                   ╲
  ╱         │           ╲
Flow      Agent       Adapter
  │         │           │
  ▼         ▼           ▼
┌────────┐┌──────────┐┌────────────┐
│Validate││ Validate ││  Validate  │
│flow    ││ agent    ││  port      │
│schema  ││ iface    ││  compat    │
└───┬────┘└────┬─────┘└─────┬──────┘
    └──────────┼────────────┘
               ▼
         ┌──────────┐
        ╱   Valid?   ╲
       ╱              ╲
    Yes                No
     │                  │
     ▼                  ▼
┌────────────┐  ┌────────────────────┐
│  Plugin    │  │ Show validation    │
│  installed │  │ errors             │
└────────────┘  └─────────┬──────────┘
                          ▼
                ┌────────────────────┐
                │ Installation       │
                │ aborted            │
                └────────────────────┘
```

```mermaid
flowchart TD
    A[Review plugin manifest] --> B{Plugin type?}
    B -->|Flow plugin| C[Validate flow schema]
    B -->|Agent plugin| D[Validate agent interface]
    B -->|Adapter plugin| E[Validate port compatibility]
    C --> F{Valid?}
    D --> F
    E --> F
    F -->|Yes| G([Plugin installed])
    F -->|No| H[Show validation errors]
    H --> I([Installation aborted])
```

## Persona and Surface Conventions

### Persona Names (use in `actor` declarations)

| Persona | Mermaid Actor | Description |
|---------|--------------|-------------|
| `developer` | `actor Dev as Developer` | Individual contributor using CLI/IDE |
| `team-lead` | `actor Lead as Team Lead` | Reviewer, approver, team manager |
| `platform-admin` | `actor Admin as Platform Admin` | System configuration, org-level settings |
| `ci-bot` | `actor CI as CI Pipeline` | Automated pipeline agent |

### Surface Names (use in `participant` declarations)

| Surface | Mermaid Participant | Description |
|---------|-------------------|-------------|
| `cli` | `participant CLI` | Command-line interface |
| `dashboard` | `participant Dash as Dashboard` | Web dashboard UI |
| `desktop` | `participant App as Desktop App` | Desktop application |
| `ide` | `participant IDE as IDE Extension` | Editor/IDE integration |
| `api` | `participant API as REST API` | External API surface |

### Backend Component Names

Name backend participants after the actual system component:

```
participant Engine as FlowEngine
participant Store as GraphStore
participant Agent as AgentBackend
participant Registry as FlowRegistry
participant Scheduler as TaskScheduler
```

## ASCII Conventions

Every `mermaid` code block MUST be preceded by an equivalent `text` code block using Unicode box-drawing characters. This ensures readability in plain-text contexts (terminals, git diffs, non-rendering editors).

### Box-Drawing Characters

| Character | Usage |
|-----------|-------|
| `─` | Horizontal lines, arrow shafts |
| `│` | Vertical lines, lifelines |
| `┌ ┐ └ ┘` | Box corners |
| `►` | Request arrow tip (right) |
| `◄` | Response arrow tip (left) |
| `▼` | Downward arrow tip |
| `▲` | Upward arrow tip |
| `╱ ╲` | Diamond approximation for decisions |
| `·` | Dotted lines for async / notes |

### Sequence Diagram Pattern

```text
┌──────────┐    ┌──────┐    ┌────────────┐
│ Persona  │    │ CLI  │    │  Backend   │
└────┬─────┘    └──┬───┘    └─────┬──────┘
     │  action     │              │
     │────────────►│  operation   │
     │             │─────────────►│
     │             │  response    │
     │  feedback   │◄─────────────│
     │◄────────────│              │
```

### State Diagram Pattern

```text
         start       pause
[*] ───► Idle ───► Running ───► Paused
                     │              │
                     │ finish       │ resume
                     ▼              └───► Running
                  Completed ───► [*]
```

### Flowchart Pattern

```text
┌─────────────────┐
│ Action step     │
└────────┬────────┘
         ▼
   ┌──────────┐
  ╱  Decision? ╲
 ╱              ╲
Yes              No
 │                │
 ▼                ▼
┌──────────┐  ┌──────────┐
│ Outcome A│  │ Outcome B│
└──────────┘  └──────────┘
```

### Rules

1. Place `text` block immediately before the corresponding `mermaid` block
2. Keep width under 80 characters where possible
3. Match the content of the Mermaid diagram — same participants, messages, states
4. Use `[if condition]` / `[else]` labels for alt/else branches in sequences
5. Use `[opt: condition]` / `[end opt]` for optional blocks
6. Use `[loop: condition]` / `[end loop]` for loop blocks
