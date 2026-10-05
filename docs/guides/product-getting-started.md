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
specforge check                                      # Validate spec
specforge product features --status in_progress      # Filter features
specforge product milestone-completion mvp           # Check progress
specforge product journey-coverage sign_in           # Features covered by modules
specforge product feature-impact user_auth           # What references a feature
specforge product health                             # Overall health
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

Features support t-shirt sizing: `xs`, `s`, `m`, `l`, `xl`, set with the `effort` field.

### Ownership

Add `owner` and `contributors` to any feature, milestone, deliverable, or release.

### Health score

`specforge product health` returns a composite score (0-100), the mean of:
- Coverage: the share of product entities with at least one reference in or out
- Connectivity: references relative to the most the entities could have
- Completeness: features with a `status` and milestones that list features

## CLI commands

The commands are `@specforge/product`'s own (`specforge product <command>`, or
`specforge product:<command>`); `specforge product --help` lists them.

| Command | Description |
|---------|-------------|
| `features` | List features (`--status`, `--priority`, `--offset`, `--limit`) |
| `journeys` | List journeys |
| `deliverables` | List deliverables (`--status`) |
| `milestones` | List milestones (`--status`) |
| `modules` | List modules |
| `terms` | List glossary terms |
| `personas` | List personas |
| `channels` | List channels |
| `releases` | List releases (`--status`) |
| `milestone-completion <milestone>` | Completion ratio of a milestone's features |
| `journey-coverage <journey>` | Share of a journey's features some module contains |
| `feature-impact <feature>` | Journeys, milestones, modules and features referencing a feature |
| `feature-dependents <feature>` | Features that depend on a feature |
| `persona-features <persona>` | Features reachable from a persona through its journeys |
| `channel-features <channel>` | Features reachable from a channel through its journeys |
| `bulk-status` | Status breakdown per kind |
| `health` | Composite health score |

Every command takes `--path <dir>` (the project, default `.`) and `--format human|json`
(default `human`), and is auto-promoted to the MCP tool `specforge.product.<id>`
(`specforge.product.milestone_completion`).
