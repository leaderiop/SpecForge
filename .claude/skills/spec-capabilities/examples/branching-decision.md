# Example: Branching Decision Capability

This example shows a **branching decision** capability — approve/reject pattern requiring both sequence and decision flowchart diagrams.

---

```markdown
---
id: UX-SF-018
kind: capability
title: "Approve or Reject a Phase Transition"
status: active
features: [FEAT-SF-018, FEAT-SF-017]
behaviors: [BEH-SF-123, BEH-SF-124, BEH-SF-133]
persona: [team-lead]
surface: [cli, dashboard]
---

# Approve or Reject a Phase Transition

## Use Case

When a flow reaches a phase boundary that requires human approval (configured as an approval gate), the system pauses and notifies the designated reviewer. The team lead reviews the phase's outputs and either approves (allowing the flow to proceed) or rejects (sending it back for another iteration with feedback).

## Interaction Flow

` ``text
┌───────────┐    ┌───────────┐    ┌────────────┐
│ Team Lead │    │ Dashboard │    │ FlowEngine │
└─────┬─────┘    └─────┬─────┘    └─────┬──────┘
      │                │  Approval       │
      │                │◄────────────────│
      │  notification  │                 │
      │◄───────────────│                 │
      │  open review   │                 │
      │───────────────►│                 │
      │  outputs       │                 │
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
` ``

` ``mermaid
sequenceDiagram
    actor Lead as Team Lead
    participant Dash as Dashboard
    participant Engine as FlowEngine

    Engine->>Dash: ApprovalRequired{runId, phase: 2}
    Dash->>Lead: Notification: phase review needed

    Lead->>+Dash: Open phase review
    Dash-->>-Lead: Phase outputs + convergence metrics

    alt Approved
        Lead->>+Dash: Click "Approve"
        Dash->>+Engine: approve(runId)
        Engine-->>-Dash: PhaseAdvanced{phase: 3}
        Dash-->>-Lead: Flow proceeding to phase 3
    else Rejected with feedback
        Lead->>+Dash: Click "Reject", enter feedback
        Dash->>+Engine: reject(runId, "Insufficient coverage analysis")
        Engine-->>-Dash: PhaseReentered{phase: 2, feedback: attached}
        Dash-->>-Lead: Phase 2 re-entered with feedback
    end
` ``

## Steps

1. Flow reaches an approval gate; system pauses and sends notification
2. Team lead receives notification via CLI prompt or dashboard alert
3. Review the phase's outputs, agent findings, and convergence metrics (BEH-SF-133)
4. Approve: `specforge approve <run-id>` (flow proceeds to next phase) (BEH-SF-123)
5. Or reject with feedback: `specforge reject <run-id> "Insufficient coverage analysis"` (BEH-SF-124)
6. On rejection, flow re-enters the phase with the feedback as additional context
7. Approval/rejection decision is recorded in the audit trail

## Decision Paths

` ``text
┌─────────────────────────────┐
│ Flow reaches approval gate  │
└──────────────┬──────────────┘
               ▼
     ┌─────────────────┐
     │ Review outputs   │
     └────────┬────────┘
              ▼
        ┌──────────┐
       ╱  Approve?  ╲
      ╱              ╲
  Yes╱                ╲No
    ▼                  ▼
┌────────────┐  ┌──────────────────┐
│ Flow       │  │ Enter rejection  │
│ advances   │  │ feedback         │
└────────────┘  └────────┬─────────┘
                         ▼
                ┌──────────────────┐
                │ Phase re-entered │
                │ with feedback    │
                └──────────────────┘
` ``

` ``mermaid
flowchart TD
    A[Flow reaches approval gate] --> B[Team lead reviews outputs]
    B --> C{Approve phase?}
    C -->|Yes| D([Flow advances to next phase])
    C -->|No| E[Enter rejection feedback]
    E --> F([Phase re-entered with feedback])
` ``

## Traceability

| Behavior | Feature | Role in this capability |
|----------|---------|------------------------|
| BEH-SF-123 | FEAT-SF-018 | Approval gate mechanics and phase advancement |
| BEH-SF-124 | FEAT-SF-018 | Rejection handling with feedback loop |
| BEH-SF-133 | FEAT-SF-017 | Dashboard review interface for phase outputs |
```

---

## Why This Pattern

- **Sequence + Decision Flowchart** — The capability has a clear binary decision point (approve/reject) that determines the flow's path.
- **Decision Paths section included** — Steps contain "approve or reject" with alternative outcomes.
- **`alt`/`else` in sequence diagram** — The sequence diagram uses `alt`/`else` to show both paths in the interaction timeline.
- **Flowchart for decision clarity** — The flowchart provides a quick visual overview of the decision tree, complementing the more detailed sequence diagram.
- **No State Model** — While the flow has states, the capability itself is about a single decision point, not lifecycle management. The flow's state model belongs to a different capability (UX-SF-005).
