# failure_mode

> **Declared by `@specforge/governance`**, one of the builtin extensions. `failure_mode` is one example of an entity kind, not part of
> SpecForge's core: the core grammar knows only `spec`, `ref`, `use`, `define` and `defaults`, and every other kind,
> with its fields, edges and rules, comes from an extension ([extension model](../extension-model.md)). Enable this
> one with `specforge add @specforge/governance`. The fields, edges and rules below are read from its declaration.

## Purpose

A `failure_mode` is one entry of an FMEA (Failure Mode and Effects Analysis): what can go wrong, why, what it
does, how bad, how likely, how detectable, and what is done about it. It answers: **"What can go wrong and how
bad is it?"**

A failure mode closes the loop between a guarantee and risk management: an invariant says "this must always be
true"; a failure mode says "here is how it could become false, and here is the countermeasure". The same shape
serves software (lost writes) and regulated processes (a mislabelled batch).

## Syntax

```spec
failure_mode write_loss "Write Acknowledged but Lost" {
  invariant  data_persistence
  severity   high
  occurrence unlikely
  detection  unlikely

  cause      "Crash between the acknowledgement and fsync"
  effect     "Silent data loss: the user believes the write succeeded"
  mitigation "Write-ahead log with fsync before acknowledging"

  post_severity   high
  post_occurrence rare
  post_detection  likely

  affected_behaviors [create_user]
}
```

Ratings are words, not numbers: `severity` is one of critical, high, medium, low; `occurrence` one of certain,
likely, occasional, unlikely, rare; `detection` one of certain, likely, moderate, unlikely, undetectable. An
`rpn` you write is kept as written: the compiler does not compute it or check it against the ratings.

## Fields

Every `failure_mode` has an id (the word after `failure_mode`) and a title (the quoted string after the id, required: E076). The fields below are the ones its declaration lists. A field no loaded extension declares is reported (W020).

### Required

| Field | Type | Description |
|-------|------|-------------|
| `severity` | enum: critical, high, medium, low | Impact severity rating before mitigation. |
| `cause` | string | Root cause or trigger of the failure. |
| `effect` | string | Consequence or impact when the failure occurs. |

### Optional

| Field | Type | Description |
|-------|------|-------------|
| `occurrence` | enum: certain, likely, occasional, unlikely, rare | Likelihood of this failure occurring before mitigation. |
| `detection` | enum: certain, likely, moderate, unlikely, undetectable | Likelihood of detecting this failure before it causes harm. |
| `mitigation` | string | Strategy or action to reduce risk of the failure. |
| `post_severity` | enum: critical, high, medium, low | Impact severity rating after mitigation is applied. |
| `post_occurrence` | enum: certain, likely, occasional, unlikely, rare | Likelihood of occurrence after mitigation is applied. |
| `post_detection` | enum: certain, likely, moderate, unlikely, undetectable | Likelihood of detection after mitigation is applied. |
| `invariant` | reference → `invariant` | The invariant that this failure mode threatens. |
| `refs` | string list | External references (URLs, documents, issue links). |
| `description` | string | Free-form description of the failure mode. |
| `risk` | string | Overall risk assessment combining severity, occurrence, and detection. |
| `rpn` | integer | Risk Priority Number (severity x occurrence x detection). |
| `post_mitigation` | block | Residual risk after mitigation: severity, occurrence, detection and rpn. |
| `threatens_features` | reference list → `feature` | Product features threatened by this failure mode (requires @specforge/product). |
| `affected_behaviors` | reference list → `behavior` | Behaviors affected by this failure mode as failure vectors. |

## Verify kinds

`@specforge/testing` makes `failure_mode` testable: write `verify <kind> "..."` lines in its body. The kinds it accepts are `unit`, `integration`, `property`. A bare `verify "..."` has no kind. A kind outside this list is W009.

## Relationships

A relation is written once, on the field that owns it; the other side is derived (ADR 0150). For an incoming edge the field is on the other entity.

| Direction | Entity | Edge | Written as | Meaning |
|-----------|--------|------|------------|---------|
| outgoing | [behavior](behavior.md) | `FailureModeAffectsBehavior` | `affected_behaviors` on `failure_mode` | Failure mode identifies a behavior as a failure vector |
| outgoing | [invariant](invariant.md) | `FailureModeTargetsInvariant` | `invariant` on `failure_mode` | Failure mode targets an invariant |
| outgoing | [feature](feature.md) | `FailureModeThreatensFeature` | `threatens_features` on `failure_mode` | Failure mode threatens a product feature |

## Validation rules

Core checks every entity whatever its kind: references must resolve (E003), ids are unique across the project (E002), required fields are present (E006), each value has its field's type, an enum value being one the field lists (E061), and every block has a title (E076). The rules below are the ones extensions declare for this kind; [diagnostics.md](../diagnostics.md) has every code.

| Code | Level | Declared by | Rule |
|------|-------|-------------|------|
| W009 | warning | `@specforge/testing` | Disallowed verify kind |


## Design guidance

### When to write one

For every invariant whose `risk` is `high` or `critical`, for anything whose failure affects data integrity,
patient or user safety, and for operational risks learned from incidents.

### Cause, effect, mitigation

The three text fields tell the story: the **cause** (what triggers it), the **effect** (what people experience),
the **mitigation** (the countermeasure).

### Reassess after mitigation

Write the `post_` ratings. A good mitigation lowers occurrence or makes the failure easier to detect; severity
rarely changes, because the harm is inherent.

## Examples

### Race condition

```spec
failure_mode email_race "Email Uniqueness Bypass via Race Condition" {
  invariant  email_uniqueness
  severity   medium
  occurrence occasional
  detection  unlikely

  cause      "Two concurrent requests create users with the same email before the uniqueness check"
  effect     "Two active users share an email: login ambiguity, possible account takeover"
  mitigation "Database-level UNIQUE constraint and serializable isolation for user creation"

  post_occurrence rare
  post_detection  certain
}
```

### A regulated process

```spec
failure_mode label_mixup "Wrong Label Applied to a Batch" {
  invariant  label_matches_batch
  severity   critical
  occurrence rare
  detection  moderate

  cause      "Two batches of similar packaging on the line at the same time"
  effect     "Patients receive a product whose label does not match its contents"
  mitigation "Line clearance check and barcode verification before labelling"

  post_detection certain
}
```
