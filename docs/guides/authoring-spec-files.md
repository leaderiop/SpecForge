# Authoring `.spec` Files — Zero to Hero

By the end of this tutorial you can write `.spec` files, read every diagnostic the compiler
gives you, and build a small project whose graph an AI agent can read and change safely.

This is the **tutorial**. Three companion guides go deeper once you've finished:

- **[Cookbook](spec-cookbook.md)** — task-oriented recipes ("how do I model X?").
- **[Best Practices](spec-best-practices.md)** — principles and named anti-patterns.
- **[Troubleshooting](spec-troubleshooting.md)** — diagnostic codes and how to fix them.

And for looking things up: **[Quick Reference](../quick-reference.md)** (cheat sheet) and
**[Entity Model](../entity-model.md)** (full field/edge/diagnostic reference).

> 💡 **Every `spec` block in this guide parses**, and a test holds it to that
> (`crates/kernel/parser/tests/guide_spec_blocks.rs`). The blocks of Acts III and IV are excerpts of
> [`examples/todo-app/`](../../examples/todo-app/): open it and run `specforge check` to see them compile together.

---

## Table of contents

- [Act I — Orientation](#act-i--orientation)
  - [What SpecForge is (and is not)](#what-specforge-is-and-is-not)
  - [The mental model: the graph is the product](#the-mental-model-the-graph-is-the-product)
  - [The five-minute win](#the-five-minute-win)
- [Act II — Core concepts](#act-ii--core-concepts)
  - [Anatomy of a `.spec` file](#anatomy-of-a-spec-file)
  - [Your first real entity, end to end](#your-first-real-entity-end-to-end)
  - [The edit → check → fix loop](#the-edit--check--fix-loop)
  - [Cross-linking: turning entities into a graph](#cross-linking-turning-entities-into-a-graph)
  - [Editing without a text editor](#editing-without-a-text-editor)
- [Act III — A tour of the builtin extensions](#act-iii--a-tour-of-the-builtin-extensions)
  - [Which kind should I use?](#which-kind-should-i-use)
  - [`@specforge/software`](#specforgesoftware)
  - [`@specforge/product`](#specforgeproduct)
  - [`@specforge/governance`](#specforgegovernance)
  - [`@specforge/formal`](#specforgeformal)
- [Act IV — The worked project](#act-iv--the-worked-project)
- [Act V — Mastery](#act-v--mastery)

---

# Act I — Orientation

## What SpecForge is (and is not)

SpecForge turns what you know about a domain — its things, how they relate, and the rules they
must obey — into **one typed, validated graph**. You write that knowledge in `.spec` files. The
compiler checks it and serves it to people and AI agents through the CLI, MCP and the LSP.

**The language is domain-free; the kinds you write come from your extensions.** The grammar knows
only five structural keywords: `spec`, `ref`, `use`, `defaults` and the reserved `define`. Every
other keyword — `behavior`, `feature`, `invariant`, `persona` — exists only because an installed
extension declares it, with its fields, its links and its rules.

This tutorial uses the **builtin extensions** (`@specforge/software`, `@specforge/product`,
`@specforge/governance`, `@specforge/formal`, `@specforge/testing`) because they ship with
SpecForge and describe a software project well. They are a starter set, not the base. A clinic,
a marketing team or a GxP quality unit would install other extensions and write other kinds the
same way — see [Model your domain](model-your-domain.md).

If you've used these tools, the analogy helps — but note the difference:

| Like… | …in that | Unlike, because SpecForge… |
|-------|----------|----------------------------|
| **Gherkin/Cucumber** | structured, human-readable intent | builds a *typed graph*; tests stay in your test runner |
| **TypeSpec / Protobuf** | a compiled schema language | takes its vocabulary from extensions, not a fixed schema |
| **Terraform HCL** | declarative, validated, version-controlled | describes knowledge for people and agents, not infrastructure to apply |

**SpecForge is NOT:**

- ❌ a code generator — agents produce the work; SpecForge gives them the facts and refuses changes that break the rules.
- ❌ a test framework — it *traces* tests and reads their results. The compiler never runs anything; `specforge collect` runs a runner's declared command only with your consent.
- ❌ a documentation format — the value is the *compiler*, not prose with conventions.

Here's the smallest meaningful `.spec` file in a project with `@specforge/software` and
`@specforge/testing`. You'll understand every line by the end of Act II:

```spec
spec "hello" {
  version "0.1.0"
}

behavior greet_user "Greet a user by name" {
  contract "Given a name, the system MUST return a personalized greeting."
  verify unit "returns 'Hello, Ada' for 'Ada'"
}
```

## The mental model: the graph is the product

This is the single most important idea in the guide.

> 💡 **Mental model.** You are not writing documents. You are declaring **nodes** (entities,
> like a behavior or a persona) and **edges** (references between them, like a behavior that
> *enforces* an invariant). The compiler assembles them into a graph, validates it, and serves
> it. **An agent's accuracy is proportional to how well-connected that graph is.**

From the project's [principles](../vision/principles.md):

> **Structure is a spectrum, not a binary.** "One entity in a `.spec` file is better than
> zero. […] A single behavior with two verify declarations already gives an AI agent more
> to work with than a page of prose." You never need full coverage to get value.

> **Validation is the value.** "What separates SpecForge from a fancy comment format is the
> compiler." It catches dangling references, unreferenced entities, missing test coverage, cycles, and
> contradictions — every error caught at compile time is a round-trip saved with an agent.

The practical consequence, which we'll repeat throughout: **maximize edges.** An entity whose
reference fields are filled in is far more valuable than one with only prose. The references
*are* the product.

## The five-minute win

> ⏱️ **~5 minutes.** Goal: create a project and see a validated graph. Don't try to understand
> everything yet — just get green output.

```bash
# 1. Create and enter a project directory
mkdir hello-spec && cd hello-spec

# 2. Scaffold a project with the extensions whose kinds you will write
specforge init --extensions @specforge/software,@specforge/product

# 3. Validate the starter spec
specforge check
#   => 0 errors

# 4. Export the graph an agent would read
specforge export --format context
```

`specforge init` writes a `specforge.json` config and a starter `spec/hello.spec` (enabling
`@specforge/software` also enables `@specforge/testing`, which gives kinds their `verify`
statements). Without `--extensions`, `init` enables nothing and the starter uses structural syntax
only. `specforge check` validates. `specforge export` emits the graph. Three commands, and you have
a validated project.

> ✅ **Checkpoint:** you have a project that compiles. Now let's understand what you wrote.

---

# Act II — Core concepts

## Anatomy of a `.spec` file

Every entity has the same shape, whatever extension declares its kind:

```text
keyword  name  "Optional Title"  {
  field  value
}
```

Annotated:

```spec
behavior greet_user "Greet a user by name" {
//  │        │            │
//  │        │            └─ optional title (string); auto-derived from name if omitted
//  │        └─ entity name: letters/digits/underscores, 2–60 chars, globally unique
//  └─ keyword: which kind of entity (declared by an installed extension)

  contract "Given a name, the system MUST return a greeting."
//  │        └─ a string value
//  └─ field key

  verify unit "returns a greeting"
//  └─ a verify statement: kind + description (from @specforge/testing)
}
```

**Field values** take the form their field's declared type asks for:

```spec
ref gh.issue:42 "Tasks need unique ids"

invariant task_id_uniqueness "Task ID Uniqueness" {
  guarantee "No two tasks may share the same id."
}

feature task_management "Task Management" {
  status     in_progress                 // enum: one word from the field's set, bare
  problem    "Users lose track of work." // string: text, quoted
  acceptance ["A user can create a task", "A user can mark a task done"]  // string list: opaque texts
}

behavior create_task "Create a task" {
  category   command                     // identifier: one bare word (a tag)
  contract """
    Multi-line text, triple-quoted.
    Indentation is stripped sensibly.
  """
  invariants [task_id_uniqueness]        // reference list: graph EDGES, compiler-checked
  features   [task_management]
  refs       [gh.issue:42]               // a scheme ref id links its `ref` entity
}
```

> 💡 The field's **declared type** decides both what a value references and how it is written:
> words and references bare, text quoted. `specforge format` writes every value so, and
> `specforge check` warns (W159) on the other form (ADR 0147).
> Every item of a `reference_list` field (and the value of a `reference` field) must resolve to
> a real entity of the kind the field names, or you get an error. A `string_list` field's items are
> opaque quoted strings. Reference fields build the graph; string lists don't.

**Comments** are line-only (`//`). References resolve across the whole project, so `use` is
optional: it records that a file depends on another (an `imports` edge), and a path that names no
file is E025.

```spec
use "types/user"                           // import a file (the .spec extension is implicit)
use { User, UserRole } from "types/user"   // selective import
```

**File defaults** state a field once for every block of one kind in the same file:

```spec
defaults behavior {
  features [task_management]
}

behavior complete_task "Complete a task" {
  category command
  contract "Given a task id, the system MUST mark the task done."
}
```

## Your first real entity, end to end

Let's build something real and watch the compiler help us. In a project with
`@specforge/software`, create `spec/tasks.spec`:

```spec
spec "first-steps" {
  version "0.1.0"
}

type Task "A unit of work" {
  id    string @readonly @unique
  title string
}

behavior create_task "Create a task" {
  category command
  types    [Task]
  contract "Given a title, the system MUST persist a Task with a unique id."
  verify unit "a created task has a non-empty id"
}
```

Run it:

```bash
specforge check
#   => 0 errors
```

Notice the edge: `create_task` lists `types [Task]`, which creates a
`BehaviorReferencesType` edge. The graph now *knows* this behavior operates on `Task`.

## The edit → check → fix loop

The core skill isn't memorizing fields — it's reading diagnostics and iterating. **Break it on
purpose:** change the behavior's `types` to reference a type that doesn't exist:

```spec
behavior create_task "Create a task" {
  category command
  types    [Tsak]          // typo!
  contract "..."
  verify unit "..."
}
```

```bash
specforge check
```
```
[E003] Error: unresolved reference 'Tsak' in entity 'create_task'
   ╭─[ spec/tasks.spec:9:13 ]
   │
 9 │   types    [Tsak]
   │             ──┬─
   │               ╰───
   │
   │ Help: did you mean 'Task'?
───╯
```

Read it like a pair programmer: **code** (`E003`), **message** (unresolved reference),
**span** (file, line and column), **help** (the likely fix). Fix the typo, re-run, get `0 errors`.
That loop — *edit → check → read → fix* — is how you author specs, and it is the same loop an AI
agent runs over MCP.

> 💡 Run `specforge explain E003` for a full description of any diagnostic code. The
> compiler reports **all** errors at once (it doesn't stop at the first), so you can fix in
> batches. (`E001` is a parse/syntax error; `E003` is an unresolved reference.)

**Severity levels:**

| Level | Example | Meaning |
|-------|---------|---------|
| Error | `E003` | Blocks compilation. Must fix. |
| Warning | `W002` | A likely problem (unreferenced, missing coverage). Doesn't block. |
| Info | `I080` | Advice. Always reported; `--severity info` shows only it. |

Use `specforge check --strict` in CI to treat warnings as errors.

## Cross-linking: turning entities into a graph

A pile of unconnected entities is barely better than prose. The leap to *graph* happens
when entities reference each other. Let's add a guarantee and connect it:

```spec
invariant task_id_uniqueness "Task ID Uniqueness" {
  guarantee "No two tasks may share the same id."
  risk medium
  verify property "concurrent creation never produces duplicate ids"
}

behavior create_task "Create a task" {
  category   command
  types      [Task]
  invariants [task_id_uniqueness]   // ← new edge: behavior ENFORCES invariant
  contract   "Given a title, the system MUST persist a Task with a unique id."
  verify unit "a created task has a non-empty id"
}
```

Now `specforge trace task_id_uniqueness` shows the behavior that upholds it, and
`specforge query create_task --depth 1` returns the behavior *plus* its type and invariant.
That neighborhood — a few KB — is what an agent reads instead of searching your files.

## Editing without a text editor

You can also change the graph with typed edits, which check the result before they write:

```bash
specforge new invariant task_id_uniqueness "Task ID Uniqueness" \
  --guarantee "No two tasks may share the same id." --in spec/tasks.spec
specforge link create_task invariants task_id_uniqueness
specforge set create_task category command
specforge link create_task invariants no_such_thing
#   error[E080]: the edit would leave the project with 1 error it does not have; nothing was written
```

An edit that would add an error is refused and nothing is written (E080). The MCP tool
`specforge.edit` applies the same edits for an agent, several at once, all or nothing — see
[Connect an agent](connect-an-agent.md).

> ✅ **Checkpoint:** you can author linked entities and self-correct from diagnostics.
> You're past "config file" and into "graph." Now let's meet the builtin vocabulary.

---

# Act III — A tour of the builtin extensions

The kinds in this act are what the four builtin extensions declare — 22 kinds between them.
This tour goes deep on the two most projects start with — `@specforge/software` and
`@specforge/product` — and gives a brief look at `@specforge/governance` and `@specforge/formal`.
`specforge model` lists the kinds, fields and links of whatever extensions *your* project enables.

## Which kind should I use?

| You're describing… | Use | Extension |
|--------------------|-----|-----------|
| A single operation's contract | **behavior** | software |
| A data shape (struct / union / error) | **type** | software |
| Something that happened (past tense) | **event** | software |
| An interface boundary / dependency | **port** | software |
| A rule that must always hold | **invariant** | governance |
| A user-facing capability | **feature** | product |
| A UX flow (persona × channel → features) | **journey** | product |
| A kind of user | **persona** | product |
| A delivery surface (web, cli, api) | **channel** | product |
| A code package | **module** | product |
| A planning phase | **milestone** | product |
| A shippable artifact | **deliverable** | product |
| A versioned bundle | **release** | product |
| A glossary definition | **term** | product |
| Why a choice was made | **decision** | governance |
| A non-functional requirement | **constraint** | governance |
| A risk (FMEA) | **failure_mode** | governance |
| A temporal/safety property | **property** | formal |
| An assumed-true foundation | **axiom** | formal |
| A multi-party protocol | **protocol** | formal |
| Abstract→concrete mapping | **refinement** | formal |
| A communicating process | **process** | formal |

If none fits, the answer is not to bend one: install or write an extension that declares the
kind you need ([Extending SpecForge](extending-specforge.md)).

> 💡 **One concept per entity.** "Create a task and email the user" is *two* behaviors. If
> you're tempted to cram, split instead — see the [God-entity anti-pattern](spec-best-practices.md#the-god-entity).

Each subsection below follows the same shape: **use when → minimal example → key fields →
✅/❌ → links to.**

## `@specforge/software`

### behavior

**Use when:** describing one operation the system performs.

```spec
behavior create_task "Create a task" {
  category   command
  invariants [task_id_uniqueness]
  types      [Task, CreateTaskCommand, InvalidTitleError]
  ports      [TaskRepository]
  produces   [task_created]
  contract """
    When a CreateTaskCommand is received, the system MUST reject an empty title
    with InvalidTitleError, otherwise persist a Task with a unique id and emit
    task_created. It MUST return Result<Task, InvalidTitleError>.
  """
  verify unit        "empty title is rejected"
  verify integration "created task is retrievable"
}
```

**Key fields:** `contract` (required — use RFC 2119 MUST/SHOULD/MAY); `category` (routing
tag); and the edge-creating lists `invariants`, `types`, `ports`, `produces`, `consumes`,
`features`.

✅ **Good** — one operation, RFC 2119, edges filled in (above).
❌ **Bad** — `contract "handles tasks"` with no references. Vague, edgeless, low value.

**Links to:** invariant, type, port, event, feature.

### type

**Use when:** defining a data shape. Kind is inferred from syntax: struct `{ … }`, union
`= a | b`, error (tagged with `_tag @literal`). Annotations: `@readonly`, `@unique`,
`@optional`, `@literal`. Arrays: `Item[]`.

```spec
type Task "Task" {
  id     string @readonly @unique
  title  string
  status TaskStatus
}

type TaskStatus "Task Status" = open | in_progress | done | archived

type TaskNotFoundError "Task Not Found Error" {
  _tag   "TaskNotFoundError" @literal
  taskId string
}
```

✅ Use `_tag` + `@literal` for discriminated unions; `@readonly`/`@unique` for identity.
❌ Don't put operations on a type — those belong on a **port**.

**Links to:** other types (`composed_types`, `extends`).

### event

**Use when:** recording that something happened. Name in **past tense**. Payload is a
**type reference**, not inline fields.

```spec
event task_created "Task Created" {
  channel  "tasks.created"
  payload  TaskCreatedPayload
  category domain
  verify integration "emitted after a successful create"
}
```

✅ Past-tense name, payload references a type.
❌ `event create_task` (imperative) or inline payload fields.

**Links to:** type (payload). Produced/consumed *by behaviors* (the edge lives on the
behavior's `produces`/`consumes`).

### port

**Use when:** defining an interface boundary (hexagonal architecture). `direction` is
**required**: `inbound`, `outbound`, or `bidirectional`. Methods return `Result`.

```spec
port TaskRepository "Persistence boundary for tasks" {
  direction outbound
  category  "persistence/task"
  method create(cmd: CreateTaskCommand) -> Result<Task, InvalidTitleError>
  method findById(id: string)           -> Result<Task, TaskNotFoundError>
  method list(status: TaskStatus)       -> Result<Task[], never>
  verify integration "contract satisfied by the SQLite adapter"
}
```

Conventions: `{Entity}Repository` (persistence), `{Service}Gateway` (external), `{Domain}API`
(inbound). A method naming a type no entity declares is E004.

**Links to:** used by behaviors (`ports [...]`).

### invariant

**Use when:** stating a rule the system must *never* violate. Fields: `guarantee`
(required), `risk` (`low`/`medium`/`high`/`critical`), `description`, `refs`.

```spec
invariant task_id_uniqueness "Task ID Uniqueness" {
  guarantee "No two tasks may share the same id."
  risk medium
  verify property "concurrent creation never produces duplicate ids"
}
```

> ⚠️ `invariant` has **no** `enforced_by` or `maintains` field. The link to a
> behavior lives on the *behavior* (`invariants [...]`); `enforced_by` is just the name of
> the inverse edge. (`maintains`/`requires`/`ensures` are added to *behaviors* by
> `@specforge/formal`.)

**Invariant vs. its neighbors:** "users can log in" is a *behavior*; "we use PostgreSQL" is
a *decision*; "p99 < 200ms" is a *constraint*. An invariant is falsifiable, universal, and
implementation-independent.

## `@specforge/product`

> Several product kinds require fields: `persona`/`channel` require `description`, `feature`
> requires `problem`, `journey` requires `flow`, `term` requires `definition`, `deliverable`
> requires `artifact_type`, `release` requires `version`.

### feature

**Use when:** describing a user-facing capability. Behaviors point *up* to features.

```spec
feature task_management "Task Management" {
  status   in_progress              // proposed | accepted | in_progress | done | deferred | deprecated
  priority high                     // critical | high | medium | low
  effort   m                        // xs | s | m | l | xl
  problem  "Users need to capture, track, and complete work items."
  solution "Create, complete, and list tasks backed by a durable repository."
  acceptance ["A user can create a task", "A user can mark a task done"]
}
```

✅ `problem` framed from the user's perspective; `solution` from the system's.
❌ Inventing a status like `wip` — only the enum values above are valid (else `E061`).

**Links to:** behaviors implement it; journeys/modules/milestones/personas reference it.

### journey

**Use when:** mapping a persona on a channel through features. `flow` (required) is an
ordered list of steps.

```spec
journey capture_and_complete "Capture and complete a task" {
  persona  individual
  channels [cli]
  features [task_management]
  priority high
  flow [
    "User runs `todo add \"Buy milk\"`",
    "System creates the task and prints its id",
    "User runs `todo done <id>` and the task is marked done",
  ]
}
```

**Links to:** persona, channel, feature.

### persona & channel

```spec
persona individual "Individual user" {
  description     "A person managing their own personal to-do list."
  technical_level beginner          // expert | advanced | intermediate | beginner | non_technical
  status          active            // active | deprecated
  goals        ["Capture tasks quickly"]
  key_features [task_management]
}

channel cli "Command-line interface" {
  description       "A terminal CLI for managing tasks."
  interaction_model request_response // request_response | event_driven | batch | streaming | bidirectional | manual
  status            active
}
```

### module, milestone, deliverable

```spec
module core "todo-core" {
  family   core
  features [task_management]
}

milestone mvp "Minimum Viable Product" {
  status        in_progress         // planned | in_progress | completed | blocked
  features      [task_management]
  modules       [core]
  exit_criteria ["create/complete/list verified", "zero E-level diagnostics"]
}

deliverable todo_cli "todo CLI" {
  artifact_type cli                  // cli|service|library|web_app|mobile_app|api|extension|documentation|package
  status        in_progress          // draft | in_progress | shipped | deprecated
  journeys      [capture_and_complete]
  modules       [core]
  milestones    [mvp]
}
```

> 💡 `depends_on` lists on `module`, `milestone`, `deliverable`, and `release` are
> **DAG-validated** — a reference cycle is reported as `W061`.

### term

```spec
term task "task" {
  definition "A unit of work the user wants to track."
  aliases  ["to-do", "item"]
  see_also [done]                    // see_also targets OTHER TERMS only
}
```

### release

A versioned bundle of deliverables/milestones. `version` is required and must be semver
(else `W093`). See [Getting started with `@specforge/product`](product-getting-started.md).

## `@specforge/governance`

Three kinds for decisions, quality bars and risks. (Concise coverage — one example each.)

```spec
decision sqlite_storage "SQLite for local storage" {
  status accepted                    // proposed | accepted | deprecated | superseded
  context "Single-user, local-first CLI needs durable storage without a server."
  decision "Use an embedded SQLite database file."
  consequences ["Zero-config persistence", "No multi-writer support"]
  invariants [task_id_uniqueness]    // links to the invariant it protects
}

constraint create_latency "Task creation is fast" {
  category    performance
  priority    medium                 // critical | high | medium | low
  description "Creating a task must feel instant on the CLI."
  metric      "create_task p99 < 50ms on local SSD"
  constrains  [create_task]
  verify load "benchmark create_task, assert p99 < 50ms"
}

failure_mode lost_task "Task acknowledged but not persisted" {
  invariant  task_id_uniqueness
  severity   high                    // critical | high | medium | low
  occurrence rare                    // certain | likely | occasional | unlikely | rare
  detection  moderate                // certain | likely | moderate | unlikely | undetectable
  cause      "Process killed before disk flush"
  effect     "User believes a task was saved but it disappears"
  mitigation "Write within a transaction; flush before acknowledging"
}
```

> ⚠️ FMEA scores are **enum words**, not 1–10 numbers (another value is `E061`).

## `@specforge/formal`

Five kinds for formal methods; requires `@specforge/software` as a peer and *enhances* its
kinds (adds `requires`/`ensures`/`maintains`/`satisfies` to behaviors, `expression` to invariants).

```spec
property no_lost_completion "Completion is never lost" {
  property_type safety               // safety | liveness | fairness
  expression "Once a task is `done`, it never silently reverts without a reopen."
  scope "task lifecycle"
}

// a behavior then declares it satisfies the property:
behavior complete_task "Complete a task" {
  category  command
  contract  "Given a task id, the system MUST mark the task done."
  satisfies [no_lost_completion]
}
```

The other four — `axiom`, `protocol`, `refinement`, `process` — are summarized in the
[Quick Reference](../quick-reference.md), with examples in the [Cookbook](spec-cookbook.md).
Machine-checkable bounds and claims are in the [Formal Verification Guide](formal-verification.md).

> ✅ **Checkpoint:** you know the builtin vocabulary and can pick the right kind. Time to build
> something complete.

---

# Act IV — The worked project

> ⏱️ **~20 minutes.** We build the **todo-app** from scratch, growing the spec one layer at
> a time and validating at each step. The finished project is at
> [`examples/todo-app/`](../../examples/todo-app/) — open it to follow along or check your
> work.

We'll author in the order recommended by [spec-writing-flow.md](../spec-writing-flow.md):
**foundation → data → guarantees → boundaries → behavior → product → governance**.

### Step 1 — Foundation

`specforge.json` declares the extensions; `spec/main.spec` holds the singleton block:

```json
{ "name": "todo-app", "version": "0.1.0", "spec_root": "spec",
  "extensions": ["@specforge/software", "@specforge/product",
                 "@specforge/governance", "@specforge/formal", "@specforge/testing"] }
```
```spec
spec "todo-app" { version "0.1.0" }
```
```bash
specforge check     # => 0 errors (an empty-but-valid project)
```

### Step 2 — Data (`spec/types/task.spec`)

Define the `Task`, its `TaskStatus` union, command payloads, error types, and event
payloads. (See the file in the example.) After adding it, `specforge check` reports
warnings like `W002` ("type not referenced by any behavior") — **expected**, because nothing
uses these types yet. The warnings will clear as we add behaviors. This is the graph telling
you about unconnected nodes.

### Step 3 — Guarantees (`spec/invariants/task.spec`)

```spec
invariant task_id_uniqueness "Task ID Uniqueness" {
  guarantee "No two tasks may share the same id."
  risk medium
  verify property "concurrent creation never produces duplicate ids"
}
```

### Step 4 — Boundaries (`spec/ports/task_repository.spec`) and events

Add the `TaskRepository` outbound port and the `task_created` / `task_completed` events.

### Step 5 — Behavior (`spec/behaviors/task.spec`)

This is where it all connects. Each behavior fills in its edges; the `use` lines record which
files this one depends on:

```spec
use "types/task"
use "invariants/task"
use "ports/task_repository"
use "events/task"
use "features/task_management"

behavior create_task "Create a Task" {
  category   command
  invariants [task_id_uniqueness]
  types      [Task, CreateTaskCommand, InvalidTitleError]
  ports      [TaskRepository]
  produces   [task_created]
  features   [task_management]
  contract """
    When a CreateTaskCommand is received, the system MUST reject an empty title
    with InvalidTitleError, otherwise persist a Task with a unique id and emit
    task_created. It MUST return Result<Task, InvalidTitleError>.
  """
  verify unit        "empty title is rejected"
  verify integration "created task is retrievable by id"
}
```

Once behaviors reference the types, ports, events, and invariants, the earlier `W002`
warnings disappear. **The graph is now connected.** (The example's file states the `ports` and
`features` every behavior shares once, in a `defaults behavior { … }` block.)

### Step 6 — Product layer

Add the `feature`, `persona`, `channel`, `journey`, `module`, `milestone`, `deliverable`,
and `term` (see `spec/features/` and `spec/product/` in the example). The behaviors already
point up to `task_management` via `features [...]`, so the feature is referenced.

### Step 7 — Governance & formal taste

Add the `decision`, `constraint`, `failure_mode`, and a `property` (wired to `complete_task`
via `satisfies`). Final validation:

```bash
specforge check
#   => 0 errors, 0 warnings, 3 infos   (I080: three entities without an owner)
```

### See what an agent sees

```bash
specforge stats
#   Entities: 30 … Edges: 42 … Declared: 100% of 16 testable

specforge trace task_management
```
```json
{
  "entity_id": "task_management", "entity_kind": "feature",
  "upstream": [
    { "entity_id": "capture_and_complete", "entity_kind": "journey",  "edge_label": "features", "depth": 1 },
    { "entity_id": "complete_task",        "entity_kind": "behavior", "edge_label": "features", "depth": 1 },
    { "entity_id": "core",                 "entity_kind": "module",   "edge_label": "features", "depth": 1 },
    { "entity_id": "individual",           "entity_kind": "persona",  "edge_label": "key_features", "depth": 1 }
  ]
}
```

(Shortened: the real output lists every upstream entity, each with a `status`.) From one feature,
the graph reaches the behaviors that implement it, the journey that exercises it, the persona
who wants it, and the module that houses it — then onward to the types, ports, events, and
invariants below. That traceable neighborhood, a few KB of JSON, is what replaces an agent's
expensive exploration.

> ✅ **Checkpoint:** you've built a complete, validated, multi-entity project.

---

# Act V — Mastery

You can now author and validate real specs. To go from competent to expert:

- **[Best Practices](spec-best-practices.md)** — the prescriptive rules (maximize edges,
  RFC 2119, one-concept-per-entity, naming) and **named anti-patterns** (God entity, unreferenced
  node, prose-only behavior, stringly-typed status) with fixes.
- **[Cookbook](spec-cookbook.md)** — copy-paste recipes for common modeling tasks.
- **[Troubleshooting](spec-troubleshooting.md)** — every common diagnostic code, what
  triggers it, and how to fix it.
- **[Model your domain](model-your-domain.md)** — the same method for kinds that are not software.
- **[Connect an agent](connect-an-agent.md)** — let Claude or any MCP client read and edit the graph.
- **[spec-writing-flow.md](../spec-writing-flow.md)** — the phased authoring flow in depth.
- **[Quick Reference](../quick-reference.md)** & **[Entity Model](../entity-model.md)** —
  the full field/edge/diagnostic reference of record.
- The per-kind **skills** under `.claude/skills/specforge-*-dsl/` carry the deepest
  field-by-field guidance for each builtin kind.

And the commands you'll lean on as you author:

```bash
specforge check --strict          # CI gate: warnings become errors
specforge check --severity error  # only the errors (the exit code still counts everything)
specforge check --severity info   # only the info-level advice
specforge explain W004            # what does a code mean?
specforge trace <id>              # traceability chain for an entity
specforge query <id> --depth 2    # neighborhood at a zoom level
specforge model                   # the kinds, fields and links your extensions declare
specforge format                  # auto-format .spec files
```

Welcome to hero status. The most expensive token is the one spent discovering what should
have been specified — and now you know how to specify it.
