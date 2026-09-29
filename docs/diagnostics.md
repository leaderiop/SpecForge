# SpecForge Diagnostic Codes

<!-- Generated file: do not edit by hand. -->

This page is generated from the catalog in `crates/specforge-cli/src/explain.rs`,
the single registry of diagnostic codes; `specforge explain <CODE>` prints the
same text. Every code emitted by the compiler, the CLI, or a first-party
extension has exactly one entry, and each entry names its owner: `core` for the
compiler and CLI, or the `@specforge/<name>` extension that emits it. A test
fails when an emitted code is missing here, is attributed to the wrong owner,
or is listed but never emitted.

Codes follow the pattern `E###` (error), `W###` (warning) and `I###` (info).
The ranges `E900`-`E998`, `W900`-`W998` and `I900`-`I998` are reserved for
third-party extensions and never appear in this catalog; `I999` is a core code.

Regenerate this page after editing the catalog:

```sh
SPECFORGE_BLESS=1 cargo test -p specforge-cli explain_docs_sync
```

## E001

```
E001: Parse error

A `.spec` file could not be parsed — invalid syntax such as a missing brace,
unclosed string, or malformed field at the reported location; the same code also
covers an internal reparse failure in the language server. Fix the syntax at the
reported span and re-save.

Owner: core
```

## E002

```
E002: Duplicate entity ID

Two entities of the same kind declare the same ID; the message names the file
where the ID was first declared. Rename one of the entities so each ID is unique
within its kind.

Owner: core
```

## E003

```
E003: Unresolved reference

A reference field names an entity ID that doesn't resolve to any declared
entity. Fix the typo or add the missing entity; a `did you mean` suggestion is
included when a close match exists.

Owner: core
```

## E004

```
E004: Unknown type in port method

A `port` entity's method field references a `type` ID that isn't declared
anywhere in the spec. Declare the missing `type` entity or fix the reference to
point at an existing one.

Owner: @specforge/software
```

## E006

```
E006: Missing required field

An entity is missing a field marked `required: true` for its kind in the field
registry. Add the missing field to the entity.

Owner: core
```

## E007

```
E007: Module dependency cycle

The `depends_on` edges between `module` entities form a cycle. Break the cycle
by removing or inverting one of the dependencies.

Owner: @specforge/product
```

## E010

```
E010: Invalid milestone behavior range

A `milestone` entity's `behaviors` range field is malformed (the reported reason
explains what's wrong, e.g. bad syntax or start after end). Correct the range to
a valid form.

Owner: @specforge/software
```

## E013

```
E013: Reserved entity ID

An entity's ID is a reserved word — a structural DSL keyword (`spec`, `ref`,
`use`, `define`) or an entity kind keyword contributed by an installed
extension. Rename the entity, e.g. by appending a suffix like `_rule` or
`_spec`.

Owner: core
```

## E014

```
E014: Entity ID length violation

An entity ID is outside the 2-60 character identifier contract. Pick a
descriptive identifier within that length.

Owner: core
```

## E015

```
E015: Milestone dependency cycle

The `depends_on` edges between `milestone` entities form a cycle. Break the
cycle by removing or inverting one of the dependencies.

Owner: @specforge/product
```

## E016

```
E016: Referenced file does not exist

A file-reference field on an entity points at a path that doesn't exist under
the spec root. Fix the path, or create the missing file; a similarly-named file
is suggested when one is found.

Owner: core
```

## E017

```
E017: Entity enhancement conflict

Two installed extensions both declare an entity-enhancement field with the same
name on the same target entity kind, and no explicit override resolves it.
Rename one extension's field, or add an override for that kind/field in
`specforge.json`.

Owner: core
```

## E018

```
E018: Grammar contribution conflict

Two extensions both contribute a tree-sitter grammar for the same entity kind.
Only one extension may own an entity kind's grammar — uninstall one of the
conflicting extensions or set a grammar conflict policy in the compiler config.

Owner: core
```

## E019

```
E019: Unsupported format version

A `.spec` file's `// specforge-format: MAJOR.MINOR` header declares a version
newer than this build supports, or the header itself doesn't parse. Lower the
declared version or upgrade SpecForge.

Owner: core
```

## E020

```
E020: Missing Wasm export

An extension's manifest declares a contribution (validator, renderer, parser,
collector, grammar, or surface command/tool) whose required Wasm export function
isn't present in the compiled module. Add the matching `#[export_name = "..."]`
export to the extension's Wasm binary.

Owner: core
```

## E022

```
E022: Reference targets wrong kind

A reference field is declared to only accept entities of a specific kind, but
the target ID resolves to an entity of a different kind. Point the field at an
entity of the expected kind.

Owner: core
```

## E023

```
E023: Entity kind conflicts with keyword

An extension declares an entity kind keyword that collides with a structural DSL
keyword (`spec`, `ref`, `use`, `define`). Choose a different keyword for the
entity kind.

Owner: core
```

## E024

```
E024: Unknown entity kind

An entity block uses a kind keyword that neither the core grammar nor any
installed extension declares. Install the extension that provides the keyword,
or fix a typo in the kind name.

Owner: core
```

## E025

```
E025: Import resolution failed

A `use` import's target either can't be read from disk or doesn't resolve to any
known `.spec` file. Fix the import path; a `did you mean` suggestion is included
when a close match exists.

Owner: core
```

## E026

```
E026: Entity kind registration conflict

Two extensions, or a project `define` block and an extension, register the same
entity kind keyword; the first registration wins and the later one is rejected.
Rename the conflicting kind keyword.

Owner: core
```

## E027

```
E027: Unsatisfiable peer dependency

An extension's required peer dependency can't be satisfied: it isn't installed,
the installed version doesn't match the required range, peer dependencies form a
cycle, an uninstall would remove an extension others still require, or an
upgrade would break a peer's requirement. Install or upgrade the named peer, or
use `--force` where the command supports it.

Owner: core
```

## E028

```
E028: Extension load or execution failure

An extension's Wasm module failed somewhere in its lifecycle — the binary is
missing or unreadable, failed to load, trapped while running
`initialize`/`validate`/a collector/body parser/surface command/MCP tool or
resource, returned output that isn't valid JSON, a grammar cache path couldn't
be written, or the extension's declared host API version isn't supported. Check
the extension's logs or report the trap to its author, and confirm the extension
is installed and up to date.

Owner: core
```

## E029

```
E029: Duplicate body parser registration

Two extensions both register a body parser for the same entity kind. At most one
body parser is allowed per entity kind — uninstall or reconfigure one of the
conflicting extensions.

Owner: core
```

## E030

```
E030: Invalid extension manifest

An extension's manifest is unreadable, isn't valid JSON, or fails schema
validation — a wrong `manifestVersion`, a missing `name`/`version`/`wasmPath`,
an empty grammar/body-parser/analyzer contribution field, or a sandbox policy
that allowlists a code file extension for output. Fix the manifest according to
the reported detail.

Owner: core
```

## E031

```
E031: Refinement drops ensures condition

A behavior that `refines` an `abstract` behavior drops one or more of the
abstract behavior's `ensures` conditions. A refinement may only strengthen its
abstraction's postconditions, never weaken them — restore or strengthen the
missing `ensures` condition(s) in the concrete behavior.

Owner: @specforge/formal
```

## E032

```
E032: Extension install or uninstall failed

An install or uninstall step failed: the downloaded `.wasm` binary's SHA-256
hash didn't match the expected value (possible tampering or a bad download), or
a filesystem step — creating the temp directory, writing the binary,
finalizing the install, or removing the extension directory on uninstall —
failed. Re-download the extension or check filesystem permissions.

Owner: core
```

## E033

```
E033: Lock file error

`specforge.lock` couldn't be serialized, written, read, or parsed, or the hash
it records for an installed extension no longer matches the binary on disk.
Delete the lock file and reinstall extensions, or reinstall the specific
extension whose binary changed.

Owner: core
```

## E035

```
E035: Reserved or invalid entity kind name

An extension-declared entity kind name is a reserved structural keyword, doesn't
match the identifier pattern `[a-z][a-z0-9_]{1,59}`, or is already reserved by
another installed extension. Choose a different, valid entity kind name.

Owner: core
```

## E037

```
E037: Grammar ABI version mismatch

A tree-sitter grammar `.wasm` binary was built against an ABI version that
doesn't match the version this SpecForge build supports. Rebuild the grammar
targeting the supported tree-sitter ABI version.

Owner: core
```

## E038

```
E038: Grammar binary too large

A tree-sitter grammar `.wasm` binary exceeds the configured maximum size. Reduce
the grammar's complexity or raise `max_size_bytes` in the compiler config.

Owner: core
```

## E039

```
E039: Duplicate surface contribution

Two extensions register the same CLI surface command ID, the same MCP tool name,
or the same MCP resource name. Rename one extension's contribution so the
identifier is unique across installed extensions.

Owner: core
```

## E040

```
E040: Missing extension project file

`specforge extension build` or `validate` was run against a directory that's
missing its `Cargo.toml` or `manifest.json`. Run the command from a scaffolded
extension project, or create the missing file.

Owner: core
```

## E041

```
E041: Refinement chain cycle

The `refines` layering graph between behaviors contains a cycle. Break the cycle
by removing or redirecting one of the `refines` edges.

Owner: @specforge/formal
```

## E042

```
E042: Process composition cycle

A `process` entity composes, transitively, with itself through its composition
edges. Remove or redirect one of the composition steps to break the cycle.

Owner: @specforge/formal
```

## E045

```
E045: Invalid test report

`specforge collect` couldn't proceed with a test report: no report files were
found (pass `--report` or place `*.json` files under the default report
directory), a report file couldn't be read, or its contents aren't valid JSON.
Check the report path and its contents.

Owner: core
```

## E046

```
E046: Metric bounds are contradictory

`specforge prove`'s SMT solver found the declared `constraint` metric bounds
mutually unsatisfiable; the cited bounds form the conflicting core. Relax or
correct one of the listed bounds.

Owner: core
```

## E047

```
E047: Formal claim not entailed by declared bounds

`specforge prove` found that a `claim` isn't guaranteed by the declared metric
bounds — a counterexample satisfying the bounds while violating the claim was
found. Strengthen the declared constraint bounds or weaken the claim.

Owner: core
```

## E051

```
E051: Invalid event trigger

An `event` entity's `trigger` field must reference a `behavior`, but it points
at something else (or nothing resolvable). Point `trigger` at an existing
`behavior` entity.

Owner: @specforge/software
```

## E052

```
E052: Deliverable dependency cycle

The `depends_on` edges between `deliverable` entities form a cycle. Break the
cycle by removing or inverting one of the dependencies.

Owner: @specforge/product
```

## E053

```
E053: Host call denied by sandbox

An extension's Wasm host call was refused by the sandbox: it was made from a
call site that isn't allowed for that operation, the relevant policy flag
(`file_system_access`/`network_access`) is disabled, a file path escaped
`spec_root`/the output directory or used a `..` component, an output extension
is blocked or not allowlisted, an HTTP domain isn't in `allowed_domains`, or a
graph node/edge referenced an undeclared kind/label or a nonexistent node.
Adjust the extension's `sandbox_policy` or the call itself to stay within the
granted permissions.

Owner: core
```

## E054

```
E054: Invalid extension specifier

The extension identifier passed to install couldn't be parsed — it was empty
or didn't match `name@version`, a local path, or a `git+https://...` URL — or
a local install path didn't exist on disk. Use a valid specifier format or check
the local file path.

Owner: core
```

## E055

```
E055: Invalid surface contribution schema

A surface contribution's schema is malformed: an MCP tool's `input_schema` or
`output_schema` isn't a JSON object, or a CLI command declares an argument type
outside the known set (`string`, `path`, `bool`, `enum`, `integer`). Fix the
schema or argument type in the manifest.

Owner: core
```

## E056

```
E056: Failed to write collected report

`specforge collect` ingested test results but couldn't write its aggregated
coverage report file to disk. Check that the output path is writable.

Owner: core
```

## E057

```
E057: Provider scheme conflict

Two extensions both register a `ref` provider for the same scheme. Configure
distinct schemes for each provider extension.

Owner: core
```

## I002

```
I002: Structural-only mode

Emitted when no extensions are installed, or when every installed extension
failed to load, so the compiler falls back to structural-only validation.
Install an extension (for example `specforge add @specforge/software`) to enable
kind-specific checks.

Owner: core
```

## I003

```
I003: No registry configured

The registry configuration has no `registries` array, or none of the configured
registries is marked as the default. Add a `registries` entry and set
`"default_registry": true` on one of them.

Owner: core
```

## I004

```
I004: Extension not installed

A `.spec` file uses a keyword, entity enhancement, or `@scope/name` extension
import that maps to a known but not-installed extension. Install the missing
extension with `specforge add <name>` to resolve the reference. An extension's
enhancement of a kind owned by an extension the project doesn't use is skipped
silently, not reported.

Owner: core
```

## I005

```
I005: Unknown provider scheme

A `ref` entity's `scheme` field, or a `scheme:target` provider reference,
doesn't match any provider scheme registered by an installed extension. Install
an extension that contributes that provider, or configure it in
`specforge.json`.

Owner: core
```

## I006

```
I006: Verify-capable kind not testable

An extension registers an entity kind that supports `verify` statements but has
not marked it `testable`, so its verify obligations won't count toward coverage.
Set `testable: true` in the extension manifest if coverage tracking is desired.

Owner: core
```

## I007

```
I007: Older format version detected

The `.spec` file's declared format version is older than the compiler's current
format version. Run `specforge migrate` to upgrade the file to the current
format.

Owner: core
```

## I010

```
I010: Unreferenced term

A `term` entity has no edges at all, meaning nothing links to or from it via
`see_also` or similar references. Link the term from a relevant entity, or
remove it if it's unused.

Owner: @specforge/product
```

## I013

```
I013: No collector auto-detected

None of the known file patterns matched files in the current project, so no
test-coverage collector could be auto-detected. Pass `--collector` explicitly to
select one.

Owner: core
```

## I016

```
I016: Schema cache missing

Prior exports exist but `.specforge/schema-cache.json` is missing, so
breaking-change detection was skipped for this compilation. Run a full
compilation to regenerate the schema cache.

Owner: core
```

## I017

```
I017: Command not auto-promoted to MCP tool

An extension command would normally be auto-promoted to an MCP tool named
`specforge.<ext>.<command>`, but an explicit MCP tool with that name already
exists. The explicit tool definition takes precedence, so no action is needed
unless the name collision was unintended.

Owner: core
```

## I046

```
I046: Unreferenced persona

A `persona` entity has no incoming edges, meaning no `journey` references it.
Reference the persona from a journey, or remove it if it's no longer needed.

Owner: @specforge/product
```

## I047

```
I047: Unreferenced channel

A `channel` entity has no incoming edges, meaning no `journey` references it.
Reference the channel from a journey, or remove it if it's no longer needed.

Owner: @specforge/product
```

## I059

```
I059: Deferred feature missing reason

A `feature` has `status: deferred` but no `reason` field explaining why. Add a
`reason` field describing why the feature was deferred.

Owner: @specforge/product
```

## I060

```
I060: Blocked milestone missing blockers

A `milestone` has `status: blocked` but no `blockers` field listing what's
blocking it. Add a `blockers` field describing what is blocking progress.

Owner: @specforge/product
```

## I066

```
I066: Deprecated deliverable missing reason

A `deliverable` has `status: deprecated` but no `reason` field explaining why.
Add a `reason` field documenting why it was deprecated.

Owner: @specforge/product
```

## I069

```
I069: Deprecated persona missing reason

A `persona` has `status: deprecated` but no `reason` field explaining why. Add a
`reason` field documenting why it was deprecated.

Owner: @specforge/product
```

## I070

```
I070: Deprecated channel missing reason

A `channel` has `status: deprecated` but no `reason` field explaining why. Add a
`reason` field documenting why it was deprecated.

Owner: @specforge/product
```

## I098

```
I098: Solver could not decide bounds

The `specforge prove` SMT solver returned an undecided result rather than
`sat`/`unsat` when checking combined metric bounds, or whether the declared
bounds entail a claim. Simplify the constraint expressions or supply tighter
bounds so the solver can decide.

Owner: core
```

## I200

```
I200: Stale inferred entities

A source file has changed on disk since it was last analyzed by inference, so
the entities inferred from it may no longer be accurate. Re-analyze the file to
refresh its inferred entities.

Owner: core
```

## I202

```
I202: High inference density

A source file produced an unusually high number of inferred entities relative to
its line count, exceeding the configured density threshold. Review the file for
over-eager inference, or adjust the density threshold if that density is
expected.

Owner: core
```

## I999

```
I999: Diagnostic output truncated

More diagnostics were produced than the 100-diagnostic display limit, so only
the first batch is shown. Fix the listed diagnostics and rerun the compiler to
see the rest.

Owner: core
```

## W001

```
W001: Behavior implements no feature

A `behavior` entity has no outgoing edge to any `feature`, meaning it doesn't
implement anything declared. Add an `implements` reference to a feature, or
remove the behavior if it's unused.

Owner: @specforge/software
```

## W002

```
W002: Unreferenced type

A `type` entity has no incoming references from any `behavior`, `port`, or other
`type`. Reference the type where it's used, or remove it if it's dead.

Owner: @specforge/software
```

## W003

```
W003: Unenforced invariant

An `invariant` entity has no incoming edges from any `behavior`, meaning nothing
enforces it. Add an `enforces` reference from a behavior, or remove the
invariant if it no longer applies.

Owner: @specforge/software
```

## W004

```
W004: Untested testable entity

A testable entity (`behavior`, `invariant`, `event`, `type`, or `port`) declares
no `verify` obligations and no Gherkin scenario, so it has no test linkage. Add
a `verify` block or a Gherkin scenario covering it.

Owner: @specforge/software
```

## W005

```
W005: Unreferenced port

A `port` entity is not referenced by any `behavior`. Reference the port from a
behavior that uses it, or remove it if it's unused.

Owner: @specforge/software
```

## W006

```
W006: Behavior missing category

A `behavior` entity has no `category` field, which agents rely on for task
routing. Add a `category` field to the behavior.

Owner: @specforge/software
```

## W007

```
W007: Event never produced

An `event` entity is not produced by any `behavior`. Add a `produces` reference
from the behavior that emits it, or remove the event if it's unused.

Owner: @specforge/software
```

## W008

```
W008: Unimplemented feature

A `feature` entity has no incoming edge from any `behavior`, meaning nothing
implements it. Add a behavior that implements the feature, or remove it if it's
not planned.

Owner: @specforge/software
```

## W009

```
W009: Disallowed verify kind

An entity uses a `verify` kind (for example `unit`, `contract`, `integration`)
that isn't in the allowed set for its entity kind. Use one of the verify kinds
listed as allowed in the diagnostic.

Owner: @specforge/software
```

## W010

```
W010: Unknown field annotation

A `type` field carries an annotation that isn't recognized by the compiler.
Remove the annotation or correct its spelling.

Owner: @specforge/software
```

## W011

```
W011: Edge references missing node

An edge was about to be added between two entities, but one or both endpoints
don't exist in the graph, so the edge was dropped. Check the referenced entity
IDs for typos or missing definitions.

Owner: core
```

## W012

```
W012: Unreferenced ref entity

A `ref` entity has no incoming edges, meaning nothing in the project references
it. Reference the `ref` from another entity, or remove it if it's unused.

Owner: core
```

## W017

```
W017: Testable kind lacks verify support

An extension registers an entity kind as `testable` but does not set
`supportsVerify: true`, so verify statements can't be declared on it. Set
`supportsVerify: true` in the extension manifest.

Owner: core
```

## W018

```
W018: Duplicate edge type

Two extensions register an edge type with the same label; the first-registered
extension's definition wins and the later one is ignored. Rename one of the
conflicting edge types to avoid the collision.

Owner: core
```

## W019

```
W019: Unknown field type

An extension manifest declares a field whose `field_type` value the compiler
doesn't recognize (it must be one of `string`, `integer`, `bool`, `enum`,
`string_list`, `reference`, `reference_list`, or `block`). Correct the field's
`field_type` in the manifest.

Owner: core
```

## W020

```
W020: Unrecognized field

An entity sets a field that isn't declared for its kind by any installed
extension. Remove the field, fix a typo in its name, or install the extension
that declares it.

Owner: core
```

## W021

```
W021: Undeclared target kind or edge label

A field or edge type references a `target_kind` or edge label that isn't
declared — either in the extension's own manifest when it declares no peer
dependencies, or in the compiler's global kind/edge registry once all extensions
are loaded. Declare the missing kind or edge label, or add the appropriate peer
dependency.

Owner: core
```

## W023

```
W023: Duplicate validation rule code

Two extensions register a validation rule using the same diagnostic code. Change
one extension's rule to use a unique code.

Owner: core
```

## W024

```
W024: Contribution targets an unregistered kind

An extension's grammar or body-parser contribution targets an entity kind that
no installed extension registers, so it is ignored. Register the target kind (or
install the extension that does) before contributing to it.

Owner: core
```

## W025

```
W025: Inaccessible contribution asset

An extension's grammar contribution points at a `.wasm` file that can't be
found, or its body-parser contribution references an export that doesn't exist
in the extension's wasm module. Fix the path or export name in the manifest.

Owner: core
```

## W026

```
W026: Invalid verify kind

A `verify` statement uses a kind that no installed extension has registered, or
uses a kind that is registered but not allowed for that entity's kind. Use one
of the verify kinds listed as allowed in the diagnostic.

Owner: core
```

## W027

```
W027: Re-export binding not found

A selective `pub use { A, B } from "target"` re-export names a binding that
isn't actually exported by the target module. Correct the binding name or remove
it from the re-export list.

Owner: core
```

## W028

```
W028: Extension memory ceiling exceeded

The combined `max_memory_mb` declared across all installed extensions' sandbox
policies exceeds the configured total memory ceiling. Reduce `max_memory_mb` in
one or more extension sandbox policies.

Owner: core
```

## W029

```
W029: Event never consumed

An `event` entity is produced by one or more behaviors but has no consumer,
meaning nothing reacts to it. Add a behavior that consumes the event, or remove
the unused production.

Owner: @specforge/formal
```

## W030

```
W030: Abstract behavior unrefined

A `behavior` marked `abstract true` has no concrete refinement — no behavior
declares `refines` against it, and no `refinement` entity names it as the
abstract entity. Add a concrete behavior with `refines`, or a `refinement`
entity naming this behavior as the `abstract_entity`.

Owner: @specforge/formal
```

## W031

```
W031: Refinement chain too deep

A behavior sits in a refinement chain deeper than the maximum allowed depth of 4
layers. Split the refinement chain, or collapse intermediate abstraction layers.

Owner: @specforge/formal
```

## W035

```
W035: Undischarged coverage items

One or more coverage-tracking items are not covered by any test linkage. Add a
`tests [...]` field pointing at the executable tests that cover them.

Owner: @specforge/formal
```

## W041

```
W041: Orphan feature

A `feature` entity has no incoming edges, meaning no `journey`, `milestone`, or
`module` references it. Link it from at least one referencing entity, or remove
it if it is no longer needed.

Owner: @specforge/product
```

## W042

```
W042: Orphan journey

A `journey` entity has no incoming edges, meaning no `deliverable` references
it. Reference the journey from a deliverable's `journeys` field, or remove it if
it is unused.

Owner: @specforge/product
```

## W044

```
W044: Orphan module

A `module` entity has no incoming edges, meaning no `deliverable` or `milestone`
references it. Reference the module from a deliverable or milestone, or remove
it if it is unused.

Owner: @specforge/product
```

## W045

```
W045: Feature dependency cycle

Two or more `feature` entities form a cycle through their `depends_on` edges.
Break the cycle by removing or restructuring one of the `depends_on` references.

Owner: @specforge/product
```

## W049

```
W049: Empty milestone

A `milestone` entity has neither `features` nor `modules` listed, so it may be
empty. Add at least one `features` or `modules` reference, or remove the
milestone.

Owner: @specforge/product
```

## W050

```
W050: Invalid decision status

A `decision` entity's `status` field is not one of the recognized values
(`proposed`, `accepted`, `deprecated`, `superseded`). Set `status` to one of
these values.

Owner: @specforge/governance
```

## W051

```
W051: Invalid failure mode severity

A `failure_mode` entity's `severity` or `post_severity` field is not one of the
recognized values (`critical`, `high`, `medium`, `low`). Set the field to one of
these values.

Owner: @specforge/governance
```

## W052

```
W052: Invalid failure mode occurrence

A `failure_mode` entity's `occurrence` or `post_occurrence` field is not one of
the recognized values (`certain`, `likely`, `occasional`, `unlikely`, `rare`).
Set the field to one of these values.

Owner: @specforge/governance
```

## W053

```
W053: Breaking schema change

Comparing the graph protocol schema before and after a migration found a change
classified as breaking (e.g. a removed or incompatibly altered field). Review
the migration to ensure it preserves backward compatibility, or accept the break
intentionally.

Owner: core
```

## W054

```
W054: Migration structural drift

Comparing the entity graph before and after a migration found entities or edges
that appeared, disappeared, or changed unexpectedly. Review the migration logic
to ensure it preserves the entities and edges it did not intend to change.

Owner: core
```

## W057

```
W057: Missing milestone exit criteria

A `milestone` entity has `status: completed` but no `exit_criteria` field. Add
an `exit_criteria` field describing how completion was verified.

Owner: @specforge/product
```

## W060

```
W060: Cross-kind ID collision

The same entity ID is declared with two different entity kinds, either within
the same compilation pass or across different files. Entity IDs share one flat
namespace regardless of kind, so rename one of the conflicting declarations; the
first declaration encountered is retained and later ones are skipped.

Owner: core
```

## W061

```
W061: Reference cycle detected

The resolved reference graph contains a cycle among entity references. Break the
cycle by removing or inverting one of the references in the reported path.

Owner: core
```

## W062

```
W062: Malformed semver version

An extension manifest declares a peer dependency range, a version, or a
`host_api_version` that is not valid semver. Use a valid semver version (e.g.
`1.0.0`) or range (e.g. `^1.0.0`, `~1.2.0`, `>=1.0.0`).

Owner: core
```

## W063

```
W063: Circular peer dependency

Two or more installed extensions declare peer dependencies on each other,
forming a cycle. Break the cycle by removing one of the peer dependency
declarations.

Owner: core
```

## W077

```
W077: Invalid feature status

A `feature` entity's `status` field is not one of the recognized values
(`proposed`, `accepted`, `in_progress`, `done`, `deferred`, `deprecated`). Set
`status` to one of these values.

Owner: @specforge/product
```

## W078

```
W078: Invalid priority value

A `feature`, `journey`, `milestone`, or `constraint` entity's `priority` field
is not one of the recognized values (`critical`, `high`, `medium`, `low`). Set
`priority` to one of these values.

Owner: @specforge/product
```

## W079

```
W079: Invalid milestone status

A `milestone` entity's `status` field is not one of the recognized values
(`planned`, `in_progress`, `completed`, `blocked`). Set `status` to one of these
values.

Owner: @specforge/product
```

## W080

```
W080: Invalid deliverable artifact type

A `deliverable` entity's `artifact_type` field is not one of the recognized
values (e.g. `cli`, `service`, `library`, `web_app`, `mobile_app`, `api`,
`extension`, `documentation`, `package`). Set `artifact_type` to one of these
values.

Owner: @specforge/product
```

## W083

```
W083: Invalid persona status

A `persona` entity's `status` field is not one of the recognized values
(`active`, `deprecated`). Set `status` to one of these values.

Owner: @specforge/product
```

## W084

```
W084: Invalid channel status

A `channel` entity's `status` field is not one of the recognized values
(`active`, `deprecated`). Set `status` to one of these values.

Owner: @specforge/product
```

## W085

```
W085: Invalid deliverable status

A `deliverable` entity's `status` field is not one of the recognized values
(`draft`, `in_progress`, `shipped`, `deprecated`). Set `status` to one of these
values.

Owner: @specforge/product
```

## W092

```
W092: Release dependency cycle

Two or more `release` entities form a cycle through their `depends_on` edges.
Break the cycle by removing or restructuring one of the dependency references.

Owner: @specforge/product
```

## W093

```
W093: Invalid release version format

A `release` entity's `version` field does not match semver format (e.g.
`1.0.0`). Set `version` to a valid semver string.

Owner: @specforge/product
```

## W095

```
W095: Invalid feature effort

A `feature` entity's `effort` field is not one of the recognized values (`xs`,
`s`, `m`, `l`, `xl`). Set `effort` to one of these values.

Owner: @specforge/product
```

## W096

```
W096: Behavior requires without ensures

A `behavior` entity declares a `requires` clause (an obligation on callers) but
no `ensures` clause (a guarantee in return). Add an `ensures` clause describing
what the behavior guarantees when its `requires` is satisfied.

Owner: @specforge/formal
```

## W098

```
W098: SMT solver unavailable

The `z3` SMT solver could not be found on `PATH`, or failed to execute, so
formal entailment and consistency checks during `--prove` were skipped. Install
z3 (https://github.com/Z3Prover/z3) and ensure it is executable to enable these
checks.

Owner: core
```

## W099

```
W099: Reference outside import graph

An entity references another entity that resolves only through the global entity
index, not through the referencing file's own declarations or its `use` imports.
Add a `use` import that makes the dependency explicit, even though the reference
still resolves.

Owner: core
```

## W110

```
W110: Refines non-abstract behavior

A behavior's `refines` field names a target behavior that is not marked
`abstract true`. Add `abstract true` to the target behavior, or point `refines`
at a behavior that is actually abstract.

Owner: @specforge/formal
```

## W111

```
W111: Grammar conflict resolved by policy

Two installed extensions register a custom body-parser grammar for the same
entity kind. Depending on the configured conflict policy, the first or the most
recently registered grammar wins; uninstall one of the conflicting extensions or
configure a different policy if the outcome is wrong.

Owner: core
```

## W112

```
W112: Validation rule cannot fire

An extension-declared validation rule cannot work as declared: its `check` kind
is unrecognized, it is missing a field or constraint its check needs, its values
list is empty, its `matches` regex does not compile, or its `wasm_function` is
absent or failed a probe call. Fix or remove the rule in the extension's
manifest.

Owner: core
```

## W113

```
W113: Circular file import

Two or more `.spec` files import each other, forming a cycle in the import
graph. Break the cycle by removing one of the `use` imports or extracting the
shared entities into a separate file.

Owner: core
```

## W114

```
W114: Integrity check skipped

A Wasm extension's integrity check was bypassed because the `--skip-verify` flag
was passed. Remove `--skip-verify` to re-enable hash verification of the
extension's `.wasm` binary.

Owner: core
```

## W115

```
W115: Invalid collector report

A test-coverage collector's output report references an entity ID that does not
match any declared entity, or its `total`/`passed`/`failed`/`skipped` stats are
inconsistent. Fix the collector integration so its report only references known
entity IDs and its counts add up.

Owner: core
```

## W116

```
W116: Extension discovery failure

While scanning an extensions directory, a `manifest.json` could not be read or
read directory itself failed, or a manifest failed to parse as valid JSON
matching the manifest schema. Fix the directory permissions or correct the
malformed `manifest.json`; discovery skips the broken entry and continues with
the rest.

Owner: core
```

## W117

```
W117: Invalid query extension pattern

An extension's tree-sitter query extension pattern (for `highlights`, `locals`,
or `injections`) is empty or contains null bytes. Provide a non-empty query
pattern with no null bytes; the invalid pattern is skipped rather than loaded.

Owner: core
```

## W118

```
W118: Invalid provider configuration

A `providers` entry in `specforge.json` is missing its `alias`/`name` or
`scheme` field, or no installed extension contributes providers to back a
configured provider. Add the missing field, or install an extension that
contributes the provider.

Owner: core
```

## W119

```
W119: Partial install cleanup failed

Rolling back a failed extension install could not remove the partially-created
extension directory. Manually delete the leftover extension directory reported
in the message.

Owner: core
```

## W120

```
W120: Invalid ref target

A `ref` entity's target string is empty or contains control characters. Provide
a clean, non-empty target such as an issue number or ticket key (e.g. `"42"`,
`"PROJ-123"`).

Owner: core
```

## W121

```
W121: Invalid failure mode detection

A `failure_mode` entity's `detection` or `post_detection` field is not one of
the recognized values (`certain`, `likely`, `moderate`, `unlikely`,
`undetectable`). Set the field to one of these values.

Owner: @specforge/governance
```

## W122

```
W122: Duplicate entity ID across files

The same entity ID and kind are declared in more than one `.spec` file. Use
unique entity IDs across files, or use imports to share a single definition
instead of redeclaring it.

Owner: core
```

## W123

```
W123: Orphan property

A `property` entity is not referenced by any `behavior`, so it may be unused.
Reference the property from a behavior's `verify` block, or remove it if it is
no longer needed.

Owner: @specforge/formal
```

## W125

```
W125: Invalid property type

A `property` entity's `property_type` field is not one of the recognized values
(`safety`, `liveness`, `fairness`). Set `property_type` to one of these values.

Owner: @specforge/formal
```

## W126

```
W126: Orphan axiom

An `axiom` entity is not referenced by any other entity, so it may be unused.
Reference the axiom from a relevant entity, or remove it if it is no longer
needed.

Owner: @specforge/formal
```

## W128

```
W128: Orphan protocol

A `protocol` entity is not referenced by any `event`, so it may be unused.
Reference the protocol from an event, or remove it if it is no longer needed.

Owner: @specforge/formal
```

## W131

```
W131: Orphan refinement

A `refinement` entity is not referenced by anything, so it may be orphaned.
Reference the refinement from the entity it refines, or remove it if it is no
longer needed.

Owner: @specforge/formal
```

## W134

```
W134: Orphan process

A `process` entity is not referenced by any other entity, so it may be unused.
Reference the process from a relevant entity, or remove it if it is no longer
needed.

Owner: @specforge/formal
```
