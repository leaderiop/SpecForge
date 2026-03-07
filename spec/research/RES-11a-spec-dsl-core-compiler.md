---
id: RES-11a
kind: research
title: "Spec DSL Core Compiler Architecture"
status: partially-superseded
date: 2026-03-01
split_from: RES-11
---

# RES-11a: Spec DSL Core Compiler Architecture

> **Status:** Active with inline corrections. Plugin architecture updated to Wasm/Extism (RES-21b, RES-23). Entity types updated to zero-entity core with extension-defined entities (RES-26). Compiler pipeline design and entity ID grammar remain as-is.

## Problem Statement

The current spec-authoring system uses markdown + YAML frontmatter validated by 62 bash scripts and 25+ Claude skills. This approach has fundamental weaknesses:

1. **References are strings** — `invariants: [INV-SF-7]` in YAML is opaque; typos discovered only after bash scripts run
2. **Same ID duplicated 5-7 places** — frontmatter, headings, index.yaml, traceability files, invariant back-references, overview document map
3. **Validation is post-hoc** — 62 rules implemented as bash scripts parsing markdown with grep/sed/yq; fragile, slow feedback
4. **Traceability manually maintained** — 10 TRACE-SF-* files are hand-written matrices, drift guaranteed
5. **No connection to code/tests** — no mechanism linking behaviors to actual test files
6. **Language-specific** — a TypeScript internal DSL would lock out Python/Go/Rust/Java projects

## Proposal: A Standalone Spec Compiler

Replace the markdown + bash system with:

- A `.spec` file format (source of truth)
- A compiler that parses, resolves references, validates, and generates outputs
- An LSP server for IDE navigation
- Universal test coverage scanning via `@spec` annotations

No database. No server. One binary.

---

## Core Architecture

```
.spec files  →  Parser  →  In-Memory Graph  →  Validation passes
   (source       (AST)      (nodes + edges)     Navigation (LSP)
    of truth)                                    Renderings (markdown, JSON)
                                                 Coverage reports
```

### Pipeline

```
                    .spec files (on disk)
                          │
                          ▼
                 ┌─────────────────┐
                 │     Parser      │  tree-sitter grammar
                 │                 │  → per-file AST
                 └────────┬────────┘
                          │
                          ▼
                 ┌─────────────────┐
                 │    Resolver     │  resolve `use` imports
                 │                 │  link ID references to definitions
                 │                 │  build in-memory graph
                 └────────┬────────┘
                          │
                          ▼
               ┌─────────────────────┐
               │   In-Memory Graph   │  nodes: extension-defined entities
               │                     │         (e.g. behavior, feature,
               │   (the "database")  │         invariant, capability...)
               │                     │  edges: references, enforces,
               │                     │         implements, traces_to
               └────────┬────────────┘
                        │
          ┌─────────────┼─────────────────┐
          ▼             ▼                 ▼
   ┌────────────┐ ┌──────────┐  ┌──────────────┐
   │ Validators │ │   LSP    │  │   Emitters   │
   │            │ │          │  │              │
   │ • orphans  │ │ • go-def │  │ • markdown   │
   │ • broken   │ │ • refs   │  │ • json       │
   │   refs     │ │ • hover  │  │ • coverage   │
   │ • coverage │ │ • diag   │  │ • trace      │
   │ • ranges   │ │ • rename │  │ • index.yaml │
   └────────────┘ └──────────┘  └──────────────┘
```

---

## The `.spec` Format

### Design Goals

- Learn in 5 minutes
- Read like documentation, compile like code
- First-class cross-references (not strings)
- Minimal syntax noise
- Language-agnostic (works for any project)

### Syntax

#### Project Root

```spec
// specforge.spec
spec "my-service" {
  version "1.0"

  test_dirs ["tests/", "src/**/*.test.*"]
}
```

#### Invariants

```spec
// invariants/data.spec

invariant data_persistence "Data Persistence" {
  guarantee """
    All committed writes survive process restarts.
    No acknowledged write may be silently dropped.
  """
  enforced_by [PostgresAdapter, WriteAheadLog]
  risk high
}

invariant email_uniqueness "Email Uniqueness" {
  guarantee "No two active users share the same email address."
  enforced_by [UniqueConstraint, UserRepository]
  risk medium
}
```

#### Decisions (ADRs)

```spec
// Example: hypothetical decision
// decisions/postgres.spec

decision postgres_over_mongodb "PostgreSQL over MongoDB" {
  status   accepted
  date     2025-03-01

  context """
    We need a primary datastore. Team has SQL expertise.
    Document model not needed — data is relational.
  """

  decision """
    Use PostgreSQL with typed schemas.
  """

  consequences [
    "Migrations required for schema changes",
    "Strong ACID guarantees",
  ]

  invariants [data_persistence]    // ← compiler-resolved reference
}
```

#### Behaviors

```spec
// behaviors/user-crud.spec

use invariants/data       // ← file-level import
use decisions/postgres

behavior create_user "Create User" {
  adrs       [postgres_over_mongodb]
  types      [user, auth]
  ports      [UserRepository, EmailService]
  invariants [data_persistence, email_uniqueness]

  contract """
    When a valid CreateUserCommand is received,
    the system MUST create a user record with unique email
    and MUST return Result<User, DuplicateEmailError>.
  """

  verify unit        "insert user, retrieve by ID, assert equal"
  verify integration "insert user, restart process, retrieve persists"
  verify property    "email uniqueness holds under concurrent inserts"

  tests [
    "tests/user_test.go::TestCreateUser",
    "tests/user.test.ts:45",
    "tests/test_user.py::test_create_user",
  ]
}

behavior read_user_by_id "Read User by ID" {
  adrs       [postgres_over_mongodb]
  types      [user]
  ports      [UserRepository]
  invariants [data_persistence]

  contract """
    Given a valid user ID, MUST return the user or NotFoundError.
    MUST NOT return stale data after a successful write.
  """

  verify unit "insert then get by ID"
}

behavior update_user_email "Update User Email" {
  adrs       [postgres_over_mongodb]
  types      [user, auth]
  ports      [UserRepository, EmailService]
  invariants [data_persistence, email_uniqueness]

  contract """
    MUST validate new email uniqueness before committing.
    MUST return DuplicateEmailError if email already taken.
  """

  verify unit        "update to unique email succeeds"
  verify unit        "update to taken email fails with DuplicateEmailError"
  verify integration "concurrent updates to same email — exactly one wins"
}
```

#### Features

```spec
// features/user-management.spec

use behaviors/user-crud
use roadmap/phases

feature user_management "User Management" {
  behaviors [create_user, read_user_by_id, update_user_email]
  roadmap   [phase_1_core]

  problem """
    Administrators need to manage user accounts
    with guaranteed data integrity.
  """

  solution """
    CRUD operations backed by PostgreSQL with
    unique email constraints and full audit trail.
  """
}
```

#### Capabilities

```spec
// capabilities/admin-users.spec

use features/user-management

capability create_new_user "Create a New User" {
  persona  admin
  surface  [web, cli, api]
  features [user_management]

  flow """
    1. Admin opens user management page
    2. Clicks "New User"
    3. Fills form (name, email, role)
    4. Submits → system validates uniqueness
    5. Success: user appears in list
    6. Failure: inline error on email field
  """
}
```

#### Risk Assessment (FMEA)

```spec
// risk-assessment/data-integrity.spec

use invariants/data

failure_mode write_ack_lost "Write Acknowledged but Lost" {
  invariant  data_persistence
  severity   8
  occurrence 2
  detection  3
  rpn        48    // auto-computed: S × O × D

  cause      "Crash between ACK and fsync"
  effect     "Silent data loss — user believes write succeeded"
  mitigation "Write-ahead log with fsync before ACK"

  post_mitigation {
    severity   8
    occurrence 1
    detection  2
    rpn        16
  }
}
```

---

## Formal Grammar

The `.spec` format is defined by a tree-sitter grammar. Below is an EBNF sketch of the core syntax. This is the normative reference for the parser.

### Top-Level Structure

```ebnf
file            = { use_statement } , { top_level_decl } ;
use_statement   = "use" , module_path , [ selective_import ] ;
module_path     = identifier , { "/" , identifier } ;
selective_import = "{" , identifier , { "," , identifier } , "}" ;

top_level_decl  = spec_decl | invariant_decl | constraint_decl | decision_decl
                | behavior_decl | event_decl | feature_decl | capability_decl
                | failure_mode_decl | type_decl | port_decl
                | library_decl | deliverable_decl | roadmap_decl ;
```

### Block Declarations

```ebnf
spec_decl       = "spec" , string_literal , block ;
invariant_decl  = "invariant" , entity_name , string_literal , block ;
constraint_decl = "constraint" , entity_name , string_literal , block ;
decision_decl   = "decision" , entity_name , string_literal , block ;
behavior_decl   = "behavior" , entity_name , string_literal , block ;
event_decl      = "event" , entity_name , string_literal , block ;
feature_decl    = "feature" , entity_name , string_literal , block ;
capability_decl = "capability" , entity_name , string_literal , block ;
failure_mode_decl = "failure_mode" , entity_name , string_literal , block ;
library_decl    = "library" , entity_name , string_literal , block ;
deliverable_decl = "deliverable" , entity_name , string_literal , block ;
roadmap_decl    = "roadmap" , entity_name , string_literal , block ;

block           = "{" , { attribute | nested_block | comment } , "}" ;
nested_block    = keyword , [ entity_name , string_literal ] , block ;
```

### Entity Names

```ebnf
entity_name     = snake_ident | pascal_ident ;
snake_ident     = lower , { lower | digit | "_" } ;   (* 2-60 chars *)
pascal_ident    = upper , { letter | digit } ;         (* 2-60 chars *)
```

### Attributes and Values

```ebnf
attribute       = keyword , value ;
keyword         = identifier ;

value           = string_literal | triple_string | number_literal
                | boolean_literal | enum_value | reference
                | reference_list | string_list
                | verify_stmt ;

string_literal  = '"' , { char } , '"' ;
triple_string   = '"""' , { any_char } , '"""' ;
number_literal  = digit , { digit } ;
boolean_literal = "true" | "false" ;
enum_value      = identifier ;                    (* e.g., high, medium, accepted *)
reference       = entity_name | identifier ;       (* resolved by linker *)
reference_list  = "[" , reference , { "," , reference } , "]" ;
string_list     = "[" , string_literal , { "," , string_literal } , "]" ;
verify_stmt     = "verify" , enum_value , string_literal ;

comment         = "//" , { any_char_except_newline } ;
```

### Key Semantic Rules

1. **Triple-quoted strings (`"""`):** Preserve internal whitespace and newlines. Leading common indentation is stripped (like Kotlin `trimMargin`). No escape sequences inside triple-quoted strings.

2. **Reference lists vs. string lists:** `[data_persistence, email_uniqueness]` is a reference list (resolved by the linker). `["foo", "bar"]` is a string list (opaque). The parser distinguishes by the presence/absence of quotes.

3. **Fixed attribute set:** Each block type has a fixed set of allowed attributes. Unknown attributes are a compile error. This prevents typos and ensures all `.spec` files are interpretable by the compiler. Custom attributes use the `define` mechanism (see [Extensibility](#extensibility)).

4. **Type syntax:** Type expressions like `Result<User, DuplicateEmailError>` and union types `DuplicateEmailError | UserNotFoundError` are parsed in `type` and `port` blocks only. These are fully parsed and resolved as part of the type resolution pipeline.

---

## Graph Schema

The compiler builds an in-memory directed graph from parsed `.spec` files. This section defines the node types, edge types, and structural invariants of that graph.

### Node Types

All entity types below are extension-defined. `@specforge/software` provides behavior, event, feature, type, port, invariant. `@specforge/product` provides capability, deliverable, roadmap, library, glossary. `@specforge/governance` provides decision, constraint, failure_mode. The `spec` block is the only structural element defined by core.

**Core (structural):**

| Node Type | Required Properties | Optional Properties |
|---|---|---|
| `spec` | `name`, `version` | `test_dirs`, `coverage`, `persona`, `surface` |

**@specforge/software:**

| Node Type | Required Properties | Optional Properties |
|---|---|---|
| `invariant` | `guarantee` | `enforced_by`, `risk` |
| `behavior` | `contract` | `invariants`, `adrs` (soft ref), `types`, `ports`, `verify[]`, `tests[]` |
| `event` | `trigger` | `payload`, `consumers`, `channel` |
| `feature` | `behaviors`, `problem`, `solution` | `roadmap` |
| `type` | `fields` | `@readonly`, `@unique`, `@literal` |
| `port` | `direction`, `methods` | `category` |

**@specforge/product:**

| Node Type | Required Properties | Optional Properties |
|---|---|---|
| `capability` | `persona`, `features`, `flow` | `surface` |
| `deliverable` | `capabilities` | `libraries`, `roadmap`, `personas`, `type` |
| `roadmap` | `status` | `behaviors`, `features`, `criteria` |
| `library` | `features` | `depends_on`, `ports_defined`, `family` |
| `glossary` | `terms` | — |

**@specforge/governance:**

| Node Type | Required Properties | Optional Properties |
|---|---|---|
| `decision` | `status`, `context`, `decision` | `date`, `consequences`, `invariants` |
| `constraint` | `description`, `category`, `priority` | `metric`, `behaviors`, `invariants` |
| `failure_mode` | `invariant`, `severity`, `occurrence`, `detection` | `rpn`, `cause`, `effect`, `mitigation`, `post_mitigation` |

### Edge Types

| Edge Type | From → To | Cardinality | Semantics |
|---|---|---|---|
| `uses_type` | behavior → type | N:M | "This behavior uses these type definitions" |
| `uses_port` | behavior → port | N:M | "This behavior uses these port interfaces" |
| `enforces` | invariant → identifier | 1:N | "This invariant is enforced by these components" |
| `references` | behavior → invariant | N:M | "This behavior depends on these invariants" |
| `protects` | decision → invariant | N:M | "This decision protects these invariants" |
| `shaped_by` | behavior → decision | N:M | "This behavior was shaped by these decisions" (soft ref) |
| `implements` | feature → behavior | 1:N | "This feature is composed of these behaviors" |
| `traces_to` | capability → feature | N:M | "This UX capability maps to these features" |
| `mitigates` | failure_mode → invariant | 1:1 | "This failure mode threatens this invariant" |
| `constrains` | constraint → behavior/invariant | N:M | "This quality requirement applies to these entities" |
| `produces` | behavior → event | 1:N | "This behavior emits these events" |
| `consumes` | event → behavior | 1:N | "This event triggers these behaviors" |
| `bundles` | deliverable → capability | 1:N | "This deliverable ships these capabilities" |
| `built_from` | deliverable → library | 1:N | "This deliverable uses these libraries" |
| `depends_on` | library → library | N:M | "This library depends on that library" |
| `provides` | library → feature | 1:N | "This library provides the code for these features" |
| `defines_port` | library → port | 1:N | "This library defines this port interface" |
| `schedules` | roadmap → feature/deliverable | 1:N | "This phase schedules these features or deliverables" |
| `imports` | file → file | N:M | "This file uses symbols from that file" |

### Graph Invariants

The validator enforces these structural rules on the graph:

1. **No dangling references:** Every name in a reference list must resolve to a declared node. Violation = `E001 unresolved reference`.
2. **No duplicate names:** Each entity name is globally unique across all `.spec` files. Violation = `E002 duplicate name`.
3. **No import cycles:** The `imports` edges form a DAG. Violation = `E003 circular import`.
4. **Orphan detection:** A behavior not referenced by any feature emits `W001 orphan behavior`. A feature not referenced by any capability emits `W002 orphan feature`.
5. **RPN consistency:** If `severity`, `occurrence`, and `detection` are all present, `rpn` must equal their product (or be omitted for auto-computation). Violation = `E005 RPN mismatch`.
6. **Event trigger validity:** An event's trigger must reference an existing behavior. Violation = `E006 event trigger invalid`.
7. **No circular library deps:** `depends_on` edges between library nodes form a DAG. Violation = `E007 circular library dependency`.
8. **Deliverable coverage:** Every capability in a deliverable should be reachable through its library chain. Violation = `W008 uncovered capability`.

---

## Import Resolution

### `use` Syntax

The `use` statement imports symbols from other `.spec` files:

```spec
// File-based import — all symbols from the file
use invariants/data

// Selective import — only specific symbols
use invariants/data { data_persistence, email_uniqueness }

// Multiple imports
use invariants/data
use decisions/postgres
use behaviors/user-crud
```

### Path Resolution

- `use invariants/data` resolves to `<spec_root>/invariants/data.spec`
- The spec root is the directory containing `specforge.spec`
- Paths are always forward-slash separated (platform-independent)
- File extensions (`.spec`) are implicit and must not be included in `use` statements

### Resolution Algorithm

The resolver runs after parsing and before validation:

```
1. PARSE PHASE
   For each .spec file in the project:
     Parse → AST
     Extract: declared symbols (names), use statements, reference lists

2. BUILD FILE GRAPH
   For each use statement:
     Resolve path to target file
     Add edge: source_file → target_file
     Record which symbols are imported (all or selective)

3. DETECT CYCLES
   Run topological sort on the file graph
   If cycle detected → emit E003 with the cycle path:
     error[E003]: circular import detected
       --> behaviors/a.spec:1:1
        |
      1 | use behaviors/b
        | ^^^^^^^^^^^^^^^^
        |
       --> behaviors/b.spec:1:1
        |
      1 | use behaviors/a
        | ^^^^^^^^^^^^^^^^
        = note: cycle: a.spec → b.spec → a.spec

4. RESOLVE TOPOLOGICALLY
   Process files in topological order (dependencies first)
   For each file:
     Bind each reference to its declaration
     If selective import: only bind the listed symbols
     If unresolved: collect as error candidate

5. LINK SYMBOLS
   Build the in-memory graph:
     Create nodes for each declaration
     Create edges for each reference binding
     Emit E001 for any unresolved references with suggestions:
       - Levenshtein distance for "did you mean?"
       - List available symbols in scope
```

### Visibility Rules

- All top-level declarations in a `.spec` file are public by default
- `use` brings symbols into scope for reference resolution only (no re-export)
- A file that doesn't `use` another file cannot reference its symbols — this is a compile error, not a silent miss

---

## Compiler Validation

### What becomes impossible by construction

| Current VAL rule | In the compiler |
|---|---|
| VAL-001–009: No duplicate IDs | **Parser error** — duplicate name = compile error |
| VAL-010–017: Frontmatter schema | **No frontmatter** — the syntax IS the schema |
| VAL-018–024: Forward references | **Resolver error** — unresolved `use` or name = compile error |
| VAL-025–030: Reverse coverage | **Validator warning** — orphan detection on the graph |
| VAL-031–037: Index completeness | **No index files** — compiler generates them |
| VAL-038–042: Overview completeness | **Compiler generates** overview from graph |
| VAL-043–048: Content structure | **Syntax enforced** — `behavior` block requires `contract` |
| VAL-049–053: Traceability | **Auto-generated** — traverse the graph |

From 62 hand-written bash rules → ~10 graph-level warnings emitted automatically. The rest are structurally impossible.

### Graph-Level Validation

Beyond structural impossibility, the validator performs these graph-level checks:

1. **Reachability analysis:** Every invariant should be referenced by at least one behavior. Unreferenced invariants get `W003 unused invariant`.
2. **Coverage completeness:** Every behavior in a feature should have at least one `verify` statement. Missing verification emits `W004 unverified behavior`.
3. **Decision staleness:** Decisions with `status: proposed` that are older than 30 days emit `I001 stale proposal`.
4. **Risk coverage:** Invariants with `risk: high` that have no associated `failure_mode` emit `W005 unmitigated high-risk invariant`.

### Error Messages

```
error[E001]: unresolved reference `email_uniquenes`
  --> behaviors/user-crud.spec:12:18
   |
12 |     invariants [data_persistence, email_uniquenes]
   |                                   ^^^^^^^^^^^^^^^ not found
   |
   = help: did you mean `email_uniqueness`?
   = note: available invariants: data_persistence, email_uniqueness, audit_trail

error[E002]: duplicate behavior name `create_user`
  --> behaviors/order-crud.spec:5:3
   |
 5 |   behavior create_user "Create Order" {
   |            ^^^^^^^^^^^ already defined here
   |
  --> behaviors/user-crud.spec:8:3
   |
 8 |   behavior create_user "Create User" {
   |            ^^^^^^^^^^^ first definition

warning[W001]: orphan behavior — not referenced by any feature
  --> behaviors/user-crud.spec:30:3
   |
30 |   behavior soft_delete_user "Soft Delete User" {
   |            ^^^^^^^^^^^^^^^^ add to a feature or remove
```

Errors styled like `rustc` — file, line, column, context, suggestions.

---

## Incremental Compilation

### Watch Mode

The compiler supports a `specforge watch` mode for rapid feedback during authoring. The incremental pipeline works as follows:

```
File system event (create/modify/delete .spec file)
          │
          ▼
┌─────────────────────────┐
│  Invalidation           │  Determine affected files:
│                         │  - The changed file
│                         │  - All files that `use` it (transitive)
└────────────┬────────────┘
             │
             ▼
┌─────────────────────────┐
│  Re-parse               │  Only re-parse invalidated files
│                         │  Reuse cached ASTs for unchanged files
└────────────┬────────────┘
             │
             ▼
┌─────────────────────────┐
│  Re-resolve             │  Rebuild graph edges for affected subgraph
│                         │  Existing unaffected edges preserved
└────────────┬────────────┘
             │
             ▼
┌─────────────────────────┐
│  Re-validate            │  Run validators on affected subgraph only
│                         │  Emit updated diagnostics
└────────────┬────────────┘
             │
             ▼
┌─────────────────────────┐
│  Emit diagnostics       │  Push to LSP clients / terminal output
└─────────────────────────┘
```

### Performance Targets

- **File change → diagnostics:** <100ms for projects up to 500 `.spec` files
- **Full rebuild from cold:** <2s for projects up to 500 files
- **Memory footprint:** <50MB for the in-memory graph of a 500-file project

### LSP Integration

The LSP server and watch mode share the same incremental pipeline:

- `textDocument/didChange` triggers the same invalidation → re-parse → re-resolve → re-validate cycle
- The LSP server holds the in-memory graph as long-lived state
- Diagnostics are pushed via `textDocument/publishDiagnostics`
- Go-to-definition, references, hover, and autocomplete all read from the in-memory graph

### Known Limitations

- **No persistent cache:** The graph is rebuilt from scratch on compiler startup. Incremental compilation only applies within a single session (watch mode / LSP).
- **No parallel parsing:** Files are parsed sequentially in topological order. Parallelism is a future optimization.

---

## LSP: Navigation Layer

The LSP server runs against the in-memory graph. Compiler in watch mode rebuilds the graph on file change (see [Incremental Compilation](#incremental-compilation)).

### Features

**Go-to-definition:**
Ctrl+click on `data_persistence` anywhere → jumps to `invariants/data.spec:3`

**Find all references:**
Right-click `data_persistence` → shows every behavior, decision, and feature that references it

**Hover info:**
```
┌──────────────────────────────────────────────┐
│ invariant data_persistence "Data Persistence"│
│                                              │
│ All committed writes survive process         │
│ restarts.                                    │
│                                              │
│ Referenced by: 3 behaviors, 1 ADR            │
│ Risk: high                                   │
│ Tests: 4 ✓                                   │
└──────────────────────────────────────────────┘
```

**Autocomplete:**
```
invariants [da|
              ├─ data_persistence    "Data Persistence"
              ├─ data_encryption     "Data Encryption"
              └─ data_retention      "Data Retention Policy"
```

**Rename symbol:**
Rename `data_persistence` → updates every `.spec` file that references it.

**Diagnostics (live, as you type):**
- Red squiggle on broken references
- Yellow squiggle on orphan behaviors
- Info squiggle on behaviors without tests

**Code actions:**
```
create_order has no tests.
  Quick fix: Add verify block | Add tests field | View in graph
```

**Outline view (sidebar):**
```
behaviors/user-crud.spec
  User CRUD
    ├─ create_user          "Create User"         ✓ 3 tests
    ├─ read_user_by_id      "Read User by ID"     ✗ no tests
    ├─ update_user_email    "Update User Email"   ✗ no tests
    └─ ...
```

### Editor Queries

The tree-sitter grammar ships four query files in `queries/` that any tree-sitter-aware editor (Neovim, Helix, Zed, Emacs) can consume:

| File | Purpose |
|------|---------|
| `highlights.scm` | Syntax highlighting — maps block keywords, sub-block keywords, entity names, strings, types, annotations, and punctuation to standard capture groups |
| `folds.scm` | Code folding — marks all block types and sub-blocks (persona, surface, term, provider, nested) as foldable regions |
| `indents.scm` | Auto-indentation — `{`/`[` trigger indent, `}`/`]` trigger dedent |
| `injections.scm` | Language injection — enables markdown highlighting inside triple-quoted strings |

These are passive artifacts — the compiler does not use them. They exist solely for editor DX and are maintained alongside the grammar.

---

## CLI Commands

```bash
specforge init                        # scaffold a new spec project
specforge check                       # parse + resolve + validate (like tsc --noEmit)
specforge check --strict              # treat warnings as errors
specforge watch                       # incremental recompilation on file change

specforge trace                       # print full traceability tree
specforge trace create_user           # trace one behavior up and down

specforge render markdown ./docs/     # emit .md files for stakeholders
specforge render json ./out/          # emit JSON graph for tooling

specforge stats                       # summary: counts, coverage %, orphans
specforge graph                       # dump DOT format for visualization
specforge graph | dot -Tsvg > spec.svg

specforge lsp                         # start LSP server (editor integration)

specforge migrate --from=1.0 --to=2.0 # migrate spec files between format versions
```

> **Note:** `specforge coverage` is documented in the coverage subsystem.

---

## Traceability: Auto-Generated

Traceability is NOT a file you write. It's a graph traversal the compiler performs.

```bash
$ specforge trace

TRACEABILITY CHAIN
══════════════════
create_new_user "Create a New User"
  └─ user_management "User Management"
      ├─ create_user "Create User"
      │   ├─ data_persistence "Data Persistence"
      │   ├─ email_uniqueness "Email Uniqueness"
      │   ├─ postgres_over_mongodb "PostgreSQL over MongoDB"
      │   └─ tests: 3 (go, ts, py)  ✓
      ├─ read_user_by_id "Read User by ID"
      │   ├─ data_persistence "Data Persistence"
      │   └─ tests: 0  ✗ MISSING
      └─ update_user_email "Update User Email"
          ├─ data_persistence "Data Persistence"
          ├─ email_uniqueness "Email Uniqueness"
          └─ tests: 0  ✗ MISSING
```

### Trace a Single Entity

```bash
$ specforge trace create_user

create_user "Create User"
  ▲ upstream
  │ └─ user_management "User Management"
  │     └─ create_new_user "Create a New User"
  │
  ▼ downstream
  ├─ data_persistence "Data Persistence"
  ├─ email_uniqueness "Email Uniqueness"
  ├─ postgres_over_mongodb "PostgreSQL over MongoDB"
  ├─ write_ack_lost "Write Acknowledged but Lost" (RPN: 48)
  └─ tests:
      ├─ tests/user_test.go::TestCreateUser
      ├─ tests/user.test.ts:45
      └─ tests/test_user.py::test_create_user
```

---

## Extensibility

### Core Schema

The compiler core defines structural constructs only. All domain entity types come from extensions (RES-26).

- **Core structural types:** `spec` (project root block)
- **Entity types:** Extension-defined (see [Graph Schema](#graph-schema) for the full set across `@specforge/software`, `@specforge/product`, `@specforge/governance`)
- **Attributes per node type:** Declared by extensions. Unknown attributes are compile errors.
- **Edge types:** `references`, `implements`, `produces`, `consumes`, `uses_type`, `uses_port`, `enforces`, `imports`, `traces_to`, `bundles`, `built_from`, `provides`, `depends_on`, `defines_port`, `schedules`, `protects`, `shaped_by`, `constrains`, `mitigates`
- **Emitters:** markdown, JSON, DOT graph, index.yaml, traceability report

The core compiler parses any `keyword name { ... }` block generically. Extensions declare which keywords are valid and what attributes they accept.

### Meta-Schema: `define` Blocks

For domain-specific node types beyond the core set, the `define` mechanism in `specforge.spec` allows user-defined types with attribute validation, reference resolution, orphan detection, and LSP support:

```spec
spec "my-service" {
  version "1.0"

  define research {
    attributes {
      outcome     enum [adr, behavior, deferred, rejected]
      related_adr ref? decision
      date        string
    }
  }
}
```

### Extension API

Extensibility beyond the `define` mechanism is supported through Wasm/Extism extensions (RES-21b, RES-23):

1. **Custom node types:** Define new node kinds via extension manifests.
2. **Custom attributes:** Extend existing node types with project-specific attributes.
3. **Emitter extensions:** Custom output formats beyond the built-in set.
4. **Validator extensions:** Custom graph-level validation rules.

Extensions are Wasm modules loaded via the Extism runtime. See RES-23 for the contribution-based extension model.

---

## Versioning & Breaking Changes

### Format Version

Every project declares its format version in the root `specforge.spec`:

```spec
spec "my-service" {
  version "1.0"    // ← this is the format version
}
```

The compiler checks compatibility on startup:

- **Same major version:** fully compatible, proceed normally
- **Newer minor version:** compatible, but may emit `I003 newer format features available`
- **Older major version:** incompatible, emit error with migration instructions

### Migration Tooling

```bash
# Check what would change
specforge migrate --from=X --to=Y --dry-run

# Apply migration
specforge migrate --from=X --to=Y

# Migration is reversible
specforge migrate --from=Y --to=X
```

### Deprecation Policy

1. Features deprecated in version N are removed in version N+1
2. Deprecated features emit `W010 deprecated` warnings with migration guidance
3. At least one minor version between deprecation and removal
4. The `migrate` command handles all mechanical transformations

---

## Migration: Drop YAML Fallback

The original RES-11 proposed a phased migration with YAML as an intermediate format. **This section supersedes that approach.**

### Decision: Go Straight to `.spec`

We skip the YAML fallback and implement the `.spec` parser directly. Rationale:

1. **No legacy users:** This is a greenfield tool. There are zero existing `.spec.yaml` files to migrate.
2. **Tree-sitter is fast:** A tree-sitter grammar can be prototyped in days, not weeks. The grammar is simple (no operator precedence, no expressions, just blocks + attributes).
3. **Dual-format doubles maintenance:** Supporting both YAML and `.spec` means two parsers, two sets of error messages, two documentation paths. The complexity is not worth it for zero users.
4. **YAML is a poor fit:** YAML's indentation sensitivity, implicit typing, and "Norway problem" (`NO` → `false`) make it a poor choice for a structured spec language.

### Implementation Strategy

If timeline is critical (need a working prototype before tree-sitter grammar is polished):

1. **Prototype:** Hand-written recursive descent parser in Rust. Simple, fast, good error messages. Handles the full grammar.
2. **Production:** Migrate to tree-sitter for LSP integration (incremental parsing, syntax highlighting, error recovery). The tree-sitter grammar becomes the normative syntax definition.

Both parsers produce the same AST. The handwritten parser can be the primary parser for CLI usage (faster startup, no tree-sitter runtime dependency), while tree-sitter powers the LSP.

---

## What Changes with the Compiler

### What You Don't Write Anymore

| Before (markdown + YAML + bash) | After (.spec compiler) |
|---|---|
| `index.yaml` per directory | **Generated** by `specforge render` |
| `TRACE-SF-*.md` traceability files | **Generated** by `specforge trace` |
| `overview.md` document map | **Generated** — compiler knows all files |
| 10 bash validation scripts | **Gone** — compiler validates by construction |
| Manual cross-reference maintenance | **Gone** — `use` imports + named identifiers |
| 25 Claude skills for authoring | **Optional** — compiler enforces structure directly |

### What You Still Write

| Document | Why |
|---|---|
| `.spec` files | Source of truth — behaviors, invariants, features, decisions |
| `specforge.spec` project config | Thresholds, version |

> **Note:** Test files with `spec()` / `violation()` wrappers are documented in the coverage subsystem.

---

## Implementation Plan

### Technology: Rust CLI

A Rust crate producing two binaries:

```
specforge-cli        check, trace, render, stats, graph, migrate
specforge-lsp        Language Server Protocol
```

The parser is a library shared by both binaries.

### Distribution

- `npx specforge` (npm wrapper around binary)
- `brew install specforge`
- `cargo install specforge`
- GitHub releases (prebuilt binaries for mac/linux/windows)

### Build Order

| Step | Deliverable | Depends On |
|------|-------------|------------|
| 1 | **Tree-sitter grammar** — defines `.spec` syntax for ~~7 core block types~~ generic entity blocks + extension loading | — |
| 2 | **Parser** — `.spec` → AST (shared library) | 1 |
| 3 | **Resolver** — AST → in-memory graph, import resolution, symbol linking | 2 |
| 4 | **Validator** — graph → diagnostics (E001–E003, E005–E010, W001–W011, I001, I003–I004) | 3 |
| 5 | **CLI** — `check`, `trace`, `render`, `stats`, `graph`, `migrate` | 4 |
| 6 | **LSP** — diagnostics, go-to-def, references, hover, autocomplete, rename | 3 |
| 7 | **Emitters** — markdown, JSON, DOT graph, index.yaml | 3 |
| 8 | `@specforge/vitest` — TypeScript test runner plugin | 5 |
| 9 | `@specforge/pytest` — Python test runner plugin | 5 |
| 10 | `@specforge/go` — Go test collector | 5 |
| 11 | `specforge coverage` CLI — report merging + threshold gating | 8–10 |
| 12 | Extension API — Wasm/Extism interface for community extensions | 11 |

Steps 1–5 = usable tool. Step 6 = great DX. Step 7 = complete core. Steps 8–10 are parallelizable. Step 12 uses Wasm/Extism (RES-21b, RES-23).

---

## Comparison: Before vs. After (Core Compiler)

| Aspect | Before | After |
|---|---|---|
| Source of truth | 700 markdown files | `.spec` files |
| Storage | Filesystem + Neo4j | In-memory graph (rebuilt on compile) |
| Validation | 62 bash rules, post-hoc | Compiler, instant, as-you-type |
| Navigation | grep / manual search | LSP: go-to-def, find refs, rename |
| Traceability | 10 hand-written TRACE files | `specforge trace` — auto-generated |
| Cross-references | YAML strings | Typed imports + compiler-checked names |
| Index files | Manual `index.yaml` | Generated |
| Learning curve | 25 Claude skills + conventions | One syntax, one CLI |
| Dependencies | Node.js + bash + yq + sed | One binary |
| Language lock-in | TypeScript ecosystem | Universal — works with any project |
