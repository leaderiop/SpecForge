# Example: Stateful Lifecycle Capability

This example shows a **stateful lifecycle** capability — pause/resume/cancel pattern requiring both sequence and state diagrams.

---

```markdown
---
id: UX-SF-005
kind: capability
title: "Pause, Resume, and Cancel a Flow"
status: active
features: [FEAT-SF-004, FEAT-SF-009]
behaviors: [BEH-SF-065, BEH-SF-066, BEH-SF-113]
persona: [developer]
surface: [cli, dashboard]
---

# Pause, Resume, and Cancel a Flow

## Use Case

A developer needs to interrupt a running flow — perhaps to review intermediate results before proceeding, to free up resources, or to abort a run that is consuming too many tokens. The system supports graceful pause (completing the current agent turn), resume (continuing from the paused state), and cancel (terminating with cleanup).

## Interaction Flow

` ``text
┌───────────┐    ┌─────┐    ┌────────────┐
│ Developer │    │ CLI │    │ FlowEngine │
└─────┬─────┘    └──┬──┘    └─────┬──────┘
      │  run cmd    │             │
      │────────────►│  execute    │
      │             │────────────►│
      │             │  progress   │
      │             │◄────────────│
      │  pause      │             │
      │────────────►│  pause      │
      │             │────────────►│
      │             │  complete   │
      │             │  turn ───┐  │
      │             │       ◄──┘  │
      │             │  FlowPaused │
      │  paused     │◄────────────│
      │◄────────────│             │
      │  resume     │             │
      │────────────►│  resume     │
      │             │────────────►│
      │             │  resumed    │
      │             │◄────────────│
      │             │  progress   │
      │             │◄────────────│
      │             │  result     │
      │  summary    │◄────────────│
      │◄────────────│             │
` ``

` ``mermaid
sequenceDiagram
    actor Dev as Developer
    participant CLI
    participant Engine as FlowEngine

    Dev->>+CLI: specforge run spec-verify
    CLI->>+Engine: execute(flow)
    Engine-->>CLI: ProgressEvent{phase: 1}

    Dev->>CLI: specforge pause <run-id>
    CLI->>Engine: pause(runId)
    Engine->>Engine: Complete current agent turn
    Engine-->>CLI: FlowPaused{checkpoint: saved}
    CLI-->>Dev: Flow paused at phase 1

    Dev->>CLI: specforge resume <run-id>
    CLI->>Engine: resume(runId)
    Engine-->>CLI: FlowResumed{phase: 1}
    Engine-->>CLI: ProgressEvent{phase: 2}
    Engine-->>-CLI: FlowResult{status: completed}
    CLI-->>-Dev: Summary + exit code 0
` ``

## Steps

1. Start a flow normally: `specforge run <flow-name>`
2. Pause: `specforge pause <run-id>` or click Pause in the dashboard
3. System completes the current agent turn and checkpoints state (BEH-SF-065)
4. Flow enters `paused` state; agents are suspended
5. Resume: `specforge resume <run-id>` to continue from checkpoint
6. System restores state and resumes from the paused phase (BEH-SF-066)
7. Cancel: `specforge cancel <run-id>` to terminate
8. System runs cleanup hooks and records partial results (BEH-SF-113)

## State Model

` ``text
                              · Checkpoint saved to disk
         start       pause   ·        resume
[*] ───► Idle ───► Running ───► Paused ───► Running
                     │  │           │
                     │  │  cancel   │ cancel
                     │  └─────┐     │
                     │ finish │     │
                     ▼        ▼     ▼
                 Completed  Cancelled
                     │          │  · Cleanup hooks executed
                     ▼          ▼
                    [*]        [*]
` ``

` ``mermaid
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

    note right of Paused: Checkpoint saved to disk
    note right of Cancelled: Cleanup hooks executed
` ``

## Traceability

| Behavior | Feature | Role in this capability |
|----------|---------|------------------------|
| BEH-SF-065 | FEAT-SF-004 | Graceful pause with state checkpointing |
| BEH-SF-066 | FEAT-SF-004 | Resume from checkpointed state |
| BEH-SF-113 | FEAT-SF-009 | CLI commands for pause/resume/cancel |
```

---

## Why This Pattern

- **Sequence + State diagrams** — The capability involves explicit lifecycle states (Idle, Running, Paused, Completed, Cancelled) with user-triggered transitions.
- **State Model section included** — Steps mention "pause", "resume", "cancel" and the flow has multiple distinct statuses.
- **No Decision Paths** — While there are multiple actions, they are all direct commands, not conditional approve/reject decisions.
- **Detection heuristic match** — The title and steps contain "pause/resume/cancel", which triggers the stateful lifecycle pattern.
