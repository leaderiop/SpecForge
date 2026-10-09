# Prove, coverage grading, the build cache and the context schema read declarations

**Status:** accepted (2026-10-03)

ADR 0007 left four places where the host still names a domain (principle 2), listed as its known
gaps, plus one it introduced: the prove pass reads `constraint`'s `metric` and any entity's
`expression`, and the unknown-field check accepts `expression` on every kind for it; the coverage
rule grades `invariant` by its `risk`; the build cache records every entity's `status`; and the
context export's JSON schema lists `contract` and `status` while the export lifts any field an
extension declares `headline`, so a context export can fail its own schema. Each is closed by a
declaration the owning extension makes, and nothing is added for a consumer that does not exist.

## A. The prove pass reads declared proof roles

A field may declare a **proof role** (`proof_role` on `FieldDescriptor`, optional, serde default
none): `bound`, a fact the solver assumes (its conjuncts must be consistent: E046), or `claim`, a
statement that must follow from the bounds (W139 when it does not; an entailed claim is a proved
claim). `specforge_ops::prove` takes every field whose registry entry has a role, on any entity,
in the forms it already parses (an `expr { }` block, or one comparison per line of a string, prose
lines skipped and counted). It names no kind and no field. The registry refuses any other role
value, as it refuses an unknown `derived_from`.

The builtins declare:

| Field | Kind | Owner | Role |
|---|---|---|---|
| `metric` | `constraint` | governance | `bound` |
| `expression` | `property` | formal | `claim` |
| `expression` | `axiom` | formal | `bound` (an axiom is assumed, not proved) |
| `expression` | `invariant` (enhancement) | formal | `claim` |

The unknown-field carve-out for `expression` is removed: `title` is the one structural field.
`expression` on a kind no extension declares it for is W020 like any other field. Formal's
enhancement keeps this repository's corpus valid (`spec/governance/invariants.spec`); a project
that wrote `expression` on an invariant without `@specforge/formal` now gets W020, and the prove
pass ignores it, which it should, since nothing declared it a claim.

E046, W139, W098 and I098 stay core codes: consistency of declared bounds and entailment of
declared claims are structural over roles, as cycle detection is over edges. Their texts say
"declared bounds", not "constraint metric bounds", and the prove summary's
`constraints_with_metrics` becomes `entities_with_bounds`.

Axioms change meaning: an axiom's expression was a claim the bounds had to entail (with no bounds,
a tautology), which inverts what an axiom is. It is now an assumption. No corpus file declares one.

**Rejected now: formal owns a prover** (the collect pattern of ADR 0002: the host runs `z3` with
consent, a pure export encodes the script and reads the answer). It is the eventual home, but it
needs a tree-sitter-free expression crate, `FieldValue::Expression` in pass snapshots (an SDK
change), a prover descriptor, the four codes re-owned, and governance's bounds reaching formal by
peer dependency or role anyway. Roles are the part of that design which is right either way: an
extension-owned prover would read the same declarations.

**Next step, recorded:** move the solver encoding and verdicts into `@specforge/formal` behind a
prover descriptor, when a second solver or a non-arithmetic logic (temporal properties) is wanted,
or when a domain needs a different encoding. ADR 0004 D3-f (MCP analyze passes no proved claims
unless prove ran) is unchanged by this ADR.

## B. The coverage rule receives its risk grading from `@specforge/testing`

`specforge_coverage::assess` takes a **risk grading** argument, `Option<&RiskGrading>` with the
graded kind and the risk at which one of its entities without obligations is an error. With
grading, entities of that kind get the risk tallies, the enforced and orphan counts, and A002;
without it, none. The crate stops naming `invariant` and `high` (`INVARIANT_KIND`, `HIGH_RISK`
go); A002's message and suggestion name the entity's kind, so their text is unchanged for
invariants. `PROPERTY_VERIFY_KIND` stays: it is testing's own verify-kind vocabulary.

`@specforge/testing` supplies the grading from its `TESTABLE` table, which already names
software's kinds as their peer: the `invariant` row gains its risk field (`risk`) and its error
level (`high`), and the pass fills `Entity::risk` only for that row. The host passes no grading
and no risk (`specforge_project::coverage::rule_entity` stops reading `risk`): its per-entity
views use verdicts only, and no host surface reads the tallies.

The summary keys (`invariant_enforced`, `invariant_orphans`, `invariants`) stay. They are
testing's report, the `--min` gate deserializes the summary, and renaming them would break
consumers for no change in meaning while testing grades one kind.

**Rejected: a `grades_risk` flag on software's `risk` field.** Grading is a coverage policy, which
belongs to the coverage owner (ADR 0002, D2-f), not a property of the field; and the host would
have to forward the flag into the pass to reach the rule.

## C. The build cache records each kind's declared lifecycle field

An entity kind may declare a **lifecycle field** (`lifecycle_field` on `EntityKindDescriptor`,
optional, serde default none): the field holding its entities' lifecycle state, whose previous
value the build cache records for check-phase passes. The registry refuses a name the kind does
not declare. `BuildCache::of` records, for each entity whose kind declares one and whose value
is text, `{kind, status}` with the value under `status`. The format (1), the file, and the
SDK's `PassBuildCache` are unchanged: `status` is the format's name for the recorded state.

`@specforge/product` declares `lifecycle_field: "status"` on feature, milestone, deliverable,
persona, channel and release, the kinds its transition rules (W087–W091, W094) cover. Other
kinds' `status` fields (a behavior's, a decision's) are no longer recorded; nothing reads them.

A kind-level declaration, not a field flag: a kind has one lifecycle, so it names one field and
there is no multiplicity to resolve. Format 2 (recording several flagged fields per entity) was
rejected: it changes the file and the SDK for a consumer that does not exist. Deferring was
rejected because the cache already writes, and naming `status` in core is the defect.

## D. The context export's schema describes what the export writes

The Context node schema declares `id`, `kind`, `title`, `verify` and `fields` (the normative
fields, an object, which it omitted), and types every other property as a string
(`additionalProperties: {"type": "string"}`): those are the headline fields, which
`context::headline_fields` writes only as text and never under a node key. The names `contract`
and `status` leave the emitter's schema code. `GraphProtocolSchema` does not carry `headline`
(ADR 0007), so the schema cannot list the names without a protocol change, and it need not: the
shape is exact.

## What would reopen this

- A: a second solver, a temporal or otherwise non-arithmetic claim language, or a domain that
  needs its own encoding (then the next step above); a field that is both bound and claim.
- B: a second risk-graded kind (then the grading becomes a list and the summary keys a map).
- C: a rule that compares a field other than a kind's lifecycle across builds (then format 2).
- D: a consumer that needs the headline names in the schema (then `headline` joins the Graph
  Protocol's field schema, a minor-version addition).

## Amendment (architecture round 5, plan 16): `invariant_unreferenced`

The summary key `invariant_orphans` is renamed `invariant_unreferenced`: an invariant nothing references is an
unreferenced entity (CONTEXT.md). The reason this ADR gave for keeping the keys, that renaming them would break
consumers, does not hold before a release (no backward compatibility); `invariant_enforced` and `invariants` keep
their names, which say what they are.
