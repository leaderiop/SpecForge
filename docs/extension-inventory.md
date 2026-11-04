# Extension Inventory

What you get from this page: the catalogue of the extensions that ship with SpecForge, and exactly what each one
declares. They are a **starter set**: examples of what extensions do, aimed at software and product work. None of
them is part of the core, a project loads only the ones it lists in `specforge.json`, and your own extensions use
the same SDK ([extension model](extension-model.md)).

Together the four vocabulary builtins declare 22 entity kinds and 56 edge types. Six more declare no kinds:
`@specforge/testing` and the runner extensions `@specforge/cargo-test` and `@specforge/vitest` handle test
obligations and results ([ADR 0002](adr/0002-test-runner-extensions.md)); `@specforge/rust` and
`@specforge/typescript` read source code for `specforge infer`. `@code/core`, the first builtin of a family
(ADR 0328), declares the two collector-written kinds of test facts ([ADR 0329](adr/0329-code-core-holds-test-facts-and-their-ids.md)), and
`@javascript/node-test` collects them from `node --test`. `@specforge/checks` declares the generic value
checkers, which any field's `checks` can name ([ADR 0338](adr/0338-one-foundation-extension-holds-the-generic-value-checkers.md)),
and one collector-written kind, `check_outcome`, which its `checks` collector publishes when named or under
`checks.publish`.

The counts and lists on this page are read from the builtins' pinned declarations
(`crates/runtime/component/tests/declarations/`); tests hold the kind count and the edge tables to them.

## At a glance

| Extension | Kinds | Edge types | Adds to other kinds | Peers |
|-----------|-------|------------|---------------------|-------|
| `@specforge/product` | 9 | 20 | -- | none |
| `@specforge/software` | 4 | 15 | `module`, `milestone` | product, governance (both optional) |
| `@specforge/governance` | 4 | 11 | -- | software, product (both optional) |
| `@specforge/formal` | 5 | 10 | `behavior`, `event`, `invariant` | software, governance (both optional) |
| `@specforge/testing` | 0 | 0 | `verify` kinds on 7 kinds | software, governance (both optional) |
| `@specforge/cargo-test` | 0 | 0 | -- | testing (required) |
| `@specforge/vitest` | 0 | 0 | -- | testing (required) |
| `@specforge/rust` | 0 | 0 | -- | none |
| `@specforge/typescript` | 0 | 0 | -- | none |
| `@code/core` | 2 (collector-written) | 1 | -- | none |
| `@javascript/node-test` | 0 | 0 | -- | code/core (required) |
| `@specforge/checks` | 1 (collector-written) | 0 | -- | none |

Extensions load after the peers they declare; extensions with no peer between them keep the order
`specforge.json` lists them in (ADR 0041). A required peer must be loaded (E027); an optional one, when loaded,
must be in the declared version range.

## In progress and not started

| Family | State |
|--------|-------|
| `@gsd/*` (the GSD spec framework modelled as extensions) | Being designed; nothing built yet |
| BMAD, OpenSpec, SpecKit | Not started |

## @specforge/product

Product planning and delivery: what is built, for whom, through which channels, when it ships. Nothing in it is
software-only; a clinic, a course or a campaign has features, personas, journeys and milestones too.

### Entity kinds (9)

| Kind | Description |
|------|-------------|
| [`feature`](entities/feature.md) | A unit of value, with problem and solution framing |
| [`journey`](entities/journey.md) | One persona's path through some features, on some channels |
| [`deliverable`](entities/deliverable.md) | Something that ships, bundling journeys and modules |
| [`milestone`](entities/milestone.md) | A planning phase with scheduled features and exit criteria |
| [`module`](entities/module.md) | A structural unit (package, subsystem, workstream) that builds features |
| [`term`](entities/term.md) | A glossary entry |
| [`persona`](entities/persona.md) | A kind of person who uses what you build |
| [`channel`](entities/channel.md) | The medium people reach it through |
| [`release`](entities/release.md) | Deliverables and milestones that ship together |

### Edge types (20)

| Edge Type | Source | Target | Semantics |
|-----------|--------|--------|-----------|
| `FeatureDependsOn` | feature | feature | This feature depends on that feature |
| `FeatureRelatesTo` | feature | feature | This feature is related to that feature |
| `JourneyExercisesFeature` | journey | feature | This UX flow exercises these features |
| `JourneyTargetsPersona` | journey | persona | This journey is performed by this persona |
| `JourneyUsesChannel` | journey | channel | This journey occurs through this channel |
| `DeliverableSupportsJourney` | deliverable | journey | This deliverable ships these journeys |
| `DeliverableContainsModule` | deliverable | module | This deliverable includes these modules |
| `DeliverableTrackedByMilestone` | deliverable | milestone | This deliverable targets these milestones |
| `DeliverableDependsOn` | deliverable | deliverable | This deliverable depends on that deliverable |
| `MilestoneDeliversFeature` | milestone | feature | This phase schedules these features |
| `MilestoneScopesModule` | milestone | module | This milestone includes these modules |
| `MilestoneDependsOn` | milestone | milestone | This milestone depends on that milestone completing first |
| `ModuleContainsFeature` | module | feature | This module implements these features |
| `ModuleDependsOn` | module | module | This module depends on that module |
| `TermReferencesRelatedTerm` | term | term | This term cross-references that term |
| `TermBelongsToModule` | term | module | This term belongs to this module's domain |
| `ReleaseIncludesDeliverable` | release | deliverable | This release ships these deliverables |
| `ReleaseCompletesMilestone` | release | milestone | This release targets these milestones |
| `ReleaseDependsOn` | release | release | This release depends on that release |
| `PersonaPrioritizesFeature` | persona | feature | This persona prioritizes this feature |

### Also declares

- A shared field, `tags` (string list), on all nine kinds.
- 47 rule declarations (unreferenced entities, dependency cycles, missing owners, missing reasons, non-SemVer
  versions, ...), each listed on its kind's page.
- Two passes: `lifecycle` (every compile: a completed milestone delivering a feature that is not done, W154, and
  similar contradictions across entities) and `delivery_evidence` (`specforge analyze`: a `done` feature the
  recorded tests do not prove, I071).
- 40 commands, `specforge product <command>` on the CLI and the same names as MCP tools: the list commands
  (`features`, `journeys`, ...) and graph queries such as `milestone-completion`, `critical-path`,
  `deliverable-traceability`, `owner-workload`, `coverage-matrix` and `health`. `specforge product --help` lists
  them all.

## @specforge/software

Software design: behavioral contracts, domain events, data shapes and interfaces. Its product links
(`features`, and the fields it adds to `module` and `milestone`) need `@specforge/product`; without it they are
I004 advice and the additions are skipped.

### Entity kinds (4)

| Kind | Description |
|------|-------------|
| [`behavior`](entities/behavior.md) | One operation's contract |
| [`event`](entities/event.md) | Something the system announces |
| [`type`](entities/type.md) | A data shape (struct or union) |
| [`port`](entities/port.md) | An interface contract with typed methods |

### Edge types (15)

| Edge Type | Source | Target | Semantics |
|-----------|--------|--------|-----------|
| `References` | any | any | General cross-reference |
| `BehaviorImplementsFeature` | behavior | feature (product) | This behavior implements this feature |
| `BehaviorProducesEvent` | behavior | event | This behavior emits these events |
| `BehaviorConsumesEvent` | behavior | event | This behavior reacts to these events |
| `BehaviorReferencesType` | behavior | type | This behavior uses these type definitions |
| `EventCarriesPayloadType` | event | type | This event carries this payload type |
| `TypeComposesType` | type | type | This type composes or extends that type |
| `BehaviorUsesPort` | behavior | port | This behavior uses these port interfaces |
| `BehaviorEnforcesInvariant` | behavior | invariant | This behavior enforces these invariants |
| `TypeExtendsType` | type | type | This type extends that type (inheritance) |
| `PortReferencesType` | port | type | This port names this type in a method parameter or return type |
| `ExternalRef` | any | URI | This entity links to an external reference |
| `MilestoneIncludesBehavior` | milestone (product) | behavior | This milestone delivers these behaviors |
| `ModuleConsumesPort` | module (product) | port | This module consumes these ports |
| `ModuleDefinesPort` | module (product) | port | This module defines these ports |

### Adds to other kinds

| Kind | Owner | Added fields |
|------|-------|--------------|
| `module` | `@specforge/product` | `ports` (reference list → port), `ports_defined` (reference list → port) |
| `milestone` | `@specforge/product` | `behaviors` (reference list → behavior) |

### Also declares

Eight rules (W001, W002, W005–W008, W010, E010). Two are custom validators written in the extension's
code: milestone behavior references (E010) and type member annotations (W010). The host reports a type name that
names no type (E004). A starter `.spec` file `specforge init` writes.

## @specforge/governance

Rules that must always hold, decisions, measurable limits and risk. Its links into software and product kinds are optional: without those
extensions they are I004 advice.

### Entity kinds (4)

| Kind | Description |
|------|-------------|
| [`invariant`](entities/invariant.md) | A rule that must always hold |
| [`decision`](entities/decision.md) | A recorded choice with context, alternatives and consequences |
| [`constraint`](entities/constraint.md) | A measurable quality or limit, with metric and threshold |
| [`failure_mode`](entities/failure-mode.md) | An FMEA entry: cause, effect, ratings, mitigation |

### Edge types (11)

| Edge Type | Source | Target | Semantics |
|-----------|--------|--------|-----------|
| `DecisionProtectsInvariant` | decision | invariant | This decision protects these invariants |
| `ConstraintEnforcedByBehavior` | constraint | behavior (software) | This constraint applies to these behaviors |
| `DecisionSupersedesDecision` | decision | decision | This decision supersedes that decision |
| `ConstraintConstrainsBehavior` | constraint | behavior (software) | This quality requirement constrains these behaviors |
| `ConstraintProtectsInvariant` | constraint | invariant | This constraint protects these invariants |
| `FailureModeTargetsInvariant` | failure_mode | invariant | This failure mode threatens this invariant |
| `ConstraintGovernsFeature` | constraint | feature (product) | This constraint governs these features |
| `DecisionAffectsFeature` | decision | feature (product) | This decision affects these features |
| `FailureModeThreatensFeature` | failure_mode | feature (product) | This failure mode threatens these features |
| `DecisionImposesConstraint` | decision | constraint | This decision imposes these constraints |
| `FailureModeAffectsBehavior` | failure_mode | behavior (software) | This failure mode affects these behaviors |

`failure_mode` declares `severity`, `occurrence` and `detection` (and their `post_` forms) as word vocabularies
(E061 otherwise); a written `rpn` is not checked against them.

One rule (W003): an invariant nothing references.

## @specforge/formal

Formal methods: temporal properties, assumptions, protocols, refinement between specification layers, and
communicating processes. Requires `@specforge/software`. See [Formal verification](guides/formal-verification.md).

### Entity kinds (5)

| Kind | Description |
|------|-------------|
| `property` | A temporal or behavioral assertion (safety, liveness, ...) |
| `axiom` | Something assumed true without proof |
| `protocol` | A synchronization contract events share |
| `refinement` | A mapping from an abstract behavior to a concrete one |
| `process` | A CSP-style communicating process with an alphabet |

### Edge types (10)

| Edge Type | Source | Target | Semantics |
|-----------|--------|--------|-----------|
| `AxiomAssumesInvariant` | axiom | invariant | This axiom assumes this invariant |
| `BehaviorSatisfiesProperty` | behavior (software) | property | This behavior satisfies this property |
| `BehaviorRefinesBehavior` | behavior (software) | behavior (software) | This concrete behavior refines this abstract behavior |
| `EventFollowsProtocol` | event (software) | protocol | This event follows this protocol |
| `PropertyDependsOnInvariant` | property | invariant | This property depends on this invariant |
| `RefinementRefinesAbstract` | refinement | behavior (software) | This refinement maps from this abstract behavior |
| `RefinementRefinesConcrete` | refinement | behavior (software) | This refinement maps to this concrete behavior |
| `RefinementChainsToRefinement` | refinement | refinement | This refinement chains to that refinement |
| `EventParticipatesInProcess` | event (software) | process | This event participates in this process |
| `ProcessComposesProcess` | process | process | This process composes that process |

### Adds to other kinds

| Kind | Owner | Added fields |
|------|-------|--------------|
| `behavior` | `@specforge/software` | `requires`, `ensures`, `maintains`, `sync` (blocks), `satisfies` (reference list → property), `abstract` (bool), `refines` (reference → behavior) |
| `event` | `@specforge/software` | `follows_protocol` (reference list → protocol), `participates_in` (reference list → process), `sync` (block) |
| `invariant` | `@specforge/governance` | `expression` (string) |

### Passes (5)

| Pass | After | When |
|------|-------|------|
| `analysis_available` | resolve | Every compile |
| `condition_check` | resolve | `specforge analyze` |
| `layering_verify` | condition_check | `specforge analyze` |
| `event_graph_analyze` | layering_verify | `specforge analyze` |
| `coverage_tracking` | event_graph_analyze | `specforge analyze` |

Refinement chains (E041) and process composition (E042) must have no cycle.

## @specforge/testing and the runner extensions

`@specforge/testing` gives `verify [kind] "..."` lines their meaning ([ADR 0002](adr/0002-test-runner-extensions.md)):
the core grammar parses them, and this extension says which kinds owe test obligations and which verify kinds each
accepts. It declares no kinds and no edges.

### Verify kinds it adds (7 kinds)

An addition to a kind whose extension is not loaded is skipped silently.

| Kind | Owner | Verify kinds | W004 when it has none |
|------|-------|--------------|-----------------------|
| `behavior` | `@specforge/software` | unit, contract, integration, property, performance | yes |
| `invariant` | `@specforge/governance` | unit, integration, property, performance, mutation | yes |
| `event` | `@specforge/software` | integration, unit, deadlock_free, liveness | yes |
| `type` | `@specforge/software` | unit, property | yes |
| `port` | `@specforge/software` | integration, unit | yes |
| `constraint` | `@specforge/governance` | unit, integration, property, load, contract | no |
| `failure_mode` | `@specforge/governance` | unit, integration, property | no |

W009 is a verify kind outside a kind's list.

### The coverage pass

`coverage` (`specforge analyze coverage`) scores intent (A001/A002), counts enforcement, and reads proof from the
results `specforge collect` records: failing tests (A014), obligations no passing test names (A015), tests naming
obligations the spec does not declare (A016). `--min` gates CI on proof coverage.

### Runner extensions

Each test runner has its own extension that declares one collector: the files that select it, the command
`specforge collect` runs (after you approve it, or with `--yes`), where the report lands, and a pure mapping from the
report. `@specforge/cargo-test` and `@specforge/vitest` require `@specforge/testing` and map the report to entities;
`@javascript/node-test` requires `@code/core` and maps it to test facts.

| Extension | Detected by | How a test names its entity |
|-----------|-------------|-----------------------------|
| `@specforge/cargo-test` | `Cargo.toml` | `#[specforge_test(behavior = "...", verify = "...")]`, or tests named `entity_id__obligation_slug` or inside `mod entity_id` ([guide](guides/rust-test-tracing.md)) |
| `@specforge/vitest` | `vitest.config.*`, `vitest.workspace.*` | `meta: { specforge: { behavior: "...", verify: "..." } }` ([guide](guides/vitest-test-tracing.md)) |
| `@javascript/node-test` | `package.json` | the entity cites the test: `js:<package>/<file>#<test>` in a `proof` field ([guide](guides/node-test-facts.md)) |

With cargo-test and vitest, spec files carry no test paths: the link lives in the tests. With node-test it runs the
other way: the test names nothing, and the entity cites the test's id. `specforge init` enables `@specforge/testing` with
`@specforge/software`, and a runner extension when it finds that runner in the project.

Proof today is test-centred. A general notion of evidence for non-software work (a signed record, an audit
finding) is future work.

## @specforge/rust and @specforge/typescript

Each declares one analyzer: a scanner that finds the public items (Rust) or exported symbols (TypeScript and
JavaScript) of source files, which `specforge infer` uses to guide an agent writing specs for existing code.

## @code/core

The neutral base of test facts ([ADR 0329](adr/0329-code-core-holds-test-facts-and-their-ids.md)). Its kinds are
written by a runner pack's collector, never by hand: a spec cites a fact by its id, and `specforge collect` writes the
store the session serves. It declares no collector; a runner pack peers it and mints the facts with the linked
`specforge-code-junit` library (`extensions/code/junit`, an extension library, ADR 0328 D3), so every pack names a
test the same way.

| Kind | Scheme and id | Fields |
|------|---------------|--------|
| `test` | `js:<package>/<file>#<suite > … > name>` | `package`, `file` (project-relative), `suite`, `name`, `status` (`passed`, `failed`, `skipped`; the kind's `result`), `names` (the entities the test says it proves) |
| `test_run` | `run:<test id>` | `test` (edge `run_of` to the `test`), `status`, `duration_ms` |

Neither kind is scoped: a reference names the fact whole. Its check passes judge the references:

| Code | When |
|------|------|
| `code/E101` | A reference in `js` or `run` names no fact, and a served store holds facts of that scheme; it suggests the nearest test of the same file |
| `code/W102` | A reference in `js` or `run` names no fact, and no served store holds facts of that scheme (a fresh clone, or before a collect) |
| `code/W401` | An entity that is not pending links a test whose last run failed |
| `code/I402` | A test's `names` holds an id no entity has |

An entity is pending while its owner is (its kind's `complete_when`, or its nearest scope ancestor's): its references
are not checked and its failing tests are not reported, so a test not written yet is TDD, not an error.

`@javascript/node-test` is its first runner pack: its collector `node-test`, detected by a `package.json`, runs
`node --test` with the `spec` reporter to standard output and the `junit` reporter to `.specforge/reports/node-test.xml`,
and names each test after the nearest `package.json` with a `name` ([guide](guides/node-test-facts.md)).

## @specforge/checks

The generic value checkers ([ADR 0338](adr/0338-one-foundation-extension-holds-the-generic-value-checkers.md)): each
decided by a cross-industry standard, attached to a field by its `checks` or by `specforge.json`'s `checks.fields`, and
run in the values phase ([ADR 0336](adr/0336-a-field-declares-the-checks-its-values-pass.md)). A
checker reads only the text and what the host's probes answer; the host contains every path and performs every probe.
Five bool settings, each on by default, switch the families: a family that is off is not called, sends no probe and
adds no scope to W170, and its setting `gates` the codes only it reports (E103 is shared, so none gates it).

| Family (switch) | Scope | Checkers and their arguments | Codes |
|-----------------|-------|------------------------------|-------|
| files (`families.files`) | `files` (fs: stat, read, walk) | `file`, `dir`, `glob` (`min`, `max`), `size` (`min`, `max`), `digest` (`<path>@sha256:<hex>`), `encoding` (`bom`, `line_ends`), `line_range` (`<path>:<from>-<to>`); each takes `root` (default `project`) | E016, E096, W176, E103 |
| formats (`families.formats`) | none | `email`, `semver` (`prerelease`), `semver_range`, `spdx` (`deprecated`), `iso_date` (`form`), `iso_duration`, `cron` (`seconds`), `regex`, `uuid` (`version`), `mime`, `bcp47`, `iso3166` (`form`), `iso4217` (`form`), `iana_tz` (`canonical`), `e164`, `ip` (`version`), `cidr` (`strict`), `hostname` (`wildcard`), `port` (`zero`), `hex_color` | E103 |
| digits (`families.digits`) | none | `luhn`, `mod11`, `mod97`, `gtin`, `isbn`, `iban` | E103 |
| links (`families.links`) | `links` (http: HEAD, GET, from the values) | `url` (`schemes`, default `https`; `status`), `url_anchor` (as `url`), `content_type` (`allow`), `redirect` (`hosts`, `*.` wildcards), `tls` (`days`, default 14), `dns` (a host name) | E091, E092, E093, E104, E112, W172, W173, W178, W181, W182, W183, E103 |
| git (`families.git`) | `git` | `commit` (`min_len`, default 7), `ref` (`kind`: `tag`, `branch` or `any`), `file_at` (`<revision>:<path>`), `tracked` (`root`) | E106, W180, E103 |

A links checker reads the answer after at most 5 redirects: only 404 or 410 is an error (E093, or W183 on a field
whose check says `status: "warn"`); 401 and 403 are W182, 429, 5xx and any other status W181, a host that cannot be
reached W172, and a redirect not followed W173. W172, W173, W181 and W182 are the remote's answers, so they are
`machine`. A malformed or relative URL, or one with credentials, is E091 and a scheme outside `schemes` E092, both
never fetched; a HEAD answered 405 or 501 is asked once more as a GET.

The `regex` checker accepts exactly what the kernel's `matches` accepts: one engine, the workspace's `regex`. Domain
identifiers (a national drug code, a jurisdiction's tax number) go in packs built on `specforge-checks-kit`, not here.

It declares one collector-written kind and the collector that writes it:

| Kind | Scheme and id | Fields |
|------|---------------|--------|
| `check_outcome` | `check:<entity>/<field>#<item>@<checker>` | `entity`, `field`, `item`, `checker`, `status` (`passed`, `failed`, `skipped`; the kind's `result`), `outcome` (`ok`, `status:404`, `unreachable:dns`, …), `codes`, `observed_at` (RFC 3339) |

The `checks` collector reads only the `check_outcomes` input and declares no detection, command or report: it runs
when named (`specforge collect --runner checks`) or, on a plain collect, under `checks.publish`.

## Related

- [Extension model](extension-model.md): what an extension can declare and how it runs
- [Entity model](entity-model.md): the structural core and the builtin kinds
- [Extension SDK](extension-sdk.md) and [Extension protocol](extension-protocol.md): building one
