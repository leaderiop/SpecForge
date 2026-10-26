# Getting Started with @specforge/product

`@specforge/product` is a builtin extension: it teaches SpecForge 9 planning kinds — **feature**, **journey**, **deliverable**, **milestone**, **module**, **term**, **persona**, **channel**, and **release** — and about 40 commands that answer planning questions over them. Few of its words are about code (the fields that classify things, such as a deliverable's `artifact_type`, take any word), so a marketing team launching a campaign or a clinic planning a new service can use it too. Together the kinds model the planning lifecycle: who it is for, what they need, how they get it, and when it ships.

## Quick start

### 1. Install the extension

```bash
specforge init --extensions @specforge/product
```

Or add to an existing project:

```bash
specforge add @specforge/product
```

### 2. Define your personas and channels

Start by declaring who uses your product, and where they meet it:

```spec
persona developer "Developer" {
  description "Software engineer building features"
  technical_level expert
  goals ["Ship features quickly", "Maintain code quality"]
  tags ["primary"]
}

channel web "Web" {
  description       "The browser application"
  interaction_model request_response
}
```

### 3. Define features

Features describe what your product does in problem/solution terms:

```spec
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
  tags ["auth", "mvp"]
}
```

### 4. Map journeys

Journeys describe how personas interact with features:

```spec
journey sign_in "Sign In" {
  persona  developer
  channels [web]
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

```spec
milestone mvp "Minimum Viable Product" {
  status      in_progress
  features    [user_auth]
  start_date  2026-01-15
  target_date 2026-03-31
  owner       "alice"
  exit_criteria [
    "All MVP features done",
    "Zero E-level diagnostics",
  ]
}
```

### 6. Define modules and deliverables

```spec
module auth_module "Authentication" {
  family   core
  features [user_auth]
}

module web_frontend "Web frontend" {
  family   core
  features [user_auth]
}

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

```spec
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

Every arrow is a validated graph edge. Unreferenced detection finds disconnected entities.

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
`specforge product:<command>`); `specforge product --help` lists all of them. The most used:

| Command | Description |
|---------|-------------|
| `features` | List features (`--status`, `--priority`, `--tags`, `--sort-by`, `--offset`, `--limit`) |
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

The rest answer finer questions: `deliverable-traceability`, `feature-deliverables`,
`persona-channels`, `deliverable-personas`, `deliverable-completion`, `release-completion`,
`deliverable-priority`, `unscheduled-features`, `owner-workload`, `feature-ordering`,
`critical-path`, `module-depth`, `module-coupling`, `deliverable-dependents`,
`coverage-matrix`, `channel-coverage-matrix`, `feature-overlap`, `term-graph`,
`term-clusters`, `term-density`, `milestone-timeline`, `milestone-velocity` and
`weighted-milestone-completion`.

Every command takes `--path <dir>` (the project, default `.`) and `--format human|json`
(default `human`), and is served to agents as the MCP tool `specforge.product.<id>`
(`specforge.product.milestone_completion`), so an agent can ask the same questions — see
[Connect an agent](connect-an-agent.md).
