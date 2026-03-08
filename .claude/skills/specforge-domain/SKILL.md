---
name: Specforge Domain
description: "SpecForge product domain knowledge -- the .spec file format, compiler pipeline, graph schema, validation, LSP, CLI, and test coverage plugins. Use when working on any @specforge/core file, reasoning about the .spec compiler, discussing the DSL syntax, or making cross-domain connections between compiler subsystems."
---

# Specforge Domain

This skill teaches Claude the SpecForge product domain -- the "what" and "why" of the system. SpecForge is a standalone `.spec` file compiler that replaces disconnected markdown + YAML + bash validation with a single source of truth, compiler-checked references, and auto-generated traceability.

## When to use this skill

- Working on any `@specforge/core` file
- Reasoning about the `.spec` file format or compiler pipeline
- Discussing the in-memory graph, validation, or LSP features
- Working on test coverage plugins
- Writing or reviewing `.spec` files
- Answering questions about how SpecForge works
- Deciding whether a concept belongs in the DSL or should stay as markdown

## Core Mission

Domain-agnostic compiler that parses `.spec` files into validated entity graphs for AI agent consumption and human stakeholder review. Replaces disconnected markdown + YAML + bash validation with a single source of truth, compiler-checked references, and auto-generated traceability.

- A `.spec` file format (source of truth)
- A compiler that parses, resolves references, validates, and renders outputs
- An LSP server for IDE navigation
- Universal test coverage scanning via framework-native plugins

No database. No server. One binary.

**SpecForge is NOT a code generator.** It provides context; agents produce output.

**Package:** `@specforge/core`

## Key Principles

1. **Learn in 5 minutes** -- Minimal syntax, block-based structure, obvious semantics
2. **Zero domain knowledge in core** -- The compiler is a pure typed-graph engine. ALL domain vocabulary (behavior, invariant, feature, etc.) comes from extensions. If a new domain requires a compiler change, the architecture has failed.
3. **Agents are first-class consumers** -- Structured JSON, stable schemas, multi-resolution queries, token-efficient output
4. **Read like docs, compile like code** -- `.spec` files are human-readable documentation that is also machine-verifiable
5. **First-class cross-references** -- `[data_persistence]` is a compiler-resolved reference, not a string; typos are compile errors
6. **The graph protocol is the standard** -- The JSON schema is the product. Ten compilers producing the same graph is success.
7. **Extensions over built-ins, always** -- When in doubt, make it an extension
8. **Language-agnostic** -- Works for any project (TypeScript, Python, Go, Rust, Java, etc.); no runtime dependency

## Core Concepts Glossary

| Concept | Definition |
|---------|-----------|
| **`.spec` file** | Source-of-truth file in the SpecForge DSL; parsed by the compiler into an AST |
| **Compiler pipeline** | Parser -> Resolver -> In-Memory Graph -> Validators -> Emitters |
| **In-memory graph** | Directed graph of nodes (invariants, behaviors, features, etc.) and edges (references, implements, etc.) built from parsed `.spec` files |
| **Entity name** | Globally unique identifier -- any case style allowed. Titles are optional, auto-derived from names. |
| **`use` import** | File-level import statement that brings symbols from another `.spec` file into scope for reference resolution |
| **Resolver** | Compiler phase that resolves `use` imports, links name references to definitions, and builds the in-memory graph |
| **Validator** | Compiler phase that enforces graph invariants (no dangling refs, no duplicate names, no import cycles, orphan detection) |
| **Emitter** | Output renderer that traverses the graph to produce markdown, JSON, DOT graphs, traceability reports |
| **LSP server** | Language Server Protocol server providing go-to-def, find-refs, hover, autocomplete, rename, and live diagnostics |
| **`specforge-report.json`** | Standard report file emitted by test runner plugins containing per-behavior pass/fail/skip results |
| **`spec()` / `violation()`** | Test primitives provided by framework plugins to tag tests with behavior names or invariant violation tests |

## The `.spec` Format

### Block Types

Organized into core (structural) + three official extensions. The core compiler is domain-agnostic -- all domain vocabulary comes from extensions.

**Core (structural):**

| Block | Naming Convention | Purpose |
|-------|-------------------|---------|
| `spec` | -- (singleton) | Project root config: version, plugins, providers, test_dirs, coverage, persona, surface |
| `ref` | `scheme.kind:identifier` | External reference -- typed link to issues, tickets, designs (e.g., `gh.issue:42`) |

**@specforge/software:**

| Block | Naming Convention | Purpose |
|-------|-------------------|---------|
| `invariant` | `identifier` | Runtime guarantee with `guarantee` text, `enforced_by` references, `risk`, `maintains`, `verify` |
| `behavior` | `identifier` | Behavioral contract with `contract`, `category`, `requires`/`ensures`/`maintains`, `verify`, `abstract`/`refines`, `produces`/`consumers`, `tests`, `gherkin` (extension-declared field) |
| `feature` | `identifier` | User-facing capability composed of behaviors, with problem/solution framing |
| `event` | `identifier` | Domain/system event with trigger, payload (type reference), consumers, `sync` block, `verify` |
| `type` | `identifier` | Data type definition with `kind`, fields, and annotations (`@readonly`, `@unique`, `@literal`, `@optional`) |
| `port` | `identifier` | Interface definition with direction, methods (PortOperation with requires/ensures), and Result types |

**@specforge/product:**

| Block | Naming Convention | Purpose |
|-------|-------------------|---------|
| `capability` | `identifier` | UX flow mapping persona + surface to features |
| `deliverable` | `identifier` | Shippable artifact mapping capabilities to libraries |
| `roadmap` | `identifier` | Planning phase with behaviors, features, exit criteria |
| `library` | `identifier` | Code package mapping features to ports, with dependency DAG |
| `glossary` | -- (singleton) | Ubiquitous language term definitions |

**@specforge/governance:**

| Block | Naming Convention | Purpose |
|-------|-------------------|---------|
| `decision` | `identifier` | Architecture Decision Record with context, decision, consequences |
| `constraint` | `identifier` | Non-functional requirement with metric and threshold |
| `failure_mode` | `identifier` | FMEA risk assessment tied to an invariant (severity, occurrence, detection, RPN) |

### Entity Naming

Entity IDs are **variable-name identifiers**, not sequential numeric prefixes:

- Any valid identifier (starts with letter, letters/digits/underscores, 2-60 chars): `data_persistence`, `UserRepository`, `validate_input`
- Titles are optional -- auto-derived from name if omitted (`auth_login` -> "Auth Login")
- Names: 2-60 chars, letters/digits/underscores, no reserved words (the 16 entity keywords)
- Scheme refs for external references: `gh.issue:42`, `jira.epic:PROJ-123`

### Syntax Features

- **Triple-quoted strings (`"""`):** Preserve whitespace/newlines, strip common leading indentation
- **Reference lists:** `[data_persistence, email_uniqueness]` -- compiler-resolved, not strings
- **String lists:** `["foo", "bar"]` -- opaque string values
- **`use` imports:** `use invariants/data` imports all symbols; `use invariants/data { data_persistence }` for selective import
- **Comments:** `//` line comments
- **`verify` statements:** `verify unit "description"` inside behavior, invariant, and event blocks
- **Design-by-Contract blocks:** `requires { }`, `ensures { }`, `maintains { }` on behaviors and ports
- **Sync blocks:** `sync { barrier [...] timeout 30s "..." }` on events for CSP concurrency

### Example

```spec
use invariants/data

behavior create_user "Create User" {
  category command
  ports      [UserRepository, EmailService]
  invariants [data_persistence, email_uniqueness]

  contract """
    When a valid CreateUserCommand is received,
    the system MUST create a user record with unique email
    and MUST return Result<User, DuplicateEmailError>.
  """

  requires {
    valid_command      "CreateUserCommand passes schema validation"
    email_available    "no active user exists with the same email"
  }

  ensures {
    user_created       "a new User record exists in the datastore"
    email_unique       "email uniqueness invariant holds"
  }

  verify unit        "insert user, retrieve by ID, assert equal"
  verify integration "insert user, restart process, retrieve persists"

  tests [
    "tests/user_test.go::TestCreateUser",
    "tests/user.test.ts:45",
  ]
}
```

## Compiler Pipeline

```
.spec files  ->  Parser  ->  In-Memory Graph  ->  Validation passes
   (source       (AST)      (nodes + edges)     Navigation (LSP)
    of truth)                                    Renderings (markdown, JSON)
                                                 Coverage reports
                                                 Formal analysis
```

### Stages

1. **Parser** -- Tree-sitter grammar produces per-file ASTs
2. **Resolver** -- Resolves `use` imports, links name references to definitions, builds the in-memory graph; processes files in topological order (dependencies first); detects import cycles
3. **In-memory graph** -- Directed graph of nodes and edges; the "database" (no external database required)
4. **Validators** -- Enforce graph invariants; emit errors (E001-E003, E005-E009, E011-E014, E030-E035), warnings (W001-W040), info diagnostics (I001, I003-I009, I011)
5. **LSP** -- Reads from the in-memory graph; provides go-to-def, find-refs, hover, autocomplete, rename, live diagnostics
6. **Emitters** -- Traverse the graph to produce markdown, JSON, DOT graph, index.yaml, traceability reports

### Import Resolution

- `use invariants/data` resolves to `<spec_root>/invariants/data.spec`
- Spec root is the directory containing `specforge.spec`
- Paths are forward-slash separated (platform-independent), `.spec` extension is implicit
- All top-level declarations are public by default
- Files must `use` another file to reference its symbols -- this is enforced at compile time

### Incremental Compilation (Watch Mode)

File change -> invalidation (changed file + transitive dependents) -> re-parse only invalidated files -> rebuild affected subgraph edges -> re-validate affected subgraph -> emit diagnostics.

**Performance targets:** <100ms file-change-to-diagnostics (500 files), <2s full cold rebuild, <50MB memory.

## Graph Schema

### Node Types

**Core (structural) nodes:**

| Node Type | Naming | Required Properties | Optional Properties |
|---|---|---|---|
| `spec` | -- (singleton) | `name`, `version` | `namespace`, `display_prefix`, `test_dirs`, `coverage`, `persona`, `surface` |
| `ref` | `scheme.kind:id` | `scheme`, `identifier` | `title`, provider-specific fields |

**@specforge/software nodes:**

| Node Type | Naming | Required Properties | Optional Properties |
|---|---|---|---|
| `invariant` | `identifier` | `guarantee` | `enforced_by`, `risk`, `maintains`, `verify[]` |
| `behavior` | `identifier` | `contract` | `invariants`, `types`, `ports`, `produces`, `consumers`, `category`, `abstract`, `refines`, `requires`, `ensures`, `maintains`, `verify[]`, `tests[]`, `gherkin[]` |
| `feature` | `identifier` | `behaviors`, `problem`, `solution` | -- |
| `event` | `identifier` | `trigger` | `payload` (type ref), `consumers`, `channel`, `sync`, `verify[]` |
| `type` | `identifier` | `kind`, `fields` | -- |
| `port` | `identifier` | `direction`, `methods` | `category` |

**@specforge/product nodes:**

| Node Type | Naming | Required Properties | Optional Properties |
|---|---|---|---|
| `capability` | `identifier` | `persona`, `features`, `flow` | `surface` |
| `deliverable` | `identifier` | `capabilities` | `libraries`, `roadmap`, `personas`, `type` |
| `roadmap` | `identifier` | `status` | `behaviors`, `features`, `criteria` |
| `library` | `identifier` | `features` | `depends_on`, `ports_defined`, `family` |
| `glossary` | -- (singleton) | `terms` | -- |

**@specforge/governance nodes:**

| Node Type | Naming | Required Properties | Optional Properties |
|---|---|---|---|
| `decision` | `identifier` | `status`, `context`, `decision` | `date`, `consequences`, `invariants` |
| `constraint` | `identifier` | `description`, `category`, `priority` | `metric`, `behaviors`, `invariants` |
| `failure_mode` | `identifier` | `invariant`, `severity`, `occurrence`, `detection` | `rpn`, `cause`, `effect`, `mitigation`, `post_mitigation` |

### Edge Types

**Core (structural) edges:**

| Edge Type | From -> To | Semantics |
|---|---|---|
| `imports` | file -> file | "This file uses symbols from that file" |
| `links_to` | any entity -> ref | "This entity links to this external reference" |

**@specforge/software edges:**

| Edge Type | From -> To | Semantics |
|---|---|---|
| `references` | behavior -> invariant | "This behavior depends on these invariants" |
| `implements` | feature -> behavior | "This feature is composed of these behaviors" |
| `produces` | behavior -> event | "This behavior emits these events" |
| `consumes` | event -> behavior | "This event triggers these behaviors" |
| `uses_type` | behavior -> type | "This behavior uses these type definitions" |
| `uses_port` | behavior -> port | "This behavior uses these port interfaces" |
| `enforces` | invariant -> behavior | "This invariant is enforced by these behaviors" |
| `refines` | behavior -> behavior | "This behavior refines an abstract behavior" |

**@specforge/product edges:**

| Edge Type | From -> To | Semantics |
|---|---|---|
| `traces_to` | capability -> feature | "This UX capability maps to these features" |
| `bundles` | deliverable -> capability | "This deliverable ships these capabilities" |
| `built_from` | deliverable -> library | "This deliverable uses these libraries" |
| `depends_on` | library -> library | "This library depends on that library" |
| `provides` | library -> feature | "This library provides the code for these features" |
| `defines_port` | library -> port | "This library defines this port interface" |
| `schedules` | roadmap -> feature/deliverable | "This phase schedules these features" |

**@specforge/governance edges:**

| Edge Type | From -> To | Semantics |
|---|---|---|
| `protects` | decision -> invariant | "This decision protects these invariants" |
| `constrains` | constraint -> behavior/invariant | "This quality requirement applies to these entities" |
| `mitigates` | failure_mode -> invariant | "This failure mode threatens this invariant" |

### Graph Invariants

**Core invariants:**

1. **No dangling references** -- Every name in a reference list must resolve to a declared node; cross-plugin references use soft resolution (`E001` / `I004`)
2. **No duplicate names** -- Each entity name is globally unique across all `.spec` files (`E002`)
3. **No import cycles** -- The `imports` edges form a DAG (`E003`)

**@specforge/software invariants:**

4. **Orphan detection (software)** -- Behavior not in any feature -> `W001`

**@specforge/product invariants:**

5. **Orphan detection (product)** -- Feature not in any capability -> `W002`
6. **No circular library deps** -- `depends_on` edges between library nodes form a DAG (`E007`)
7. **Deliverable coverage** -- Every capability in a deliverable should be reachable through its library chain (`W008`)

**@specforge/governance invariants:**

8. **RPN consistency** -- If severity/occurrence/detection are all present, `rpn` must equal their product (`E005`)

## Validation

### Error Codes

Diagnostics are module-scoped: plugin rules only fire when the plugin is installed.

**Core:**

| Code | Level | Description |
|------|-------|-------------|
| `E001` | error | Unresolved reference -- name not found; "did you mean?" suggestions; soft for cross-plugin/provider refs |
| `E002` | error | Duplicate entity name -- same name declared in multiple files |
| `E003` | error | Circular import -- `use` statements form a cycle |
| `E011` | error | Invalid ref target format -- provider validates identifier doesn't match expected pattern |
| `E012` | error | Unknown provider kind -- ref uses kind not registered by its provider |
| `E013` | error | Reserved word used as identifier -- entity name matches a keyword |
| `E014` | error | Invalid identifier characters -- name contains invalid chars or format |
| `W012` | warning | Orphan ref -- declared but never referenced by any entity |
| `W013` | warning | Vague entity name -- name too short or non-descriptive |
| `I003` | info | Newer format features available -- project version < compiler version |
| `I004` | info | Unknown entity in reference field -- suggests installing a plugin |
| `I005` | info | Unknown provider scheme -- ref uses scheme not registered by any installed provider |

**@specforge/software:**

| Code | Level | Description |
|------|-------|-------------|
| `E004` | error | Port method references invalid type |
| `E006` | error | Event trigger invalid -- event's trigger must reference an existing behavior |
| `E030` | error | Always-false precondition in requires block |
| `E031` | error | Liskov compliance violation -- strengthened precondition or weakened postcondition in refinement |
| `E032` | error | Cycle in refinement chain |
| `E033` | error | Behavior not satisfying feature requirements |
| `E034` | error | Event deadlock detected -- circular event dependency |
| `E035` | error | Channel type mismatch -- producer and consumer payload types differ |
| `W001` | warning | Orphan behavior -- not referenced by any feature |
| `W003` | warning | Unused invariant -- not referenced by any behavior |
| `W004` | warning | Unverified behavior -- no `verify` statement |
| `W007` | warning | Orphan event -- event with no consumers |
| `W010` | warning | Unknown annotation on type field |
| `W029` | warning | Unmatched event producers -- no consumers |
| `W030` | warning | Incomplete refinement chain -- abstract with no concrete |
| `W031` | warning | Deep refinement chain (depth > 4) |
| `W032` | warning | Livelock risk -- re-triggering without backoff |
| `W033` | warning | Starvation risk -- unfair port access |
| `W034` | warning | Unbounded channel buffer -- no sync timeout |
| `W035` | warning | Undischarged proof obligation |
| `W036` | warning | Port-behavior contract incompatibility |
| `W037` | warning | Unverifiable contract condition |
| `W038` | warning | Unreachable postcondition -- contradicts preconditions |
| `W039` | warning | Redundant precondition -- implied by sibling |
| `W040` | warning | Invariant without formal property -- no maintains block |
| `I007` | info | Proof obligation verified by test |
| `I008` | info | Deadlock freedom verified |
| `I009` | info | Formal analysis available -- suggests `specforge analyze` |
| `I011` | info | Ensures without requires |

**@specforge/product:**

| Code | Level | Description |
|------|-------|-------------|
| `E007` | error | Circular library dependency -- `depends_on` edges between libraries form a cycle |
| `E008` | error | Persona not defined -- capability's `persona` doesn't match any persona defined in `spec` root |
| `E009` | error | Surface not defined -- capability's `surface` doesn't match any surface defined in `spec` root |
| `W002` | warning | Orphan feature -- not referenced by any capability |
| `W008` | warning | Uncovered capability -- deliverable references a capability not reachable via its libraries |
| `W009` | warning | Orphan library -- library not referenced by any deliverable's `libraries` field |
| `W011` | warning | Orphan capability -- capability not referenced by any deliverable's `capabilities` field |

**@specforge/governance:**

| Code | Level | Description |
|------|-------|-------------|
| `E005` | error | RPN mismatch -- severity x occurrence x detection != declared rpn |
| `W005` | warning | Unmitigated high-risk invariant -- `risk: high` or `risk: critical` with no `failure_mode` |
| `W006` | warning | Unconstrained behavior -- behavior with no constraint coverage for common categories |
| `I001` | info | Stale proposal -- decision with `status: proposed` older than 30 days |

### What Becomes Impossible by Construction

| Old manual rule | In the compiler |
|---|---|
| No duplicate names (62 bash rules) | **Parser error** -- duplicate name = compile error |
| Frontmatter schema validation | **No frontmatter** -- the syntax IS the schema |
| Forward reference checks | **Resolver error** -- unresolved `use` or name = compile error |
| Reverse coverage checks | **Validator warning** -- orphan detection on the graph |
| Index file completeness | **No index files** -- compiler generates them |
| Overview completeness | **Compiler generates** overview from graph |
| Content structure enforcement | **Syntax enforced** -- `behavior` block requires `contract` |
| Traceability matrices | **Auto-generated** -- graph traversal via `specforge trace` |

Error messages are styled like `rustc` -- file, line, column, context, suggestions.

## LSP Features

The LSP server shares the incremental compilation pipeline with watch mode.

| Feature | Description |
|---------|-------------|
| **Go-to-definition** | Ctrl+click on `data_persistence` -> jumps to its declaration |
| **Find all references** | Right-click on any entity -> shows every file that references it |
| **Hover info** | Shows entity title, guarantee/contract text, reference count, test count |
| **Autocomplete** | `invariants [data_\|` -> suggests all invariants in scope with titles |
| **Rename symbol** | Rename an entity -> updates every `.spec` file that references it |
| **Diagnostics** | Red squiggle on broken references, yellow on orphans, info on missing tests |
| **Code actions** | Quick-fixes for common issues |
| **Outline view** | Sidebar tree of all entities in the file with test coverage indicators |

## CLI Commands

```bash
specforge init                         # scaffold a new spec project (core only)
specforge add @specforge/product       # install product plugin (5 entities)
specforge add @specforge/governance    # install governance plugin (3 entities)
specforge remove @specforge/governance # remove a plugin
specforge plugins                      # list installed plugins
specforge check                        # parse + resolve + validate (like tsc --noEmit)
specforge check --strict               # treat warnings as errors
specforge watch                        # incremental recompilation on file change

specforge trace                        # print full traceability tree
specforge trace create_user            # trace one entity up and down

specforge render markdown ./docs/      # emit .md files for stakeholders
specforge render json ./out/           # emit JSON graph for tooling

specforge stats                        # summary: counts, coverage %, orphans
specforge graph                        # dump DOT format for visualization
specforge graph | dot -Tsvg > spec.svg

specforge lsp                          # start LSP server (editor integration)

specforge migrate --from=1.0 --to=2.0  # migrate spec files between format versions

specforge analyze contracts            # run contract_check pass, report results
specforge analyze refinement           # run refinement_verify pass
specforge analyze concurrency          # run process_analyze pass (CSP analysis)
specforge analyze all                  # run all analysis passes, unified report
specforge analyze all --strict         # exit non-zero on any violation (for CI)
specforge analyze all --json           # machine-readable JSON output

specforge coverage                          # merge specforge-report.json files + report
specforge coverage --min=95                 # CI gate: fail if below threshold

specforge collect go                        # parse `go test -json` output -> report
specforge collect rust                      # parse JUnit XML -> report
```

## Test Coverage Plugins

Framework-native plugins connect test runners to `.spec` behavior names.

### The `specforge-report.json` Protocol

Every plugin emits a standard JSON report after test execution containing per-behavior test results (pass/fail/skip/duration) and per-invariant violation test results.

### Test Primitives

- **`spec("create_user", () => { ... })`** -- Wraps `describe()`; tags all inner tests with the behavior name
- **`violation("email_uniqueness", () => { ... })`** -- Tags tests as invariant violation tests (prove the system prevents a bad state)
- Unknown names fail the test suite immediately (validated against `.spec` files)

### Framework Plugins

| Package | Language | Pattern |
|---------|----------|---------|
| `@specforge/vitest` | TypeScript | Reporter plugin; `spec()` / `violation()` wrappers |
| `@specforge/pytest` | Python | Pytest plugin; `@spec()` / `@violation()` decorators |
| `@specforge/go` | Go | Test helper + `go test -json` output collector |

## Extension Model

SpecForge has three extension mechanisms, inspired by Terraform's model:

| Mechanism | What It Extends | How Declared | Multiple Instances |
|-----------|----------------|--------------|-------------------|
| **Plugin** | Entity model -- adds new block types | `plugins [@specforge/product]` in spec root | No (one per plugin) |
| **Provider** | Ref validation -- registers schemes, kinds, URL templates | `providers { gh { ... } }` in spec root | Yes (aliases: `gh_enterprise`) |
| **Renderer** | Output format -- reads graph, emits files | `plugin "my-renderer" { type renderer }` | No (one per renderer) |

### Syntax Examples

**Library:**

```spec
library core_auth "Core Auth" {
  family     platform
  features   [user_authentication, token_management]
  depends_on [crypto_lib]

  ports_defined [UserRepository, TokenService]
}
```

**Deliverable:**

```spec
deliverable web_app "Web Application" {
  type        webapp
  personas    [developer, admin]
  capabilities [user_management_flow, admin_dashboard_flow]
  libraries   [core_auth, web_framework]
  roadmap     mvp
}
```

**Roadmap:**

```spec
roadmap mvp "MVP" {
  status     active
  behaviors  [create_user, validate_input, parse_spec_files]
  features   [user_management, input_validation, spec_parsing]

  criteria """
    All MVP behaviors passing.
    Coverage >= 90%.
    Zero open E-level diagnostics.
  """
}
```

### Syntax Example: Persona and Surface in Spec Root

```spec
spec "my-project" {
  version "1.0"

  persona developer "Software Developer" {
    description "Builds and maintains the application"
  }

  persona admin "System Administrator" {
    description "Manages deployment and configuration"
  }

  surface web "Web Browser" {
    type webapp
  }

  surface cli "Command Line" {
    type terminal
  }
}
```

## What NOT to Do

- Do not confuse `.spec` files with markdown spec documents -- `.spec` is a compiled DSL, not documentation
- Do not assume a database (Neo4j or otherwise) -- the compiler uses an in-memory graph rebuilt on each compile
- Do not assume a server or agent system -- SpecForge is a CLI tool, not an orchestration platform
- Do not treat entity names as strings -- they are compiler-resolved references with type checking
- Do not hand-write traceability matrices -- they are auto-generated by `specforge trace`
- Do not hand-write index files -- the compiler generates them
- Do not use comment scanning (`// @spec BEH-...`) for test coverage -- use the framework-native `spec()` / `violation()` plugins
- Do not promote narrative concepts (research, product, process, references) to the DSL -- they are prose documents that gain nothing from compilation
- Do not add traceability or overview as source node types -- they are compiler-generated outputs
- Do not confuse the three extension mechanisms: **plugins** extend the entity model, **providers** extend ref validation, **renderers** extend output formats -- see the Extension Model section
