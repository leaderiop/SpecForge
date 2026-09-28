# SpecForge Diagnostic Codes

Generated from `crates/specforge-cli/src/explain.rs` (`specforge explain <CODE>`).
Regenerate after editing explanations; the `explain_docs_sync` test fails
when this file and the code catalog drift apart.

## E001

```
E001: Parse error

The input could not be parsed as valid SpecForge syntax.
Check for missing braces, unclosed strings, or invalid field syntax.
```

## E002

```
E002: Duplicate entity

Two entities with the same kind and ID were found.
Entity IDs must be unique within their kind across all files.
```

## E003

```
E003: Unresolved reference

A reference list contains an ID that does not match any entity in the graph.
Check for typos or missing entity definitions.
```

## E004

```
E004: Invalid field value

A field has a value that does not match its expected type.
For example, a reference list field contains a plain string.
```

## E005

```
E005: Cycle detected

A circular dependency was found in the graph.
Entity A depends on B, which depends on A (directly or transitively).
```

## E006

```
E006: Missing required field

An entity is missing a field that its kind requires.
Check the extension manifest for required fields.
```

## E007

```
E007: Invalid status value

A status field contains a value not in the allowed enum.
Check the extension manifest for valid status values.
```

## E008

```
E008: Invalid priority value

A priority field contains a value not in the allowed enum.
Valid priorities are typically: critical, high, medium, low.
```

## E009

```
E009: Invalid artifact type

A deliverable's artifact_type is not recognized.
Valid types are defined by the @specforge/product extension.
```

## E010

```
E010: Invalid event direction

An event's direction must be 'inbound', 'outbound', or 'internal'.
```

## E016

```
E016: Referenced file does not exist

A file reference (e.g. `sources`, `test_files`) points to a path that does not exist in the project. Check the path relative to the spec root.
```

## E019

```
E019: Unsupported format version

The `.spec` file declares a format version this compiler does not understand. Run `specforge migrate` to upgrade the project.
```

## E022

```
E022: Mistyped reference

A reference points to an entity of the wrong kind.
For example, a 'features' field referencing a behavior instead of a feature.
```

## E028

```
E028: Incompatible host API version

An extension requires a host API version that this build of SpecForge 
does not support. Upgrade SpecForge or use a compatible extension version.
```

## E042

```
E042: Process composition cycle

A process composes (transitively) with itself via `sub_processes`. Break the cycle — composition must form a DAG. Reported by `specforge analyze` with @specforge/formal installed.
```

## E045

```
E045: Invalid test report

A collector report could not be read or is not the expected JSON shape (`entity_results` entries with `entity_id` and `test_results`). Re-run the test runner or `specforge collect` with a valid --report.
```

## E046

```
E046: Metric bounds are contradictory

Two or more declared metric bounds cannot hold simultaneously - no value satisfies them all, so at least one bound is wrong. Bounds are checked corpus-wide with an SMT solver: the error names the minimal set of bounds (across files) that contradict each other, with the exact metric line for each. Unit suffixes are normalized (100ms vs 1s compare correctly). Relax or correct one of the bounds. Reported by `specforge analyze --prove`.
```

## E047

```
E047: Formal claim is not entailed by declared bounds

An entity declares a formal `expression` claim, but the prove pass found a counterexample: concrete values that satisfy every declared constraint bound while violating the claim. The claim is not WRONG - the declared bounds simply do not guarantee it yet. Strengthen the constraint bounds or weaken the claim. The counterexample values are rendered in the claim's declared unit. Reported by `specforge analyze --prove`.
```

## I004

```
I004: Cross-extension reference

A reference targets an entity from another extension that is not installed.
The reference is kept as-is but cannot be validated.
```

## I005

```
I005: Entity from unknown extension

An entity uses a kind not registered by any installed extension.
The entity is still parsed but not validated.
```

## I010

```
I010: No spec files found

The spec root directory contains no .spec files.
Create spec files or check the spec_root setting in specforge.json.
```

## I076

```
I076: Deliverable chain gap

A deliverable's dependency chain has a gap (missing intermediate deliverable).
```

## I077

```
I077: Feature multi-milestone

A feature appears in multiple milestones.
This is informational — it may indicate scope overlap.
```

## I078

```
I078: Priority escalation gap

A high-priority feature depends on a lower-priority feature.
```

## I079

```
I079: Milestone implicit ordering

Milestones have an implicit ordering that may not match intent.
```

## I085

```
I085: Inconsistent owner strings

Different entities use slightly different owner strings for the same person.
```

## I091

```
I091: Duplicate release version

Two releases share the same version string.
```

## I097

```
I097: Build cache absent

Status transition validation requires specforge-cache.json.
Run `specforge check` to generate the cache file.
```

## I098

```
I098: Solver could not decide metric bounds

The SMT solver returned unknown for a metric bound group, so the constraints could not be verified or refuted. Simplify the bounds or rerun with a newer solver. Reported by `specforge analyze --prove`.
```

## I999

```
I999: Diagnostic output truncated

More than 100 diagnostics were generated. Only the first 100 are shown.
Fix the reported issues and rerun to see remaining diagnostics.
```

## W001

```
W001: Missing verify statement

A testable entity has no verify statements.
Verify statements declare expected behavior for traceability.
```

## W002

```
W002: Unused entity

An entity is not referenced by any other entity.
It may be orphaned or missing connections.
```

## W003

```
W003: Missing contract field

A behavior entity has no contract field.
Contracts define the expected input/output behavior.
```

## W010

```
W010: Unknown annotation on a type field

A `type` field carries an annotation the compiler does not recognize.
Check the spelling against the supported annotations, or remove the
annotation if it is obsolete.
```

## W035

```
W035: Undischarged coverage items

One or more coverage items (invariants and testable entities) have no `tests [...]` linkage, so nothing connects their intent to an executable test. Add tests fields or drop the items. Reported by the @specforge/formal coverage_tracking compiler pass via specforge analyze.
```

## W041

```
W041: Orphan feature

A feature is not referenced by any journey, milestone, or module.
It may not be reachable in the product graph.
```

## W042

```
W042: Orphan journey

A journey has no deliverables referencing it.
```

## W043

```
W043: Orphan deliverable

A deliverable is not included in any release.
```

## W044

```
W044: Orphan milestone

A milestone is not referenced by any release.
```

## W045

```
W045: Orphan module

A module is not referenced by any deliverable or milestone.
```

## W046

```
W046: Orphan term

A term is not referenced by any other entity via see_also.
```

## W049

```
W049: Empty reference list

A reference list field is present but empty.
Either add references or remove the field.
```

## W057

```
W057: Missing title

An entity has no title string after its ID.
Titles improve readability and appear in exports.
```

## W060

```
W060: Cross-kind ID collision

The same ID is used by entities of different kinds.
This can cause ambiguity in reference resolution.
```

## W061

```
W061: Reference cycle detected

A circular dependency was found in entity references.
Entity A references B, which references A (directly or transitively).
This is a warning — cycles may indicate a design issue.
```

## W062

```
W062: Malformed semver version

A version string in an extension manifest is not valid semver.
Use versions like 1.0.0 and ranges like ^1.0.0, ~1.2.0, or >=1.0.0.
```

## W063

```
W063: Circular peer dependency

Two or more extensions have circular peer dependency declarations.
Extension A depends on B, and B depends on A. Break the cycle by removing 
one dependency or restructuring the extension boundaries.
```

## W075

```
W075: Mixed list types

A list field contains both string literals and identifier references.
Use a consistent type: all strings or all identifiers.
```

## W087

```
W087: Invalid feature status transition

A feature's status changed to a state not reachable from its previous state.
Valid transitions are defined by the status state machine.
```

## W088

```
W088: Invalid milestone status transition

A milestone's status transition is not valid.
```

## W089

```
W089: Invalid deliverable status transition

A deliverable's status transition is not valid.
```

## W090

```
W090: Invalid persona status transition

A persona's status transition is not valid.
```

## W091

```
W091: Invalid channel status transition

A channel's status transition is not valid.
```

## W092

```
W092: Release dependency cycle

Releases form a circular dependency chain.
```

## W093

```
W093: Release version not semver

A release's version field is not valid semver.
Use the format MAJOR.MINOR.PATCH (e.g., 1.2.3).
```

## W094

```
W094: Invalid release status transition

A release's status transition is not valid.
```

## W095

```
W095: Invalid effort value

An effort field contains an unrecognized size.
Valid values: xs, s, m, l, xl.
```

## W096

```
W096: Behavior requires without ensuring

A behavior declares a requires clause but no ensures clause: it obligates callers without providing a guarantee (Design by Contract obligations/benefits symmetry). Add an ensures clause or drop the requirement. Reported by the @specforge/formal condition_check compiler pass via specforge analyze.
```
