# Authoring `.spec` Files — Guidelines, Best Practices & Snippets

A practical guide to writing SpecForge `.spec` files. It covers the DSL syntax, conventions, and copy-paste-ready snippets for every builtin extension.

> **Mental model:** The graph is the product. You are not writing documentation — you are writing a *typed, validated graph* that AI agents consume. Every field that creates an edge (`invariants [...]`, `produces [...]`, `features [...]`) is what makes the spec valuable. Prefer explicit references over prose.

**Contents**
- [1. Getting started](#1-getting-started)
- [2. Core DSL syntax](#2-core-dsl-syntax)
- [3. General best practices](#3-general-best-practices)
- [4. `@specforge/software`](#4-specforgesoftware)
- [5. `@specforge/product`](#5-specforgeproduct)
- [6. `@specforge/governance`](#6-specforgegovernance)
- [7. `@specforge/formal`](#7-specforgeformal)
- [8. Validation workflow](#8-validation-workflow)

---

## 1. Getting started

Scaffold a project (interactive extension selection), then validate:

```bash
specforge init        # creates specforge.json + a starter hello.spec
specforge check       # validate all .spec files, report diagnostics
specforge export      # emit the typed graph for an agent to consume
```

A project is configured by **`specforge.json`** at the root:

```json
{
  "name": "payments",
  "version": "0.1.0",
  "spec_root": "spec",
  "extensions": ["@specforge/software"]
}
```

| Field | Required | Meaning |
|-------|----------|---------|
| `name` | yes | Project name |
| `version` | yes | Semver string |
| `spec_root` | no | Directory holding `.spec` files (default: `spec`) |
| `extensions` | no | Installed extension packages (empty = core engine only) |

Add or remove extensions later with `specforge add @specforge/product` / `specforge remove <name>`.

---

## 2. Core DSL syntax

The compiler carries **zero domain knowledge**. It only understands one universal shape:

```spec
keyword name "Optional Title" {
  field value
}
```

`keyword` (e.g. `behavior`, `feature`) and the legal fields come entirely from the installed extensions.

### Entity names

- Free-form identifiers: **letters, digits, underscores**, 2–60 chars, must start with a letter.
- **No sequential prefixes.** Use `create_user`, not `BEH-001`. (Older docs show `BEH-`/`FEAT-` patterns — those are obsolete.)
- Names must be **globally unique** across all `.spec` files in the project (duplicates → `E002`).
- The quoted title is optional; if omitted it's auto-derived (`auth_login` → "Auth Login").
- Pick one casing convention per project. `snake_case` for behaviors/events/invariants and `PascalCase` for types/ports is a common split.

### The `spec` block (project root)

Every project has exactly one `spec` block. Minimal:

```spec
spec "my-service" {
  version "0.1.0"
}
```

### Field value kinds

```spec
behavior example "Example" {
  status   draft                       // bare keyword / enum value
  category "auth"                       // string (quotes optional for single words)

  contract """                          // triple-quoted multi-line string
    When X happens, the system MUST do Y
    and MUST return a Result.
  """

  invariants [data_persistence]         // reference list -> creates graph edges
  produces   [user_logged_in]           // reference list -> creates graph edges

  refs       ["gh.issue:42"]            // string list (opaque values)
}
```

- **Reference lists** `[a, b]` are compiler-checked: every id must resolve to a real entity, or you get a diagnostic. This is what builds the graph.
- **String lists** `["...", "..."]` are opaque (exit criteria, test paths, etc.).
- **Triple-quoted strings** `"""..."""` preserve newlines — use them for contracts, guarantees, and descriptions.

### `verify` declarations

Declare *intent to test* an entity. The kind (`unit`, `integration`, `property`, …) is extension-declared.

```spec
verify unit        "rejects invalid password"
verify integration "user persists across restart"
verify property    "email uniqueness holds under concurrency"
```

### External references (`ref`)

Typed links to issues, tickets, designs. Pattern: `scheme.kind:identifier`.

```spec
ref gh.issue:42 "Track login timeout bug"
ref jira.epic:PROJ-123 "Q2 auth overhaul"

// then reference them from an entity:
behavior login "Login" {
  refs [gh.issue:42, jira.epic:PROJ-123]
}
```

### Imports (`use`)

To reference symbols defined in another file, import it (the `.spec` extension is implicit):

```spec
use invariants/data                       // import a file
use invariants/data { data_persistence }  // selective import
```

Circular imports → `E003`.

### Comments

Line comments only (`//`). Block comments are not supported.

```spec
// Section header
behavior login { ... }  // inline comment
```

### `define` meta-blocks

For user-defined entity types beyond what extensions provide, declare them in the `spec` block:

```spec
spec "my-service" {
  version "0.1.0"

  define research {
    id_prefix "RES"
    attributes {
      outcome     enum [adr, behavior, deferred, rejected]
      related_adr ref? decision
      date        string
    }
  }
}
```

---

## 3. General best practices

1. **One spec file already helps.** Coverage is a spectrum — you do not need to model everything before agents benefit. Start small.
2. **Maximize edges.** A behavior with `invariants`, `produces`, `types`, and `features` filled in is far more valuable to an agent than one with only a prose `contract`.
3. **Use RFC 2119 keywords** (MUST, MUST NOT, SHOULD, MAY) in contracts and guarantees — they are unambiguous for both humans and agents.
4. **One concept per entity.** Split "create and email the user" into two behaviors. Combine nothing.
5. **Always add at least one `verify`.** A behavior with no `verify` triggers a warning (`W004`).
6. **Organize files by kind**, e.g. `spec/behaviors/`, `spec/types/`, `spec/invariants/`. Use `use` to wire them together.
7. **Let diagnostics guide you.** Run `specforge check` often; `specforge explain E001` describes any code.
8. **Choose a casing convention and stick to it.**

---

## 4. `@specforge/software`

Five entity kinds for the universal specification chain. `use "@specforge/software"` implied once installed.

| Kind | Keyword | Purpose |
|------|---------|---------|
| behavior | `behavior` | A single operation's contract |
| type | `type` | Data shape (struct / union / error) |
| event | `event` | Domain event emitted/consumed by behaviors |
| port | `port` | Interface boundary (hexagonal) |
| invariant | `invariant` | A guarantee the system must never violate |

### behavior

| Field | Kind | Notes |
|-------|------|-------|
| title | string | positional, after id |
| `contract` | string | **the key field** — RFC 2119 contract |
| `category` | string | classification tag (e.g. `command`, `query`, `"auth"`) |
| `invariants` | ref list | → enforces invariant |
| `types` | ref list | → references type |
| `ports` | ref list | → uses port |
| `produces` | ref list | → emits event |
| `consumes` | ref list | → consumes event |
| `features` | ref list | → implements feature (requires `@specforge/product`) |
| `status`, `description`, `severity`, `refs` | — | optional metadata |

```spec
behavior create_user "Create User" {
  category   command
  invariants [data_persistence, email_uniqueness]
  types      [User, CreateUserCommand, DuplicateEmailError]
  ports      [UserRepository]
  produces   [user_created]

  contract """
    When a valid CreateUserCommand is received, the system MUST
    validate the email is unique, create a user record, and emit
    a user_created event. It MUST return Result<User, DuplicateEmailError>.
  """

  verify unit        "create user with unique email succeeds"
  verify unit        "duplicate email is rejected"
  verify integration "user persists after creation"
}
```

### type

Inferred kind from syntax: struct (`{ fields }`), union (`= a | b`), error (tagged). Field annotations: `@readonly`, `@unique`, `@optional`, `@literal`. Arrays: `Item[]`.

```spec
type User {
  id        string    @readonly
  email     string    @unique
  name      string
  role      UserRole
  createdAt timestamp @readonly
}

type UserRole = admin | editor | viewer

type DuplicateEmailError {
  _tag    "DuplicateEmailError" @literal
  email   string
  message string
}
```

**Best practice:** use `_tag` + `@literal` for discriminated unions; never put operations on a type — those belong on a `port`.

### event

Past-tense names. Payload is a **type reference**, not inline fields.

```spec
event user_created "User Created" {
  channel  "users.created"
  payload  UserCreatedPayload
  category "domain"

  verify integration "event emitted after user creation"
}
```

The producing/consuming relationship lives on the **behavior** (`produces`/`consumes`), not on the event.

### port

Interface boundary. `direction` is required: `inbound`, `outbound`, or `bidirectional`. All methods return `Result`.

```spec
use "types/user"

port UserRepository {
  direction outbound
  category  "persistence/user"

  method create(cmd: CreateUserCommand) -> Result<User, DuplicateEmailError>
  method findById(id: string)           -> Result<User, UserNotFoundError>
  method findByEmail(email: string)     -> Result<User?, never>
  method delete(id: string)             -> Result<void, UserNotFoundError>

  verify integration "UserRepository contract is satisfied"
}
```

Conventions: `{Entity}Repository` (persistence), `{Service}Gateway` (external), `{Domain}API` (inbound). Critical methods may carry `requires`/`ensures` blocks.

### invariant

A guarantee that must always hold. Fields: `guarantee` (required), `risk` (`low`/`medium`/`high`/`critical`), `description`, `refs`.

```spec
invariant data_persistence "Data Persistence" {
  guarantee """
    All committed writes survive process restarts.
    No acknowledged write may be silently dropped.
  """
  risk high

  verify property "committed writes survive a simulated crash"
}
```

The link between an invariant and the behaviors that uphold it lives on the **behavior** side (`invariants [...]`) — there is no `enforced_by` field on a builtin invariant; it's the inverse edge name. (The formal-methods `maintains`/`requires`/`ensures` blocks attach to behaviors, not invariants.)

**Invariant vs. others:** "users can log in" is a *behavior*; "we use PostgreSQL" is a *decision*; "p99 < 200ms" is a *constraint*. An invariant is a falsifiable, universal, implementation-independent guarantee.

---

## 5. `@specforge/product`

Nine entity kinds for product planning and ubiquitous language.

| Kind | Keyword | Purpose |
|------|---------|---------|
| feature | `feature` | User-facing capability (problem/solution) |
| journey | `journey` | persona × channels → features (UX flow) |
| persona | `persona` | A user archetype |
| channel | `channel` | A delivery surface (web, cli, api…) |
| module | `module` | Code package implementing features |
| milestone | `milestone` | Planning phase with exit criteria |
| deliverable | `deliverable` | Shippable artifact |
| release | `release` | Versioned bundle of deliverables/milestones |
| term | `term` | Glossary entry (ubiquitous language) |

> **Note:** `persona` and `channel` are **first-class top-level entity blocks** (not declarations inside the `spec` block). "surface" was renamed to "channel".

> `persona`, `channel`, `constraint`, and `term` all require a `description`/`definition` field.

```spec
persona developer "Software developer" {
  description     "A developer integrating with the system"
  technical_level expert
  status          active
}

channel cli "Command-line interface" {
  description "Terminal-based interaction surface"
  status      active
}

feature user_auth "User authentication" {
  status   proposed
  priority high
  problem  "Users need secure access to the system"
  solution "Password auth with bcrypt and rate-limited login."
}

journey onboarding "New user onboarding" {
  persona  developer
  channels [cli]
  features [user_auth]

  flow """
    1. Developer runs `app login`
    2. Prompted for credentials
    3. System validates and issues a token
  """
}

module core "Core module" {
  family   platform
  features [user_auth]
}

milestone mvp "Minimum Viable Product" {
  status        planned
  features      [user_auth]
  modules       [core]
  exit_criteria ["Core auth flow works end-to-end", "Coverage >= 90%"]
}

deliverable app "Application" {
  status        draft
  artifact_type cli
  journeys      [onboarding]
  modules       [core]
  milestones    [mvp]
}

term committed_write "committed write" {
  definition """
    A write acknowledged to the caller AND durably persisted.
    Guaranteed to survive process restarts.
  """
  context  "Used throughout the persistence subsystem"
  aliases  ["durable write", "acknowledged write"]
  // see_also references *other terms* only
}
```

**Tips:** `depends_on` lists on `module`/`milestone`/`deliverable` are DAG-validated (cycles → errors). A behavior points *up* to features via `features [...]`; a feature also lists `behaviors [...]`.

---

## 6. `@specforge/governance`

Three entity kinds for architecture governance.

```spec
decision postgres_over_mongodb "PostgreSQL over MongoDB" {
  status accepted
  date   2025-03-01
  context """
    We need a relational primary datastore with ACID transactions.
    The team has deep SQL expertise.
  """
  decision "Use PostgreSQL 15+ with typed schemas and row-level security."
  consequences [
    "Migrations required for schema changes",
    "Strong ACID guarantees",
  ]
  invariants [data_persistence]
}

constraint api_latency "API Latency Under Load" {
  category    performance
  priority    high
  description "API must stay responsive under sustained load"
  metric """
    response_time_p99 < 200ms at 1000 concurrent users, sustained 5 min
  """
  constrains [create_user]
  verify load "k6 with 1000 VUs, assert p99 < 200ms"
}

failure_mode write_lost "Write Acknowledged but Lost" {
  invariant  data_persistence
  severity   high
  occurrence unlikely
  detection  moderate
  cause      "Crash between ACK and fsync"
  effect     "Silent data loss — user believes write succeeded"
  mitigation "Write-ahead log with fsync before ACK"
}
```

- `decision` — ADR. `status`: proposed/accepted/deprecated/superseded. `context`, `decision`, `consequences` are the substance.
- `constraint` — non-functional requirement. Requires `description`. `category` (performance, security, …), `priority` (`critical`/`high`/`medium`/`low`), a measurable `metric`, and `constrains [...]` to link behaviors.
- `failure_mode` — FMEA entry. `severity`, `occurrence`, and `detection` are **enum words**, not 1–10 numbers:
  - `severity`: `critical` / `high` / `medium` / `low`
  - `occurrence`: `certain` / `likely` / `occasional` / `unlikely` / `rare`
  - `detection`: `certain` / `likely` / `moderate` / `unlikely` / `undetectable`

---

## 7. `@specforge/formal`

Five entity kinds for formal methods. Requires `@specforge/software` as a peer dependency and enhances its entities (behavior, event). Formal warnings only fire under `warning_level=strict`.

| Kind | Keyword | Purpose |
|------|---------|---------|
| property | `property` | Temporal assertion: safety / liveness / fairness |
| axiom | `axiom` | Assumed-true foundational truth (no coverage tracking) |
| protocol | `protocol` | Shared synchronization contract between processes |
| refinement | `refinement` | Maps an abstract behavior to a concrete one |
| process | `process` | CSP-style communicating process (alphabet, states, composition) |

```spec
property no_double_charge "No Double Charge" {
  property_type safety
  expression    "A payment is never captured more than once per order."
  scope         "payment subsystem"
}

refinement abstract_to_sql "Abstract create_user -> SQL" {
  abstract_entity  create_user
  concrete_entity  create_user_postgres
  invariant_deltas ["adds: unique index enforces email_uniqueness"]
}
```

> Conditions (`requires`/`ensures`/`maintains`) are **not** standalone entities — they are inline fields on behaviors/invariants/port methods.

---

## 8. Validation workflow

```bash
specforge check                 # errors + warnings
specforge check --strict        # warnings become errors (CI gate)
specforge check --lint=pedantic # also surface info-level diagnostics
specforge explain W004          # what does a code mean?
specforge trace create_user     # traceability chain for an entity
specforge query create_user --depth 2   # neighborhood at a given zoom level
specforge model                 # render the logical data model
specforge format                # auto-format .spec files
```

Diagnostic severities: **E** (error, blocks) · **W** (warning) · **I** (info, pedantic only). Each builtin extension owns a documented range of codes — see [docs/README.md](../README.md) and [docs/entity-model.md](../entity-model.md).

---

**See also:** the per-kind skills under `.claude/skills/specforge-*-dsl/` carry the authoritative field lists and deeper guidance, and `specforge schema` emits the machine-readable Graph Protocol.
