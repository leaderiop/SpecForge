# The @specforge/rust Vision

> **Status.** This essay predates [ADR 0002](../../../docs/adr/0002-test-runner-extensions.md), which settled
> the mechanism: tests link themselves with the `#[specforge_test]` attribute, `@specforge/cargo-test`
> declares the `cargo test` command that `specforge collect` runs with the user's consent, and
> `@specforge/testing` scores coverage (`specforge analyze coverage`). Passages about mechanism are updated
> to match; the argument is unchanged. See the [Rust test tracing guide](../../../docs/guides/rust-test-tracing.md).

## Why This Exists

SpecForge compiles human intent into a validated typed entity graph. The `@specforge/software` extension gives that graph its vocabulary: behaviors, invariants, features, events, types, ports. But a graph of declared intent without proof of implementation is a wish list. The graph needs to know what is tested, what passes, and what is broken.

`@specforge/rust` closes the loop. It bridges the gap between what the spec says the system should do and what the test suite proves it actually does. It is the first language-specific integration in the SpecForge ecosystem, and it sets the pattern for every integration that follows.

This document is the vision for that integration. Not a roadmap. Not a technical specification. A statement of conviction about what the Rust integration must be, why it matters, and what world it helps build.

---

## The Traceability Loop

Rust's compiler is the best static verifier in mainstream systems programming. Borrow checker, lifetime analysis, exhaustive pattern matching, trait coherence — by the time `cargo build` succeeds, entire categories of bugs are structurally impossible. This is precisely why traceability matters *more* for Rust, not less.

The bugs that survive `rustc` are behavioral. The function compiles, the types align, the lifetimes check out — and it returns the wrong answer. The retry policy compiles with exponential backoff but caps at the wrong ceiling. The event handler compiles with the correct signature but swallows errors silently. The parser compiles against the grammar but rejects valid input on an edge case.

Rust's type system creates a false summit. Teams that climb it feel safe. They shouldn't. The gap between "compiles" and "correct" is where every production incident lives, and it is exactly the gap that spec traceability closes.

### Four levels, each a strict subset

Traceability is not binary. A behavior is not "tested" or "untested." There are four levels, and each is a strict subset of the one before it.

**Declared.** A behavior exists in a `.spec` file with at least one `verify` statement. This is intent. Someone thought hard enough about `PaymentProcessing` to write `verify "rejects expired cards"` and `verify "applies currency conversion"` and `verify "emits PaymentCompleted event"`. Three statements of what correctness means. Zero proof that anything works. But this alone is valuable — an agent reading this spec knows what the system *should* do, which is more than a codebase with no specs provides.

**Linked.** A test says which behavior it proves: `#[specforge_test(behavior = "payment_processing", verify = "rejects expired cards")]`. Someone connected intent to implementation, in the test itself, where the link can't rot when files move. But a link is still a claim, not evidence — the test might be `#[ignore]`d, or never run in CI.

**Executed.** `specforge collect` runs `cargo test` and records every annotated test that ran against the entity it names. Now we know: two of the three verify statements have corresponding tests that *ran*. The third — "applies currency conversion" — has no matching test. It is declared but unexecuted. The gap is visible.

**Passing.** Of the two executed tests, one passes and one fails. `payment_processing__rejects_expired_cards` is green. `payment_processing__emits_payment_completed_event` is red. The graph now carries proof: one verify statement is validated, one is falsified, one is dark. Three verify statements, three different statuses, three different actions required.

Each level is a strict subset. Everything passing was executed. Everything executed was linked. Everything linked was declared. But the converses are all false, and the gaps between levels are where the work lives.

### Compounding value

Each `specforge collect` run produces a point-in-time snapshot: which behaviors have tests, which tests pass, which verify statements are dark. That snapshot alone enriches the graph — an agent reading it knows the current state of every specified behavior, which is more than any amount of source code scanning provides.

Today, `specforge-report.json` is a single snapshot. It does not store history. But even without history, the latest snapshot compounds with the spec graph itself. New behaviors get added over time, verify statements accumulate, and each collection run maps the growing spec surface against the growing test surface. The graph carries the current state of that mapping — complete, structured, and queryable.

**Future: temporal analysis.** With report history (stored in CI artifacts, a database, or version control), the graph could track trends: which behaviors are stable, which are fragile, which have been dark since declaration. This requires infrastructure beyond what `specforge-report.json` provides today — a time-series of snapshots, not just the latest one. This is a forward-looking capability, not a current feature.

None of the current-state information exists in source code or test files alone. It exists only in the feedback loop between spec declarations and test results. The graph is a memory that the codebase itself doesn't have.

This is the compounding thesis: **traceability data appreciates.** The first collection is overhead. The fiftieth is infrastructure. With report history, the two-hundredth would be an oracle.

### Suspect links

Specs change. A developer rewrites `verify "rejects expired cards"` to `verify "rejects expired cards with grace period"`. The semantics shifted. The linked test — `payment_processing__rejects_expired_cards` — still passes, still maps by naming convention. But does it test the grace period? Almost certainly not. It was written before the grace period existed.

This is a suspect link: a traceability connection where the spec has changed since the test last validated it. The link exists. The test passes. And the traceability is a lie.

Suspect link detection compares spec content hashes against the timestamp of the last test execution that validated each verify statement. When a verify statement's text changes, every test linked to it becomes suspect until the next passing run. When a behavior's fields change — new references, modified description, added verify statements — all linked tests become suspect.

Suspect links make the feedback loop self-correcting. Without them, traceability degrades silently — specs evolve, tests calcify, and the mapping between them becomes fiction maintained by inertia. With suspect links, degradation is visible. The graph knows what it doesn't trust.

### The agent self-correction loop

An AI agent is asked to fix `PaymentProcessing`. Without traceability, it reads the source code, maybe finds some tests, guesses at intent, and produces a patch. Success rate: the industry baseline of roughly 30%.

With the traceability graph, it reads:

```
behavior PaymentProcessing {
  verify "rejects expired cards"          → PASSING (stable 4 months)
  verify "applies currency conversion"    → NO TEST (dark since declaration)
  verify "emits PaymentCompleted event"   → FAILING (regressed 2 days ago)
}
```

The agent doesn't start from scratch. It starts from a map with marked safe zones, known hazards, and recent damage. It can prioritize the regression (highest confidence fix), flag the dark verify (needs a test before any claim of correctness), and leave the stable path alone (risk of introducing bugs exceeds benefit).

Without traceability, every agent interaction is stateless — a fresh attempt against an opaque codebase. With traceability, every interaction builds on every previous one. The graph carries forward what worked, what failed, and what was never tried.

**This is what "traceability is a feedback loop, not a report" means in practice.** It is not a dashboard for humans to glance at quarterly. It is a structured memory that makes every subsequent AI interaction more accurate than the last.

---

## The Developer Journey

### Five minutes to proof

A Rust developer finishes reading a blog post about SpecForge. They are skeptical. They have seen traceability tools before — heavyweight, enterprise, requiring months of buy-in before delivering anything. They give it five minutes.

```bash
cargo install specforge-cli
cd my-service
specforge init
```

Three commands. A `specforge.json` file appears. A starter `spec/` directory with one example behavior. The developer writes a second behavior — something real from their codebase:

```spec
behavior validate_payment "Validate Payment Amount" {
  description "Rejects negative amounts, applies currency-specific rounding"
  verify unit "negative amount returns InvalidAmount error"
  verify unit "zero amount returns InvalidAmount error"
  verify unit "valid amount rounds to currency precision"
}
```

They already have tests for this. The tests are called `test_negative_payment`, `test_zero_payment`, `test_rounding`. Nobody remembers which requirement each test covers. The developer renames nothing. They add one dev-dependency, `specforge-test`, and in place of `#[test]` they say what each test proves:

```rust
#[specforge_test(behavior = "validate_payment", verify = "negative amount returns InvalidAmount error")]
fn test_negative_payment() {
    let result = validate_payment(Amount::new(-100, Currency::USD));
    assert!(matches!(result, Err(PaymentError::InvalidAmount { .. })));
}
```

One attribute is the only ceremony. They run:

```bash
specforge collect            # runs cargo test once they approve the command
specforge analyze coverage
```

And there it is. Three verify statements. Two have matching tests. One does not. The `valid amount rounds to currency precision` verify is uncovered — staring back at them in the terminal, impossible to ignore.

That is the moment. Not when they installed the tool. Not when they wrote the spec. The moment they saw a gap they did not know existed — in a codebase they thought they understood — in under five minutes.

### Progressive enhancement without rewrites

The journey from convention to precision is a gradient, not a migration.

**Day 1: One attribute.** The developer adds one dev-dependency, `specforge-test`, and writes `#[specforge_test(behavior = "validate_payment")]` instead of `#[test]` — or above `#[tokio::test]`, `#[rstest]`, parameterized tests, property tests. The proc macro composes: it registers the test itself, and defers to a runner attribute that already does. (Zero-dependency naming conventions — `validate_payment__rejects_negative` — remain a planned fallback; today the attribute is the link.)

**Day 7: Obligations.** They add `verify = "..."` to name the exact obligation each test proves, so coverage is per statement, not just per behavior.

**Day 30: CI gates.** Three lines in GitHub Actions:

```yaml
- run: specforge check --strict
- run: specforge collect --yes
- run: specforge analyze coverage --min <threshold>
```

**Day 90: Living documentation.** The coverage report is an artifact attached to every PR. Reviewers see which behaviors the PR affects. Suspect links flag stale tests.

At no point does the developer rewrite anything. Day 1 annotations still work on day 90. Each phase adds capability on top of the last. The migration cost between phases is zero because there is no migration — only addition.

### Team adoption path

SpecForge does not require organizational buy-in to deliver value. One developer, one spec file, one `specforge analyze coverage` — and the value is visible. The path is organic. One developer starts writing spec files for the behaviors they own. A colleague notices in a PR and asks what they are. The colleague writes a spec for their module. A tech lead notices that PRs with spec coverage are higher quality. They propose adding `specforge analyze coverage --min 80` to CI.

This is not a rollout plan. It is an adoption pattern. The difference matters. Rollout plans have timelines and executive sponsors. Adoption patterns have a first user who found it useful and a second user who saw the first user's results. SpecForge bets on the latter because that is how every enduring developer tool has spread.

---

## Agents That Know What's Tested

### The blind agent problem

Today, every AI coding agent starts blind.

Ask an agent to fix `create_user`. It opens the source file. It reads the function signature, the imports, maybe some comments. It scans surrounding files for context. It consumes 40,000 tokens on discovery alone — and still does not know the answer to the only question that matters: what is this function supposed to do, and which parts of that contract are already verified?

The agent does not know that `create_user` has three specified behaviors. It does not know that two of them have passing tests. It does not know that the third — email uniqueness validation — has a failing test at line 45 of `tests/behaviors/user_test.rs`. It does not know any of this because none of it is structured.

So the agent guesses. Maybe it gets the email validation right. Maybe it breaks the password hashing that was already working. Three round-trips to accomplish what should have been a single, targeted fix.

This is not a model quality problem. GPT-5 will not fix it. Claude 7 will not fix it. The problem is structural: the agent lacks a compiled, validated representation of what the code is supposed to do and how much of that contract is currently fulfilled. No amount of intelligence compensates for missing information.

### The informed agent

Now consider the same task with Rust traceability data in the graph.

The agent queries `specforge show create_user --depth=2 --format=json` and receives a structured response: three verify declarations, two passing tests mapped to their source locations, one failing test on `validates_email_uniqueness` at `tests/behaviors/user_test.rs:45`. The two passing tests cover `hashes_password_with_argon2` and `rejects_empty_username`. Both green. Both stable.

The agent does not touch password hashing. It does not touch username validation. It reads the failing test, reads the verify statement that defines the expected behavior, and produces a targeted fix. One round-trip. One fix. Nothing broken.

The graph does not make the agent smarter. It makes the agent informed. Intelligence without information produces hallucinations. Intelligence with structured, validated, up-to-date information produces correct results.

### Spec as prompt

A verify declaration is not metadata. It is an acceptance criterion with a deterministic name.

```
behavior create_user {
  verify unit "hashes password with argon2"
  verify unit "rejects empty username"
  verify unit "validates email uniqueness"
}
```

Each verify statement tells the agent three things: what to test, what kind of test it is, and what to name the test function. The naming convention is deterministic: `create_user__hashes_password_with_argon2`, `create_user__rejects_empty_username`, `create_user__validates_email_uniqueness`. No ambiguity. No invention. The spec dictates the test surface.

This matters because naming is where agents waste the most creativity on the least important decision. With deterministic naming from verify declarations, the loop never breaks. The spec defines the name. The agent generates the function. The test runner produces a result. The result maps back to the spec by name. The graph updates. No human interpretation required at any step.

Verify statements are prompts that the compiler validates. They are acceptance criteria that the graph tracks. They are test names that the convention enforces. One declaration, three functions, deterministic mapping.

### Token efficiency

Context windows are finite. Every token spent on discovery is a token not spent on reasoning.

A blind agent understanding test coverage for twenty behaviors consumes 40,000-60,000 tokens on file scanning. The same agent with graph access runs one query: `specforge export --entity-kind=behavior --include=coverage --format=json`. The entire payload fits in 2,000 tokens. Complete — no missed files, no misinterpreted names, no discovery overhead.

A 20x reduction in context consumption for coverage information alone. That is 38,000 tokens returned to the agent's reasoning budget. Token efficiency is agent efficiency. Agent efficiency is first-attempt accuracy. First-attempt accuracy is the entire point.

---

## The First of Many

`@specforge/rust` is not just a plugin. It is the canonical answer to the question: how does a language-specific integration work in the SpecForge ecosystem? Every design decision made here becomes the pattern that `@specforge/typescript`, `@specforge/python`, `@specforge/go`, and `@specforge/java` follow.

### Rust as the reference implementation

Rust's test infrastructure is among the most constrained. libtest has no reporter plugin API. `#[test]` is a compiler lang item that cannot be hooked. JSON output has been unstable since 2018. Multiple test binaries fragment results across a workspace.

If the SpecForge integration model works cleanly under these constraints, it works everywhere. Python's pytest has rich plugin hooks. TypeScript's vitest has custom reporters. Go's `testing` package emits structured JSON natively. Every one of these is strictly easier than Rust.

This is why Rust goes first. Not because it is the most convenient target, but because it is the hardest. A pattern forged against Rust's constraints will never break against Python's flexibility.

The reference implementation establishes four things that every subsequent integration must provide:

1. **A runner extension** that declares how its runner is detected, the command that runs it, and a pure mapping from its native report to entity results. The user runs `specforge collect` — detection picks the runner — and the results land in `specforge-report.json`.
2. **An annotation mechanism** (proc macro, decorator, doc comment, build tag) that explicitly links test functions to spec entities.
3. **A naming convention** (`{entity_id}__{description_slug}`) that provides zero-config mapping as a fallback (planned).
4. **No test paths in `.spec` files.** Linkage lives in the tests, so it survives refactors.

### The universal report format

`specforge-report.json` is language-agnostic by design. A Rust project produces the same schema as a TypeScript project. The schema cares about entity IDs, test names, pass/fail status, and duration. It does not care whether the test runner was cargo-nextest, vitest, pytest, or `go test`.

**The report format is the contract, not the collection mechanism.** Collection is language-specific and messy, so each test runner gets its own small extension that runs it and maps its native report. What comes out the other side is clean, uniform, and universal. A polyglot monorepo with three languages runs three runners; `specforge collect` merges their results into one `specforge-report.json`. The coverage gate does not know or care which language produced which result.

### The two-level resolution model

Entity-to-test mapping follows a strict precedence hierarchy:

```
1. Language-specific annotation     (explicit in-code linkage — always wins)
2. Naming convention                (implicit, zero-config fallback — planned)
```

Spec files carry no test paths: a path in a `.spec` file rots the moment a test moves, so linkage lives in the test itself. The annotation varies by language:

| Language   | Annotation |
|------------|---------------------|
| Rust       | `#[specforge_test(behavior = "entity_id")]` proc macro (shipped) |
| TypeScript | vitest `meta: { specforge: { behavior: "entity_id" } }` (shipped) |
| Python     | `@specforge.test(behavior="entity_id")` decorator |
| Go         | `//specforge:behavior entity_id` build-tag comment |
| Java       | `@SpecforgeTest(behavior = "entity_id")` annotation |

Each is the idiomatic way to attach metadata in its language. The pattern is the same; the syntax respects each ecosystem's conventions.

### The polyglot future

A team with a Rust backend and a TypeScript frontend enables both `@specforge/cargo-test` and `@specforge/vitest`. `specforge collect` runs both and merges their results. One traceability matrix. One coverage number. One CI gate. The backend team and the frontend team see the same spec graph and contribute to the same coverage metric.

This is not a convenience feature. It is the architectural consequence of making the report format language-agnostic. The moment you decouple the report schema from the collection mechanism, polyglot traceability becomes free.

---

## Rust-Native by Design

### Working WITH Rust, not against it

Every design choice respects Rust's constraints. No unstable features required. No custom test harness. No `build.rs` magic. The proc macro composes with existing test attributes. The collection reads a report format SpecForge owns.

The Rust integration requires zero unstable features. It does not require nightly. It does not replace `cargo test` or pretend to be a test runner. It provides a proc macro that takes the place of `#[test]` and records which spec entity each test covers, and `@specforge/cargo-test` reads those per-binary records into `specforge-report.json`. The integration is a thin observation layer over machinery that already works.

### The proc macro philosophy

`#[specforge_test(behavior = "create_user")]` replaces `#[test]`: it registers the test and links it in one attribute, so the two can't drift apart (ADR 0002). Next to a runner attribute it steps aside. This is how Rust works. `#[tokio::test]` wraps to provide an async runtime. `#[rstest]` wraps for parameterized fixtures. `#[traced_test]` wraps for tracing output. Each is a layer, not a takeover. They compose because none claims ownership of the test lifecycle.

SpecForge's macro is one more layer. It expands to the test (registered by itself, or by the runner attribute below it) plus a `TestGuard` — a struct whose `Drop` implementation detects whether the test panicked and records the result. Deterministic cleanup via RAII, no runtime hooks. The guard records results into a process-local, thread-safe registry — the only shared state, flushed to disk on process exit.

### Minimal dependency footprint

`specforge-test` adds ~12-15 transitive crates (serde, serde_json, proc-macro2, syn, quote). Comparable to adding `tracing`. If you use `serde`, `tokio`, `clap`, or `tracing`, you already have them. Rust developers are allergic to bloated dependency trees. This one earns its weight. And Phase 1 requires zero dependencies at all.

### Workspace-aware by default

Each test binary writes its own mapping file to `target/specforge/<binary-name>.json`. The collector merges all per-binary files into a single `specforge-report.json`. One crate or fifty crates, the pipeline is identical. Workspace complexity is invisible to the user.

### The report the attribute writes

Stable libtest has no machine-readable output, and JUnit XML from nextest carries no link to a spec entity. So the attribute writes its own per-binary report — entity, test, obligation, outcome, duration — and `@specforge/cargo-test` reads that. It works the same under `cargo test` and `cargo nextest run`.

### What we do NOT do

No custom test harness. No unstable features. No `build.rs` injection. No nightly requirement. No reimplementation of `cargo test`. No mandatory runtime. The boundary is clear: SpecForge observes your tests and maps them to your specs. It does not modify your tests or replace your test infrastructure. When `specforge collect` runs `cargo test`, it runs exactly the command the extension declares, after you approve it — your runner, your flags, your output. The line does not move.

---

## CI That Validates Intent

A green build means tests pass. It says nothing about whether those tests cover what you specified.

A project can have 100% code coverage — every line executed, every branch taken — and 0% spec coverage. Every behavior in the `.spec` files can be unverified. The build is green. The graph is red. Nobody knows because nobody is checking.

### The three-stage pipeline

**Stage 1: Spec Validation** (`specforge check --strict`) — Under one second. Validates references, detects orphans, checks graph consistency. The cheapest check with the highest signal-per-second ratio. This is a core command.

**Stage 2: Test Execution and Collection** (`specforge collect --yes`) — Expensive. Compilation and tests take minutes. Runs second because if the spec graph is broken, there is no point compiling. The command that runs is the one `@specforge/cargo-test` declares.

**Stage 3: Spec Coverage Gate** (`specforge analyze coverage --min 90`) — Reads the graph and the recorded results, computes spec coverage, fails the build if coverage drops below threshold. The analysis is `@specforge/testing`'s coverage pass.

Three stages. Seconds, then minutes, then milliseconds. Cheap checks first. Expensive work in the middle. Gates at the end.

### Spec coverage as a first-class metric

"90% of our specified behaviors have passing tests" is a stronger statement than "87% of lines were executed." The first tells you that 90% of what you said the system should do has been verified. The second tells you that 87% of your code was touched during testing — which includes error handlers, logging statements, and utility functions. Both metrics matter. But spec coverage is the one that answers the question stakeholders actually ask: "does the system do what we said it should do?"

### Progressive strictness

**Day 1:** `specforge check` — informational. **Day 30:** `specforge check --strict` — warnings are errors. **Day 90:** `specforge analyze coverage --min 50` — low bar. **Day 180:** `specforge analyze coverage --min 90` — high bar. Teams ratchet up as coverage grows. The ratchet only moves in one direction.

---

## An Extension, Not a Feature

### Why Rust knowledge does NOT belong in the compiler core

It would take an afternoon to hardcode Rust collection into the CLI. It would also be the first crack in the architecture. The moment Rust-specific logic enters the compiler core, the compiler knows something about a language. These are facts about Rust, not facts about specification graphs. They do not belong in the core.

There is no "pragmatic Phase 1" exception. `specforge collect` is generic: it asks the enabled runner extensions for their collectors, runs the command a collector declares (only after the user approves it for the project), and passes the report to the collector's pure Wasm export. Every fact about Rust — the command, where the report lands, how to read it — lives in `@specforge/cargo-test`. The core knows none of it.

### The runner-extension pattern

`@specforge/cargo-test` is small by design. It declares one collector: the file that selects it (`Cargo.toml`), the command (`cargo test --workspace --no-fail-fast`), the report location, and a pure function that maps per-binary reports to entity results. It declares no entity kinds and no validators. It sits at the boundary between a language ecosystem and the SpecForge graph.

The host runs the command; the extension only reads bytes. The report schema is a contract. The extension honors one side. The core honors the other. Neither reaches across.

### Peer dependency composition

`@specforge/cargo-test` requires one peer extension:

- **@specforge/testing** gives `verify` its meaning, makes kinds testable, and scores coverage from the recorded results

`@specforge/software` provides the entity vocabulary those results attach to. The runner produces data; the extensions give that data meaning in the graph — clear inputs, clear outputs, no shared mutable state.

### Commands stay generic

The core CLI contains structural graph operations — `init`, `check`, `export`, `query`, `trace`, `format`, `stats`, `analyze` — and `collect`, which is generic in the same way `analyze` is: `analyze` runs whatever passes extensions contribute, `collect` runs whatever collectors they contribute. Neither knows a language.

This follows Principle 2 exactly. Terraform's core has `init`, `plan`, `apply`, `destroy` — structural operations on infrastructure state. All cloud knowledge lives in providers. All test-runner knowledge lives in runner extensions. `specforge collect` with no argument matches each collector's detection files (`Cargo.toml` → cargo-test, `vitest.config.*` → vitest) and runs every runner the project uses; `--runner` picks one.

### Separate release cycle

The proc macro crate on crates.io evolves independently of the compiler. When `custom_test_frameworks` stabilizes, @specforge/rust updates. The compiler does not know this happened. When a new test runner appears, it gets its own runner extension. The compiler does not know this happened. The compiler's release cycle is governed by graph protocol evolution, not by the release schedules of `rustc` or `cargo`.

### The extension test

Principle #7's test: "Could this be an extension instead?" Apply it: does it require changes to the parser grammar? No. The graph engine? No. The resolver? No. The validator? No. Every answer is no. The extension boundary is not a compromise — it is the natural boundary.

If tomorrow someone builds @specforge/zig, they follow the same pattern. Zero compiler changes. That ordinariness is the point.

---

## Start Where You Are

### Low-friction entry

One dev-dependency, one attribute in place of `#[test]`, one `specforge collect`. No custom harness, no build script, no nightly. Zero-dependency naming conventions are the planned next step down in friction.

### The spectrum of precision

A test can name just the behavior it covers, or the exact `verify` obligation it proves. Critical behaviors get per-obligation links; the rest can start coarse. No all-or-nothing.

### Retroactive adoption

An existing Rust project with 500 tests can adopt SpecForge without renaming anything. Write a few .spec files, swap `#[test]` for `#[specforge_test(...)]` on the tests that prove them, run `specforge collect`. The existing tests gain traceability without changing what they test.

### The ratchet

Teams start with 5% spec coverage. Each sprint, they add a few more behaviors. `specforge analyze coverage --min` ratchets up. Regression is prevented. Progress is irreversible. This is how real adoption works — not a mandate from above, but a monotonically increasing floor, enforced by CI, driven by the natural rhythm of development.

### One file is enough

A single `spec/behaviors/auth.spec` file with three behaviors and their verify declarations already gives an AI agent more information than 20 pages of README. The minimum viable spec is tiny. The maximum is unbounded.

---

## What Nobody Else Does

### The six-layer moat

No existing tool provides all six capabilities:

1. A specification language with compiler validation
2. Test intent declaration in the spec (verify/scenario)
3. Linkage from spec entities to actual test files
4. Consumption of test results for pass/fail proof
5. A four-level traceability matrix (declared → linked → executed → passing)
6. Designed for AI agent consumption

TRLC gets layer one. Serenity BDD gets layers four and five. Allure gets layer four. DOORS/Jama/Polarion cover layers one through five, behind a GUI, behind a six-figure contract. No tool provides all six. SpecForge does.

### Spec-as-prompt

The killer differentiator. A `verify` statement is three things simultaneously:

```
verify unit "validates email uniqueness under concurrent registration"
```

For a **human reviewer**, this is a requirement. For a **test framework**, this is an expectation mapped to a function name via deterministic slugification. For an **AI agent**, this is an instruction — what to test, what property to verify, what the acceptance criterion is.

This triple identity — requirement, expectation, instruction — is the architecture. Write it once, and three consumers use it for three different purposes. No other tool treats specifications as agent prompts.

### The feedback loop advantage

Allure produces beautiful reports. And they are endpoints. They do not feed back into anything. SpecForge's traceability feeds back into the graph, making the next agent task more accurate. Reports do not compound. Feedback loops do.

### Developer-first, not enterprise-first

DOORS costs $10,000 per seat. Polarion requires a dedicated administrator. SpecForge is `cargo install specforge-cli`. One command. No account. No license key. A developer can go from zero to a validated spec graph in sixty seconds, alone, without asking anyone for permission.

### The standard play

`specforge-report.json` is an open format. If Allure adds an exporter, SpecForge wins. If a new Rust test framework adopts the format natively, SpecForge wins. The moat is the standard, not the tool. Ten compilers producing the same graph schema is success. The value is in the shared contract that an ecosystem forms around.

---

## Strengthening the Standard

### Test traceability as a graph primitive

Every entity in the graph has two kinds of relationships: structural and operational. Structural relationships describe architecture — a behavior references a port. Operational relationships describe reality — a behavior has four tests, three pass. Both belong in the graph because agents need both to do their jobs.

The Graph Protocol represents coverage as a four-level progression on every testable entity. These levels are not metadata. They are graph state. An agent reading the graph sees them the same way it sees edge labels and field values — as structured, typed, queryable data.

### The report format as a micro-standard

`specforge-report.json` is deliberately simple. An array of entities, each with test results. The entire schema fits in a compact JSON Schema. A junior developer can write an adapter in an afternoon. Any test runner that emits this format automatically participates in the traceability loop.

### Multi-language graph consistency

A polyglot project's graph contains Rust behaviors and TypeScript behaviors. Both have the same coverage fields. Both show the same four-level progression. The graph schema doesn't know or care about the source language. This is what a standard means.

### The network effect

Every language integration that produces `specforge-report.json` makes every other integration more valuable. Every AI agent that learns to read coverage data from the Graph Protocol benefits from all language integrations simultaneously. The flywheel accelerates with each new integration.

The graph without coverage is a blueprint. The graph with coverage is a living system. Standards win when they describe living systems.

---

## What This Is NOT

**Not a test framework.** `cargo test` and `cargo-nextest` run tests. SpecForge's compiler never executes anything; `specforge collect` runs the one command `@specforge/cargo-test` declares, after you approve it, and reads the results. The boundary is absolute.

**Not a code coverage tool.** Spec coverage and code coverage are orthogonal. Both matter. Both should be measured. They answer different questions. @specforge/rust measures spec coverage — "did we test what the spec says?" — not code coverage — "did our tests touch all the code?"

**Not a Rust framework.** @specforge/rust does not change how you write Rust code, structure your crates, or organize your tests. It observes what you already do and maps it to your specifications.

**Not the last language integration.** It is the first. And the pattern it establishes — a runner extension, an in-test annotation, a universal report format — scales to every language an AI agent will ever need to understand.

---

*@specforge/rust closes the loop between what the spec says and what the tests prove. The graph remembers. The agent reads. The system improves. Every test run makes the next agent task more accurate. This is not a feature. It is the mechanism by which structured intent becomes verified reality.*
