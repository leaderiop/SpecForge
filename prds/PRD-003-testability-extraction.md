# PRD-003: Testability Extraction (@specforge/software-testing)

**Status:** Draft
**Author:** Mohammad AL Mechkor
**Date:** 2026-04-12
**Depends on:** PRD-001 (Extension Protocol), PRD-002 (Entity Audit)

---

## Problem Statement

Testing concepts are currently spread across four SpecForge extensions as hardcoded fields and validation rules. Every entity kind in `@specforge/software` declares `testable: true`, every entity kind across all manifests carries `supportsVerify` and `allowedVerifyKinds` metadata, and behavior has dedicated `tests` and `gherkin` fields, a `TestedBy` edge type, and W004/W009 validation rules.

This creates three problems:

1. **No opt-out.** Projects that don't use BDD testing (internal tools, data pipelines, infrastructure specs) still see `gherkin` fields in their schema output and `W004` warnings in their validation results. There is no way to turn off testing concerns without modifying the extension manifests.

2. **Principle 7 violation.** SpecForge's architecture says "extensions over built-ins, always." Testing is a cross-cutting concern that enhances domain entities — it is not intrinsic to the definition of a behavior or an invariant. A behavior's identity is its contract and its relationships, not whether it has test files attached.

3. **Scattered ownership.** Testing fields appear in `@specforge/product` (feature has `supportsVerify`), `@specforge/software` (behavior has `tests`, `gherkin`, `TestedBy`), `@specforge/governance` (constraint has `supportsVerify`), and `@specforge/formal` (property has `supportsVerify`). No single extension owns the testing concern. Changes to testing semantics require coordinated edits across all four manifests.

PRD-002 already stripped the testability metadata (`testable`, `supportsVerify`, `allowedVerifyKinds`, `verifyKinds`) from all manifests and removed the `tests`, `gherkin`, `TestedBy`, W004, and W009 from `@specforge/software`. This PRD builds the replacement: a dedicated extension that re-introduces testing capabilities as an opt-in enhancement.

## Solution

Create `@specforge/software-testing`, an **enhancement-only extension** that declares no entity kinds of its own. It adds the `gherkin` field to 13 entity kinds across 4 extensions using the entity enhancement mechanism, owns the `TestedBy` edge type, provides the W004 validation rule, and includes a Cucumber/Gherkin test result collector.

Projects that want BDD testing add `@specforge/software-testing` to their extensions list. Projects that don't want it simply omit it. The testing concern is fully modular.

### Design Decisions

**Gherkin only.** The `verify` block DSL construct is permanently removed. Gherkin `.feature` files are the sole testing mechanism. This simplifies the grammar (no `verify` production rule), the parser (no `VerifyStatement`), and the extension model (no `verify_kinds` concept).

**Enhancement-only pattern.** `@specforge/software-testing` declares zero entity kinds. It contributes only through entity enhancements, edge types, validation rules, and collectors. This is the first extension to use the enhancement-only pattern, establishing it as a proven architecture for cross-cutting concerns.

**13 entity enhancements.** The `gherkin` field (`string_list`, `file_reference: true`) is added to every entity kind where testing makes sense: all 5 software kinds, 3 product kinds (feature, deliverable, milestone), and 5 kinds from governance/formal where verification is relevant.

**Dual peer dependency.** `@specforge/software-testing` depends on both `@specforge/software` (^1.0) and `@specforge/product` (^1.0). It enhances kinds from both extensions directly. Enhancements targeting `@specforge/governance` and `@specforge/formal` kinds use soft references — they apply if those extensions are installed, and produce I004 diagnostics if they're not.

## User Stories

1. As a project author who uses BDD testing, I want to run `specforge add @specforge/software-testing`, so that all my entity kinds gain the `gherkin` field for linking Gherkin feature files.

2. As a project author who doesn't use BDD testing, I want to omit `@specforge/software-testing` from my `specforge.json` extensions list, so that I never see `gherkin` fields or test-coverage warnings.

3. As a spec author, I want to write `gherkin [tests/auth.feature]` inside a behavior block, so that SpecForge validates the file exists and tracks the behavior-to-test linkage.

4. As a spec author, I want to write `gherkin [tests/checkout.feature]` inside a feature block, so that product-level acceptance scenarios are linked to the feature they verify.

5. As a spec author, I want the `gherkin` field to work on invariant, event, type, and port entities, so that I can link verification scenarios to any testable concept.

6. As a spec author, I want the `gherkin` field to work on deliverable and milestone entities, so that I can link acceptance scenarios to planning artifacts.

7. As a spec author, I want the `gherkin` field on governance constraint entities, so that I can link performance or compliance test scenarios to non-functional requirements.

8. As a spec author, I want the `gherkin` field on formal property, protocol, process, and refinement entities, so that I can link formal verification scenarios to mathematical specifications.

9. As a project author, I want W004 to fire when a behavior has the `gherkin` field but lists no files, so that I'm warned about empty test declarations.

10. As a project author, I want W004 to fire ONLY when `@specforge/software-testing` is installed, so that projects without the testing extension never see test-coverage warnings.

11. As a CI pipeline operator, I want to run `specforge collect cucumber` to ingest Cucumber test results, so that test-to-entity traceability is populated from BDD test runs.

12. As a CI pipeline operator, I want the Cucumber collector to auto-detect `**/cucumber-report.json` and `**/cucumber-report.xml` files, so that I don't need to specify paths manually.

13. As a SpecForge contributor, I want `@specforge/software-testing` to be the canonical example of the enhancement-only extension pattern, so that future cross-cutting extensions follow the same architecture.

14. As an agent consuming the graph, I want `TestedBy` edges to appear only when `@specforge/software-testing` is installed, so that my graph queries don't include test-traceability edges in projects that don't test.

15. As a spec author, I want the `gherkin` field's `file_reference: true` to trigger E005 validation when a referenced `.feature` file doesn't exist on disk, so that broken file references are caught early.

16. As a project author upgrading from the pre-extraction era, I want my existing `gherkin` fields on behaviors to continue working after installing `@specforge/software-testing`, so that the migration is seamless.

17. As a formal methods user, I want the `gherkin` field on property entities to be optional and produce no warnings if empty (unless a stricter rule is declared), so that formal verification doesn't require BDD scenarios.

18. As a schema consumer, I want the Graph Protocol JSON output to include `gherkin` fields on enhanced entity kinds only when `@specforge/software-testing` is installed, so that the schema accurately reflects the installed extensions.

## Implementation Decisions

### Extension Architecture

`@specforge/software-testing` is built using the Extension SDK (PRD-001). It compiles to a single `.wasm` file. Its `__handshake` declares:
- `contribution_flags: { entities: false, validators: true, collectors: true }`
- `peer_dependencies: [@specforge/software ^1.0, @specforge/product ^1.0]`

Its `__describe("enhancements")` returns 13 field enhancements. Its `__describe("edges")` returns the `TestedBy` edge type. Its `__describe("validation_rules")` returns the W004 rule. Its `__describe("collectors")` returns the Cucumber collector.

### 13 Entity Enhancements

Each enhancement adds the same field:
```
gherkin: string_list, file_reference=true, description="BDD scenario files"
```

**Direct enhancements (peer dependencies guarantee target kinds exist):**
- behavior, invariant, event, type, port (from @specforge/software)
- feature, deliverable, milestone (from @specforge/product)

**Soft-reference enhancements (apply if target extension installed, I004 otherwise):**
- constraint (from @specforge/governance)
- property, protocol, process, refinement (from @specforge/formal)

### TestedBy Edge Type

A standalone edge type (no source/target kind constraint) with `dotted` style and gray color. Generated automatically by the host when a `gherkin` field with `file_reference: true` is populated. The edge connects the entity to a virtual file-reference node.

### W004 Validation Rule

```
code: W004, severity: warning
check: missing_field_when_flag_set
targetKind: behavior (and other enhanced kinds)
field: gherkin
message: "{kind} '{id}' has gherkin field but no files referenced"
```

Only fires when `@specforge/software-testing` is installed (because the rule is declared by this extension). Projects without the extension never see this warning.

### Cucumber/Gherkin Collector

```
name: cucumber
formats: [junit-xml, json]
export: collect__cucumber
auto_detect:
  files: [**/cucumber-report.json, **/cucumber-report.xml]
```

Parses Cucumber test results and maps them to entity IDs for the `specforge-report.json` traceability chain.

### Core Rust Cleanup (Deferred)

Approximately 10 Rust source files still reference `testable`, `supports_verify`, or `verify` constructs in types, grammar rules, parser nodes, registry logic, validators, emitters, and MCP tools. These references are functionally inert (the manifest values are all false/empty), but the dead code should be removed. This cleanup is tracked but not required for `@specforge/software-testing` to function.

## Testing Decisions

### What Makes a Good Test

Tests verify that installing `@specforge/software-testing` adds the `gherkin` field to the correct entity kinds, fires W004 when appropriate, and does NOT affect projects that don't install it. Tests should use the public registry and compilation APIs, not inspect internal enhancement data structures.

### Modules to Test

**Enhancement application** -- Load `@specforge/software-testing` alongside `@specforge/software` and `@specforge/product`. Verify that `FieldRegistry.contains("behavior", "gherkin")` returns true. Verify that `FieldRegistry.contains("feature", "gherkin")` returns true. Verify all 8 direct enhancements are applied. Prior art: `specforge-registry/tests/software_manifest.rs` tests for entity enhancement fields.

**Soft-reference enhancements** -- Load `@specforge/software-testing` WITHOUT `@specforge/governance` or `@specforge/formal`. Verify I004 diagnostics are emitted for the 5 soft-reference enhancements. Then load with governance and formal installed. Verify all 13 enhancements apply. Prior art: existing I004 diagnostic tests in `specforge-registry`.

**W004 validation** -- Create a spec file with a behavior that has an empty `gherkin` field. Compile with `@specforge/software-testing` installed. Verify W004 fires. Compile without it installed. Verify W004 does not fire. Prior art: existing W004 test patterns in `specforge-registry/tests/zero_entity_validation.rs`.

**Opt-out verification** -- Compile a project without `@specforge/software-testing` in the extensions list. Verify that no `gherkin` field appears in the schema, no `TestedBy` edges exist, and no W004 diagnostics fire.

**Collector** -- Feed a Cucumber JSON report to the collector. Verify it produces `specforge-report.json` entries mapping test results to entity IDs. Prior art: existing collector tests in `specforge-wasm/tests/collector_integration.rs`.

## Out of Scope

- **verify block removal from grammar/parser.** The Tree-sitter grammar still has a `verify_statement` rule and the parser still has `VerifyStatement` types. Removing them is a grammar change that requires regenerating the Tree-sitter parser and is tracked separately.

- **Core Rust cleanup of testable/verify references.** ~10 files still reference these concepts. Functionally inert but messy.

- **Test runners or test execution.** SpecForge traces tests and consumes results. It never executes them. The collector ingests external test results only.

- **Non-Gherkin testing.** The `tests` field (raw file paths without BDD semantics) was removed. If a future extension wants to support non-Gherkin test traceability, it would be a separate extension.

- **Verify kinds.** The concept of `allowedVerifyKinds` (unit, integration, property, etc.) is removed entirely. There is no replacement. Gherkin scenarios don't categorize by test type.

## Further Notes

### Migration Path

For projects currently using `gherkin` fields on behaviors:

1. Add `@specforge/software-testing` to `specforge.json` extensions list
2. Run `specforge check` -- all existing gherkin references continue to work
3. Optionally add gherkin fields to other entity kinds (features, invariants, etc.)

No spec file changes required. The migration is purely additive.

### Precedent for Enhancement-Only Extensions

`@specforge/software-testing` establishes the pattern for future cross-cutting concerns:

- **Observability** -- could add `metrics`, `traces`, `logs` fields to behaviors and ports
- **Security** -- could add `threat_model`, `attack_surface` fields to behaviors and types
- **Compliance** -- could add `regulation`, `audit_trail` fields to decisions and constraints

Each would be a separate enhancement-only extension following the same architecture.

### Relationship to PRD-001

`@specforge/software-testing` is Phase 6 of PRD-001 (Extension Protocol). It is the first extension built entirely with the SDK, using `#[enhance]` macros to add fields to foreign entity kinds. Its successful delivery validates the enhancement-only pattern end-to-end.
