# Getting Started with @specforge/product

The `@specforge/product` extension adds 9 planning entity kinds to SpecForge: **feature**, **journey**, **deliverable**, **milestone**, **module**, **term**, **persona**, **channel**, and **release**. Together they model the full product planning lifecycle.

## Quick start

### 1. Install the extension

```bash
specforge init --extensions @specforge/product
```

Or add to an existing project:

```bash
specforge add @specforge/product
```

### 2. Define your personas

Start by declaring who uses your product:

```specforge
persona developer "Developer" {
  description "Software engineer building features"
  technical_level expert
  goals ["Ship features quickly", "Maintain code quality"]
  tags [primary]
}
```

### 3. Define features

Features describe what your product does in problem/solution terms:

```specforge
feature user_auth "User Authentication" {
  problem  "Users cannot access personalized content without identity"
  solution "OAuth2 login flow with email/password fallback"
  priority high
  status   in_progress
  effort   m
  owner    "alice"
  acceptance [
    "User can sign in with Google OAuth",
    "User can sign in with email/password",
  ]
  tags [auth, mvp]
}
```

### 4. Map journeys

Journeys describe how personas interact with features:

```specforge
journey sign_in "Sign In" {
  persona  developer
  channels [web_app]
  features [user_auth]
  priority high
  flow [
    "1. User navigates to login page",
    "2. User clicks Sign in with Google [user_auth]",
    "3. OAuth redirect completes",
    "4. User lands on dashboard",
  ]
}
```

### 5. Schedule into milestones

```specforge
milestone mvp "Minimum Viable Product" {
  status      in_progress
  features    [user_auth]
  start_date  "2026-01-15"
  target_date "2026-03-31"
  owner       "alice"
  exit_criteria [
    "All MVP features done",
    "Zero E-level diagnostics",
  ]
}
```

### 6. Define deliverables

```specforge
deliverable web_app "Web Application" {
  artifact_type web_app
  status        draft
  journeys      [sign_in]
  modules       [auth_module, web_frontend]
  milestones    [mvp]
  owner         "bob"
}
```

### 7. Coordinate releases

```specforge
release v1 "Version 1.0" {
  version      "1.0.0"
  status       planned
  deliverables [web_app]
  milestones   [mvp]
}
```

### 8. Validate and query

```bash
specforge check                              # Validate spec
specforge list-features --status=in_progress # Filter features
specforge milestone-completion mvp           # Check progress
specforge weighted-milestone-completion mvp  # Effort-weighted
specforge owner-workload                     # Who owns what
specforge product-health                     # Overall health
specforge unscheduled-features               # Find gaps
```

## Progressive adoption

You do not need all 9 entity kinds on day one:

| Start with | When you need |
|------------|---------------|
| `feature` only | Just tracking what to build |
| + `milestone` | Scheduling features into phases |
| + `journey` + `persona` | Understanding who uses what |
| + `deliverable` + `module` | Mapping features to code structure |
| + `release` | Coordinating multi-deliverable shipping |
| + `term` + `channel` | Full vocabulary and surface coverage |

Every field is optional. Every entity kind is optional. SpecForge validates what you have and suggests what is missing.

## Key concepts

### Traceability chain

```
persona -> journey -> feature -> module -> deliverable -> release
               |                    |
            channel             milestone
```

Every arrow is a validated graph edge. Orphan detection finds disconnected entities.

### Effort estimation

Features support t-shirt sizing: `xs`, `s`, `m`, `l`, `xl`. The `weighted-milestone-completion` query uses Fibonacci weights (1, 2, 3, 5, 8).

### Ownership

Add `owner` and `contributors` to any feature, milestone, deliverable, or release. Use `owner-workload` to see aggregate assignments.

### Health score

`specforge product-health` returns a composite score (0.0-1.0):
- Milestone completion (30%)
- Journey coverage (25%)
- Orphan ratio (20%)
- Cycle count (15%)
- Error ratio (10%)

## CLI commands

| Command | Description |
|---------|-------------|
| `list-features` | List features with filtering |
| `list-journeys` | List journeys |
| `list-milestones` | List milestones |
| `list-deliverables` | List deliverables |
| `list-modules` | List modules |
| `list-terms` | List glossary terms |
| `list-personas` | List personas |
| `list-channels` | List channels |
| `list-releases` | List releases |
| `milestone-completion <id>` | Completion ratio |
| `weighted-milestone-completion <id>` | Effort-weighted completion |
| `journey-coverage <id>` | Journey feature coverage |
| `feature-ordering` | Topological feature sort |
| `milestone-timeline` | Chronological milestone view |
| `product-health` | Composite health metric |
| `owner-workload` | Ownership statistics |
| `graph-diff` | Compare snapshots |
| `unscheduled-features` | Features not in any milestone |
| `feature-overlap` | Features in multiple deliverables |
| `coverage-matrix` | Per-persona feature coverage |
| `critical-path` | Longest incomplete chain |
| `bulk-status` | Batch status updates |
| `snapshots` | List graph snapshots |

All commands support `--format=json|table|brief` and are auto-promoted to MCP tools.
