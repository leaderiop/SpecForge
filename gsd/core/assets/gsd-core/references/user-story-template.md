<!-- Ported from gsd-core v1.19.0 gsd-core/references/user-story-template.md, MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD -->
# User Story Template (MVP Mode)

> Used by `mvp-phase` workflow and `gsd-planner` agent when `MVP_MODE=true`. Defines the canonical "As a / I want to / So that" format and the rules for converting it into the phase's `goal` field.

## Canonical format

```
As a [user role], I want to [capability], so that [outcome].
```

Three required components:

| Slot | Question | Examples |
|---|---|---|
| `[user role]` | Who is the actor? | "new user", "admin", "signed-in customer", "API consumer" |
| `[capability]` | What can they do? | "register and log in", "upload a CSV", "see my dashboard" |
| `[outcome]` | Why does it matter? | "I can access my account", "I can bulk-import contacts", "I can see at a glance what needs attention" |

All three must be present. Refuse to assemble a partial story.

## How it lands in ROADMAP.md

The full user story replaces the existing `**Goal:**` line in the phase section:

**Before:**
```
### Phase 1: User Auth MVP
**Goal:** Users can register and log in
```

**After:**
```
### Phase 1: User Auth MVP
**Goal:** As a new user, I want to register and log in, so that I can access my dashboard.
**Mode:** mvp
```

Two structural rules:
1. The `**Goal:**` line stays on a single line (no line breaks inside the story). If the story is longer than ~120 chars, it should be split into multiple phases via SPIDR (see `spidr-splitting.md`).
2. The `**Mode:** mvp` line is added immediately below `**Goal:**`. If `**Mode:**` already exists, it is replaced (not duplicated).

## How it lands in PLAN.md

With MVP_MODE=true, each plan's objective opens with the user story, as the `gsd-planner` agent writes it:

```markdown
## Phase Goal

**As a** new user, **I want to** register and log in, **so that** I can access my dashboard.

## Acceptance Criteria
- [ ] ...

## MVP Slice Tasks
...
```

Inside a plan, and in no other place, the three keywords stand in bold: `**As a**`, `**I want to**`, `**so that**`. The phase's goal in the roadmap keeps them plain, since that goal is already shown as one bold label.
