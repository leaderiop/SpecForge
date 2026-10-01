# SpecForge Diagnostic Codes

<!-- Generated file: do not edit by hand. -->

This page is generated from the catalog in `crates/specforge-cli/src/explain.rs`,
the single registry of diagnostic codes; `specforge explain <CODE>` prints the
same text. Every code emitted by the compiler, the CLI, or a first-party
extension has exactly one entry, and each entry names its owner: `core` for the
compiler and CLI, or the `@specforge/<name>` extension that emits it. A test
fails when an emitted code is missing here, is attributed to the wrong owner,
or is listed but never emitted.

Codes follow the pattern `E###` (error), `W###` (warning) and `I###` (info);
`A###` codes are `specforge analyze` findings, whose severity the pass sets.
The registry client keeps its own family, `R###` and `R-<AREA>-###`, whose
prefix doesn't state the severity; no other family is accepted. Each entry's
`Level` is the severity every emit site uses, and a test checks the emit sites
against it. The ranges `E900`-`E998`, `W900`-`W998` and `I900`-`I998` are reserved for
third-party extensions and never appear in this catalog; `I999` is a core code.

Regenerate this page after editing the catalog:

```sh
SPECFORGE_BLESS=1 cargo test -p specforge-cli explain_docs_sync
```

## A001

```
A001: Testable entity without obligations

`specforge analyze coverage` found a testable entity (one whose kind an
extension declares testable) that declares no `verify` obligations, so nothing
states what a test must prove about it. Add `verify unit "..."` (or another
obligation kind) statements. What W004 exempts (union types, abstract entities,
governance kinds) is not reported.

Owner: @specforge/testing
Level: set by the analyze pass
```

## A002

```
A002: Invariant without obligations

An invariant declares no `verify` obligations. It is an error when the
invariant's `risk` is `high`, a warning otherwise. Add a `verify property` or
`verify unit` statement stating how the guarantee is checked.

Owner: @specforge/testing
Level: set by the analyze pass
```

## A010

```
A010: Entity without contract obligations

`specforge analyze contracts` found an entity whose kind registers contract
reference fields (`requires`, `ensures`, `maintains`, ...) that declares none of
them, so no invariant constrains it. Add the references, or ignore this
info-level finding for entities that need none.

Owner: core
Level: set by the analyze pass
```

## A014

```
A014: Failing tests

The recorded test results (`specforge-report.json`, written by `specforge
collect`) include failing tests for this entity, so what it promises is not
proven. Fix the code or the test and run `specforge collect` again.

Owner: @specforge/testing
Level: set by the analyze pass
```

## A015

```
A015: Unproven obligations

With recorded test results, some of an entity's `verify` obligations are named
by no passing test. A test proves an obligation by naming its exact text:
`verify = "..."` in `#[specforge_test(...)]`, or in a vitest test's
`meta.specforge`. Link a test to each listed obligation, or write the missing
tests.

Owner: @specforge/testing
Level: set by the analyze pass
```

## A016

```
A016: Test names an undeclared obligation

A recorded test names a `verify` obligation that its entity does not declare,
usually a typo or an obligation reworded in the spec. The test proves nothing
until its text matches the spec's statement exactly; fix whichever side is
wrong.

Owner: @specforge/testing
Level: set by the analyze pass
```

## E001

```
E001: Parse error

A `.spec` file could not be parsed — invalid syntax such as a missing brace,
unclosed string, or malformed field at the reported location; the same code also
covers an internal reparse failure in the language server, and the formatter's
parser producing no syntax tree at all. Fix the syntax at the reported span and
re-save.

Owner: core
Level: error
```

## E002

```
E002: Duplicate entity ID

Two entities of the same kind declare the same ID; the diagnostic points at the
duplicate and its message names where the ID was first declared (file:line:col).
Rename one of the entities so each ID is unique within its kind.

Owner: core
Level: error
```

## E003

```
E003: Unresolved reference

A reference field names an entity ID that doesn't resolve to any declared
entity. Fix the typo or add the missing entity; a `did you mean` suggestion is
included when a close match exists.

Owner: core
Level: error
```

## E004

```
E004: Unknown type in port method

A `port` entity's method field references a `type` ID that isn't declared
anywhere in the spec. Declare the missing `type` entity or fix the reference to
point at an existing one.

Owner: @specforge/software
Level: error
```

## E006

```
E006: Missing required field

An entity is missing a field marked `required: true` for its kind in the field
registry. Add the missing field to the entity.

Owner: core
Level: error
```

## E007

```
E007: Module dependency cycle

The `depends_on` edges between `module` entities form a cycle. Break the cycle
by removing or inverting one of the dependencies.

Owner: @specforge/product
Level: error
```

## E010

```
E010: Invalid milestone behavior range

A `milestone` entity's `behaviors` range field is malformed (the reported reason
explains what's wrong, e.g. bad syntax or start after end). Correct the range to
a valid form.

Owner: @specforge/software
Level: error
```

## E013

```
E013: Reserved entity ID

An entity's ID is a reserved word — a structural DSL keyword (`spec`, `ref`,
`use`, `define`) or an entity kind keyword contributed by an installed
extension. Rename the entity, e.g. by appending a suffix like `_rule` or
`_spec`.

Owner: core
Level: error
```

## E014

```
E014: Entity ID length violation

An entity ID is outside the 2-60 character identifier contract. Pick a
descriptive identifier within that length.

Owner: core
Level: error
```

## E015

```
E015: Milestone dependency cycle

The `depends_on` edges between `milestone` entities form a cycle. Break the
cycle by removing or inverting one of the dependencies.

Owner: @specforge/product
Level: error
```

## E016

```
E016: Referenced file does not exist

A file-reference field on an entity points at a path that doesn't exist under
the spec root. Fix the path, or create the missing file; a similarly-named file
is suggested when one is found.

Owner: core
Level: error
```

## E017

```
E017: Entity enhancement conflict

Two installed extensions both declare an entity-enhancement field with the same
name on the same target entity kind, and no explicit override resolves it.
Rename one extension's field, or add an override for that kind/field in
`specforge.json`.

Owner: core
Level: error
```

## E018

```
E018: Grammar contribution conflict

Two extensions both contribute a tree-sitter grammar for the same entity kind.
Only one extension may own an entity kind's grammar — uninstall one of the
conflicting extensions or set a grammar conflict policy in the compiler config.

Owner: core
Level: error
```

## E019

```
E019: Unsupported format version

A `.spec` file's `// specforge-format: MAJOR.MINOR` header declares a version
newer than this build supports, or the header itself doesn't parse; `specforge
migrate --target-version` reports the same for a target it can't parse or
doesn't support. Lower the declared version or upgrade SpecForge.

Owner: core
Level: error
```

## E020

```
E020: Missing Wasm export

An extension's manifest declares a contribution (validator, renderer, parser,
collector, grammar, or surface command/tool) whose required Wasm export function
isn't present in the compiled module. Add the matching `#[export_name = "..."]`
export to the extension's Wasm binary.

Owner: core
Level: error
```

## E022

```
E022: Reference targets wrong kind

A reference field is declared to only accept entities of a specific kind, but
the target ID resolves to an entity of a different kind. Point the field at an
entity of the expected kind.

Owner: core
Level: error
```

## E023

```
E023: Entity kind conflicts with keyword

An extension declares an entity kind keyword that collides with a structural DSL
keyword (`spec`, `ref`, `use`, `define`). Choose a different keyword for the
entity kind.

Owner: core
Level: error
```

## E024

```
E024: Unknown entity kind

An entity block uses a kind keyword that neither the core grammar nor any
installed extension declares. Install the extension that provides the keyword,
or fix a typo in the kind name.

Owner: core
Level: error
```

## E025

```
E025: Import resolution failed

A `use` import's target either can't be read from disk or doesn't resolve to any
known `.spec` file. Fix the import path; a `did you mean` suggestion is included
when a close match exists.

Owner: core
Level: error
```

## E026

```
E026: Entity kind registration conflict

Two extensions register the same entity kind keyword; the first registration
wins and the later one is rejected. Rename the conflicting kind keyword.

Owner: core
Level: error
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
Level: error
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
Level: error
```

## E030

```
E030: Invalid extension manifest

An extension's manifest is unreadable, isn't valid JSON, or fails schema
validation — a wrong `manifestVersion`, a missing `name`/`version`/`wasmPath`,
an empty grammar/body-parser/analyzer contribution field, or a sandbox policy
that allowlists a code file extension for output. `specforge publish` reports it
for a `manifest.json` that doesn't parse. Fix the manifest according to the
reported detail.

Owner: core
Level: error
```

## E031

```
E031: Refinement drops ensures condition

A behavior that `refines` an `abstract` behavior drops one or more of the
abstract behavior's `ensures` conditions. A refinement may only strengthen its
abstraction's postconditions, never weaken them — restore or strengthen the
missing `ensures` condition(s) in the concrete behavior.

Owner: @specforge/formal
Level: error
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
Level: error
```

## E033

```
E033: Lock file error

`specforge.lock` couldn't be serialized, written, read, or parsed, or the hash
it records for an installed extension no longer matches the binary on disk;
`specforge update` reports it when there is no lock file to update. Delete the
lock file and reinstall extensions, reinstall the specific extension whose
binary changed, or run `specforge add` first.

Owner: core
Level: error
```

## E035

```
E035: Reserved or invalid entity kind name

An extension-declared entity kind name is a reserved structural keyword, doesn't
match the identifier pattern `[a-z][a-z0-9_]{1,59}`, or is already reserved by
another installed extension. Choose a different, valid entity kind name.

Owner: core
Level: error
```

## E039

```
E039: Duplicate surface contribution

Two extensions register the same CLI surface command ID, the same MCP tool name,
or the same MCP resource name. Rename one extension's contribution so the
identifier is unique across installed extensions.

Owner: core
Level: error
```

## E040

```
E040: Missing extension project file

`specforge extension build`, `extension validate` or `publish` was run against a
directory that's missing its `Cargo.toml` or `manifest.json`, or (`publish`) the
Wasm binary the manifest's `wasmPath` names, or one of those files couldn't be
read. Run the command from a scaffolded extension project, build the binary, or
create the missing file.

Owner: core
Level: error
```

## E041

```
E041: Refinement chain cycle

The `refines` layering graph between behaviors contains a cycle. Break the cycle
by removing or redirecting one of the `refines` edges.

Owner: @specforge/formal
Level: error
```

## E042

```
E042: Process composition cycle

A `process` entity composes, transitively, with itself through its composition
edges. Remove or redirect one of the composition steps to break the cycle.

Owner: @specforge/formal
Level: error
```

## E045

```
E045: Invalid test report

`specforge collect` couldn't get a test report: the runner's command couldn't be
started, it finished without writing a report at the collector's declared
location (often because the tests didn't build), `--no-run` found no existing
report, or a report file couldn't be read. Check the runner's output above the
error and the report path. `specforge analyze` and the MCP coverage, inspect,
query and analyze tools report the same code when `specforge-report.json` (or
`--test-results`) exists but can't be read or parsed, rather than scoring the
project as if no test ran: run `specforge collect` again to rewrite it, or fix
or remove the file.

Owner: core
Level: error
```

## E046

```
E046: Metric bounds are contradictory

`specforge analyze --prove`'s SMT solver found the declared `constraint` metric
bounds mutually unsatisfiable; the cited bounds form the conflicting core. Relax
or correct one of the listed bounds.

Owner: core
Level: error
```

## E048

```
E048: Proof coverage below the minimum

`specforge analyze coverage --min N` found that fewer than N% of testable
entities are proven, where an entity is proven when a passing test names each of
its `verify` obligations or a formal claim discharges it. Prove more
obligations, or lower the threshold. The run exits 1.

Owner: core
Level: error
```

## E051

```
E051: Invalid event trigger

An `event` entity's `trigger` field must reference a `behavior`, but it points
at something else (or nothing resolvable). Point `trigger` at an existing
`behavior` entity.

Owner: @specforge/software
Level: error
```

## E052

```
E052: Deliverable dependency cycle

The `depends_on` edges between `deliverable` entities form a cycle. Break the
cycle by removing or inverting one of the dependencies.

Owner: @specforge/product
Level: error
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
Level: error
```

## E054

```
E054: Invalid extension specifier

The extension identifier passed to install couldn't be parsed — it was empty
or didn't match `name@version`, a local path, or a `git+https://...` URL — or
a local install path didn't exist on disk. Use a valid specifier format or check
the local file path.

Owner: core
Level: error
```

## E055

```
E055: Invalid surface contribution schema

A surface contribution's schema is malformed: an MCP tool's `input_schema` or
`output_schema` isn't a JSON object, or a CLI command declares an argument type
outside the known set (`string`, `path`, `bool`, `enum`, `integer`). Fix the
schema or argument type in the manifest.

Owner: core
Level: error
```

## E056

```
E056: Failed to write collected report

`specforge collect` ingested test results but couldn't write its aggregated
coverage report file to disk. Check that the output path is writable.

Owner: core
Level: error
```

## E057

```
E057: Provider scheme conflict

Two extensions both register a `ref` provider for the same scheme. Configure
distinct schemes for each provider extension.

Owner: core
Level: error
```

## E058

```
E058: No test collector

`specforge collect` found no collector to use: no enabled extension provides
one, none of the enabled collectors' detection files (such as `Cargo.toml`) are
present at the project root, the `--runner` name matches no collector, a
collector declares a report path outside the project, or `--report` was passed
while several collectors apply. Enable a runner extension (e.g. `specforge add
@specforge/cargo-test`) or pick one with `--runner`.

Owner: core
Level: error
```

## E059

```
E059: Test command not approved

A runner extension declares the command that runs its tests, and `specforge
collect` only runs it after you approve it for the project. The approval is
asked at an interactive prompt and remembered per project, extension and
command, in your user-level `~/.specforge/collector-consent.json`, never in the
project. Without a terminal (CI, `--format json`) nothing is asked: pass `--yes`
to run the command, or `--no-run` to parse a report the runner already wrote.

Owner: core
Level: error
```

## E060

```
E060: Resolved reference without a graph edge

A reference list names an entity that exists, but the resolver never turned the
reference into a graph edge. That is a SpecForge bug, not a mistake in your
spec: queries, traces and coverage would miss the relationship. Please report it
with the spec that triggers it.

Owner: core
Level: error
```

## E061

```
E061: Field value is not the declared type

The extension that registers a field declares its type, and the value given
can't be that type: an integer field got something other than an integer, a bool
field something other than true or false, an enum field a value outside its
declared values (the suggestion names the closest one), or a field declared as a
single value got a list. Values that can be read as the declared type are
converted without a diagnostic: a single string or reference on a list field
becomes a one-item list, and a quoted integer or boolean becomes the number or
boolean. Fix the value, or check the field's type with `specforge schema --kind
<kind>`.

Owner: core
Level: error
```

## E062

```
E062: Token budget too small for the export

`specforge export --max-tokens` (or the MCP `export` tool's `max_tokens`) keeps
the most central entities that fit the budget, but some of the export never
shrinks: the envelope (`format_version`, `schema_version`), the `token_budget`
block listing the dropped entity IDs, and, with `--with-schema`, the embedded
schema, which is never cut short. The budget is below that fixed part, so no
export fits. Raise the budget, or drop `--with-schema` when the schema alone is
over it.

Owner: core
Level: error
```

## E063

```
E063: No registry configured

`specforge add @scope/name@version`, `update`, `search`, `publish` and `login`
(and the MCP `add_extension` tool) talk to an extension registry, and SpecForge
has no built-in one: the only registries are those the project's
`specforge.json` lists. None is listed, so the command stopped before making any
network call. Add a `registries` array, for example `"registries": [{"alias":
"main", "url": "<registry URL>", "default_registry": true}]`; an entry with
`"scope_filter": "@acme"` serves only that scope, and the entry marked
`default_registry` serves the rest. For `login`, `--registry <alias>` must name
one of the entries, or one must be the default. Builtin extensions (`specforge
add @specforge/product`) and local `.wasm` files need no registry.

Owner: core
Level: error
```

## E064

```
E064: Unsupported extension source

`specforge add` was given a `git+https://...` extension specifier. It parses,
but installing from git isn't supported yet. Install from a registry
(`name@version`) or from a local `.wasm` path instead.

Owner: core
Level: error
```

## E065

```
E065: Invalid scaffold request

`specforge new` can't scaffold what was asked: only `--extension` projects are
supported, the extension name is empty or is a scoped name that isn't
`@scope/name`, or the destination directory already exists. Pass `--extension`,
fix the name, or pick a destination that doesn't exist yet.

Owner: core
Level: error
```

## E066

```
E066: Extension scaffold failed

`specforge new --extension` couldn't write the project: creating a directory or
writing one of the generated files failed. Check permissions and free space at
the destination, remove the partial project, and retry.

Owner: core
Level: error
```

## E067

```
E067: Invalid registry configuration

The registry configuration in `specforge.json` can't be read: the file isn't
valid JSON, `registries` isn't an array, or an entry at the reported index is
missing a required field or has the wrong type. That part of the configuration
is ignored. Fix the reported entry.

Owner: core
Level: error
```

## E068

```
E068: Coverage gate without the coverage pass

`specforge analyze --min N` gates on proof coverage, which the `coverage` pass
of `@specforge/testing` computes, but that pass didn't run: the extension isn't
enabled, or `--pass` selected a different pass. Enable it with `specforge add
@specforge/testing`, and run the `coverage` (or `all`) pass. The run exits 2.

Owner: core
Level: error
```

## I002

```
I002: Structural-only mode

Emitted when no extensions are installed, or when every installed extension
failed to load, so the compiler falls back to structural-only validation.
Install an extension (for example `specforge add @specforge/software`) to enable
kind-specific checks.

Owner: core
Level: info
```

## I003

```
I003: No registry configured

The registry configuration has no `registries` array, or none of the configured
registries is marked as the default. Add a `registries` entry and set
`"default_registry": true` on one of them.

Owner: core
Level: info
```

## I004

```
I004: Extension not installed

A reference field targets a kind no enabled extension declares, an extension
enhances a kind no enabled extension declares, or a `.spec` file has an
`@scope/name` extension import for a known but not-installed extension. Install
the missing extension with `specforge add <name>` to resolve the reference. An
entity whose keyword no enabled extension declares is an error instead (E024),
whose suggestion names the extension to install. An extension's enhancement of a
kind owned by an extension the project doesn't use is skipped silently, not
reported.

Owner: core
Level: info
```

## I005

```
I005: Unknown provider scheme

A `ref` entity's `scheme` field, or a `scheme:target` provider reference,
doesn't match any provider scheme registered by an installed extension. Install
an extension that contributes that provider, or configure it in
`specforge.json`.

Owner: core
Level: info
```

## I006

```
I006: Verify-capable kind not testable

An extension registers an entity kind that supports `verify` statements but has
not marked it `testable`, so its verify obligations won't count toward coverage.
Set `testable: true` in the extension manifest if coverage tracking is desired.

Owner: core
Level: info
```

## I007

```
I007: Older format version detected

The `.spec` file's declared format version is older than the compiler's current
format version. Run `specforge migrate` to upgrade the file to the current
format.

Owner: core
Level: info
```

## I010

```
I010: Unreferenced term

A `term` entity has no edges at all, meaning nothing links to or from it via
`see_also` or similar references. Link the term from a relevant entity, or
remove it if it's unused.

Owner: @specforge/product
Level: info
```

## I016

```
I016: Schema cache missing

Prior exports exist but `.specforge/schema-cache.json` is missing, so
breaking-change detection was skipped for this compilation. Run a full
compilation to regenerate the schema cache.

Owner: core
Level: info
```

## I017

```
I017: Command not auto-promoted to MCP tool

An extension command would normally be auto-promoted to an MCP tool named
`specforge.<ext>.<command>`, but an explicit MCP tool with that name already
exists. The explicit tool definition takes precedence, so no action is needed
unless the name collision was unintended.

Owner: core
Level: info
```

## I020

```
I020: Unknown entity kind in a filter

A `kinds` filter passed to the `specforge.query` or `specforge.search` MCP tool
names a kind that no loaded extension defines and no entity has. The kind
matches nothing and is dropped from the filter; the report rides in the tool
result's `_meta.diagnostics`, with a `did you mean` suggestion when a known kind
is close. Fix the spelling, or enable the extension that defines the kind.

Owner: core
Level: info
```

## I046

```
I046: Unreferenced persona

A `persona` entity has no incoming edges, meaning no `journey` references it.
Reference the persona from a journey, or remove it if it's no longer needed.

Owner: @specforge/product
Level: info
```

## I047

```
I047: Unreferenced channel

A `channel` entity has no incoming edges, meaning no `journey` references it.
Reference the channel from a journey, or remove it if it's no longer needed.

Owner: @specforge/product
Level: info
```

## I059

```
I059: Deferred feature missing reason

A `feature` has `status: deferred` but no `reason` field explaining why. Add a
`reason` field describing why the feature was deferred.

Owner: @specforge/product
Level: info
```

## I060

```
I060: Blocked milestone missing blockers

A `milestone` has `status: blocked` but no `blockers` field listing what's
blocking it. Add a `blockers` field describing what is blocking progress.

Owner: @specforge/product
Level: info
```

## I066

```
I066: Deprecated deliverable missing reason

A `deliverable` has `status: deprecated` but no `reason` field explaining why.
Add a `reason` field documenting why it was deprecated.

Owner: @specforge/product
Level: info
```

## I069

```
I069: Deprecated persona missing reason

A `persona` has `status: deprecated` but no `reason` field explaining why. Add a
`reason` field documenting why it was deprecated.

Owner: @specforge/product
Level: info
```

## I070

```
I070: Deprecated channel missing reason

A `channel` has `status: deprecated` but no `reason` field explaining why. Add a
`reason` field documenting why it was deprecated.

Owner: @specforge/product
Level: info
```

## I098

```
I098: Solver could not decide bounds

The `specforge analyze --prove` SMT solver returned an undecided result rather
than `sat`/`unsat` when checking combined metric bounds, or whether the declared
bounds entail a claim. Simplify the constraint expressions or supply tighter
bounds so the solver can decide.

Owner: core
Level: info
```

## I200

```
I200: Stale inferred entities

A source file has changed on disk since it was last analyzed by inference, so
the entities inferred from it may no longer be accurate. Re-analyze the file to
refresh its inferred entities.

Owner: core
Level: info
```

## I202

```
I202: High inference density

A source file produced an unusually high number of inferred entities relative to
its line count, exceeding the configured density threshold. Review the file for
over-eager inference, or adjust the density threshold if that density is
expected.

Owner: core
Level: info
```

## I999

```
I999: Diagnostic output truncated

More diagnostics were produced than the 100-diagnostic display limit, so only
the first batch is shown. Fix the listed diagnostics and rerun the compiler to
see the rest.

Owner: core
Level: info
```

## R-AUTH-020

```
R-AUTH-020: Stored registry token expired

The token `specforge login` stored for this registry expired at the reported
time. Log in again with a new token: `specforge login --registry <alias> --token
<NEW_TOKEN>`.

Owner: core
Level: error
```

## R-AUTH-021

```
R-AUTH-021: Stored registry token unreadable

The OS keyring entry that holds this registry's token is missing or can't be
read, although the credentials file refers to it. Log in again: `specforge login
--registry <alias> --token <NEW_TOKEN>`.

Owner: core
Level: error
```

## R-LOGIN-001

```
R-LOGIN-001: No login token given

`specforge login` was run without a token. Pass one with `--token <TOKEN>`.

Owner: core
Level: error
```

## R-LOGIN-002

```
R-LOGIN-002: Login token not stored

`specforge login` couldn't store the token in the OS keyring or in the fallback
file `~/.specforge/credentials.json`. Check that the keyring service is
available and that `~/.specforge` is writable.

Owner: core
Level: error
```

## R-OPS-001

```
R-OPS-001: No registry for the package

No configured registry serves this package: none has a scope that matches it,
and none is marked as the default (or no registries are configured at all). Add
a `registries` entry to `specforge.json` with a matching scope, or mark one
`"default_registry": true`.

Owner: core
Level: error
```

## R-OPS-002

```
R-OPS-002: Package integrity check failed

The SHA-256 hash of the downloaded package doesn't match the hash the registry
published for it, so the download is corrupt or was tampered with. Retry the
download; if it keeps failing, don't install the package.

Owner: core
Level: error
```

## R-OPS-003

```
R-OPS-003: Manifest not serializable

`specforge publish` couldn't serialize the extension manifest to JSON for the
upload. This is a SpecForge bug, not a mistake in your manifest; please report
it.

Owner: core
Level: error
```

## R-RES-001

```
R-RES-001: Package not in the registry

The registry has no package with this name. Check the package name and which
registry the configuration sends it to.

Owner: core
Level: error
```

## R-RES-002

```
R-RES-002: No published versions

The registry lists the package, but no version of it (or no version with a valid
semver number). Ask the publisher to publish a release, or install from another
source.

Owner: core
Level: error
```

## R-RES-003

```
R-RES-003: Invalid version range

The version range isn't valid semver range syntax. Use a range such as `^1.0`,
`~2.3` or `>=1.0.0 <2.0.0`, or `latest`.

Owner: core
Level: error
```

## R-RES-004

```
R-RES-004: No version satisfies the range

The registry has versions of the package, but none inside the requested range;
the message lists the available ones. Widen the range, or pick one of the listed
versions.

Owner: core
Level: error
```

## R-RES-005

```
R-RES-005: Unresolvable version diamond

Several extensions require this package in ranges that no single published
version satisfies (see ADR 0001). Upgrade the requirer with the narrowest range,
or pin a compatible version manually.

Owner: core
Level: error
```

## R-RES-006

```
R-RES-006: Locked peer breaks a version diamond

The extension being added requires a peer at a version the lock file doesn't
have, and the message names a version that would satisfy every requirer. No
command pins peer versions yet: reinstall the peer at that version by hand, then
run `specforge add` again.

Owner: core
Level: error
```

## R-TRUST-001

```
R-TRUST-001: Unsigned package

The registry package carries no publisher signature, so where it came from can't
be verified. Install it anyway only if you trust the source, with
`--allow-unsigned`.

Owner: core
Level: error
```

## R-TRUST-002

```
R-TRUST-002: Invalid package signature

The package's signature is malformed, or it doesn't verify: the Wasm binary or
the manifest isn't what the publisher signed. Don't install the package.

Owner: core
Level: error
```

## R-TRUST-003

```
R-TRUST-003: Publisher key changed

The package is signed with a different key than the one pinned for it when it
was first installed (trust on first use). That can be a key rotation or a
compromised publisher. If you trust the new key, re-run with `--yes` or add it
to `trusted_keys`.

Owner: core
Level: error
```

## R-TRUST-004

```
R-TRUST-004: Signature metadata mismatch

The key ID the registry reports for the package differs from the key ID inside
its signature, so the registry metadata was edited apart from the signature or
is stale. Don't install the package, and check the registry.

Owner: core
Level: error
```

## R-TRUST-005

```
R-TRUST-005: Publisher key denied

The package is signed with a key listed in `denied_keys` in your known-keys
file. Remove the key from `denied_keys` only if you trust it again.

Owner: core
Level: error
```

## R-TRUST-006

```
R-TRUST-006: Known-keys file not writable

The publisher key pinned for the package couldn't be saved to
`~/.specforge/known-keys.json`. Check the permissions on that file and its
directory.

Owner: core
Level: error
```

## R001

```
R001: Registry authentication failed

The registry rejected the request as unauthenticated (HTTP 401), or the
credentials its `auth` configuration names couldn't be read; a request is
retried once with re-read credentials first. Log in again with `specforge login
--registry <alias> --token <TOKEN>`.

Owner: core
Level: error
```

## R002

```
R002: Registry access forbidden

The registry accepted the credentials but refused the request (HTTP 403). Check
your permissions for the registry or the package scope.

Owner: core
Level: error
```

## R003

```
R003: Registry rate limit

The registry is rate limiting requests (HTTP 429); the message says how long to
wait. Retry after that delay.

Owner: core
Level: warning
```

## R004

```
R004: Registry request timed out

A request to the registry didn't answer in time. Check the network connection
and the registry URL, or try again later.

Owner: core
Level: error
```

## R005

```
R005: Registry network error

A request to the registry failed at the network level. Check the network
connection and the registry URL.

Owner: core
Level: error
```

## R006

```
R006: Package version not found

The registry answered 404 for the package or version. Check the package name and
version.

Owner: core
Level: error
```

## R007

```
R007: Version already published

`specforge publish` tried to publish a version that already exists for the
package, and published versions are immutable. Bump the version in the manifest
and publish again.

Owner: core
Level: error
```

## R010

```
R010: Registry token variable not set

The registry's `auth` configuration reads the token from an environment variable
that isn't set. Set it (`export <VAR>=<token>`) or change the registry's `auth`
configuration.

Owner: core
Level: error
```

## R011

```
R011: Registry token file unreadable

The registry's `auth` configuration reads the token from a file that can't be
read. Check that the file exists and is readable, or change the registry's
`auth` configuration.

Owner: core
Level: error
```

## R012

```
R012: Credentials file unreadable

`~/.specforge/credentials.json` can't be read, or isn't in the expected format.
Check its permissions, or delete it and log in again.

Owner: core
Level: error
```

## R013

```
R013: Credentials file not writable

The credentials couldn't be saved: creating `~/.specforge`, serializing the
credentials, or writing `~/.specforge/credentials.json` failed. Check the
permissions on `~/.specforge`.

Owner: core
Level: error
```

## W001

```
W001: Behavior implements no feature

A `behavior` entity has no outgoing edge to any `feature`, meaning it doesn't
implement anything declared. Add an `implements` reference to a feature, or
remove the behavior if it's unused.

Owner: @specforge/software
Level: warning
```

## W002

```
W002: Unreferenced type

A `type` entity has no incoming references from any `behavior`, `port`, or other
`type`. Reference the type where it's used, or remove it if it's dead.

Owner: @specforge/software
Level: warning
```

## W003

```
W003: Unenforced invariant

An `invariant` that nothing references: no behavior lists it in `invariants`,
`requires`, `ensures` or `maintains`, so no code path is bound to preserve it.
Reference it from the behaviors that must preserve it, or remove it if it no
longer applies. `specforge analyze coverage` counts these invariants but does
not report them again.

Owner: @specforge/software
Level: warning
```

## W004

```
W004: Untested testable entity

A testable entity (`behavior`, `invariant`, `event`, `type`, or `port`) declares
no `verify` obligations, so nothing states what a test must prove about it. Add
`verify unit "..."` (or another obligation kind) statements. Union types and
entities marked `abstract true` (through a flag their kind declares) are exempt;
a struct member named `verify` is a field, not an obligation.

Owner: @specforge/testing
Level: warning
```

## W005

```
W005: Unreferenced port

A `port` entity is not referenced by any `behavior`. Reference the port from a
behavior that uses it, or remove it if it's unused.

Owner: @specforge/software
Level: warning
```

## W006

```
W006: Behavior missing category

A `behavior` entity has no `category` field, which agents rely on for task
routing. Add a `category` field to the behavior.

Owner: @specforge/software
Level: warning
```

## W007

```
W007: Event never produced

An `event` entity is not produced by any `behavior`. Add a `produces` reference
from the behavior that emits it, or remove the event if it's unused.

Owner: @specforge/software
Level: warning
```

## W008

```
W008: Unimplemented feature

A `feature` entity has no incoming edge from any `behavior`, meaning nothing
implements it. Add a behavior that implements the feature, or remove it if it's
not planned.

Owner: @specforge/software
Level: warning
```

## W009

```
W009: Disallowed verify kind

An entity uses a `verify` kind (for example `unit`, `contract`, `integration`)
that isn't in the allowed set for its entity kind. Use one of the verify kinds
listed as allowed in the diagnostic.

Owner: @specforge/testing
Level: warning
```

## W010

```
W010: Unknown field annotation

A `type` field carries an annotation that isn't recognized by the compiler.
Remove the annotation or correct its spelling.

Owner: @specforge/software
Level: warning
```

## W011

```
W011: Edge references missing node

An edge was about to be added between two entities, but one or both endpoints
don't exist in the graph, so the edge was dropped. Check the referenced entity
IDs for typos or missing definitions.

Owner: core
Level: warning
```

## W012

```
W012: Unreferenced ref entity

A `ref` entity has no incoming edges, meaning nothing in the project references
it. Reference the `ref` from another entity, or remove it if it's unused.

Owner: core
Level: warning
```

## W017

```
W017: Testable kind lacks verify support

An extension registers an entity kind as `testable` but does not set
`supportsVerify: true`, so verify statements can't be declared on it. Set
`supportsVerify: true` in the extension manifest.

Owner: core
Level: warning
```

## W018

```
W018: Duplicate edge type

Two extensions register an edge type with the same label; the first-registered
extension's definition wins and the later one is ignored. Rename one of the
conflicting edge types to avoid the collision.

Owner: core
Level: warning
```

## W019

```
W019: Unknown field type

An extension manifest declares a field whose `field_type` value the compiler
doesn't recognize (it must be one of `string`, `integer`, `bool`, `enum`,
`string_list`, `reference`, `reference_list`, or `block`). Correct the field's
`field_type` in the manifest.

Owner: core
Level: warning
```

## W020

```
W020: Unrecognized field

An entity sets a field that isn't declared for its kind by any installed
extension. Remove the field, fix a typo in its name, or install the extension
that declares it.

Owner: core
Level: warning
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
Level: warning
```

## W023

```
W023: Duplicate validation rule code

Two extensions register a validation rule using the same diagnostic code. Change
one extension's rule to use a unique code.

Owner: core
Level: warning
```

## W027

```
W027: Re-export binding not found

A selective `pub use { A, B } from "target"` re-export names a binding that
isn't actually exported by the target module. Correct the binding name or remove
it from the re-export list.

Owner: core
Level: warning
```

## W028

```
W028: Extension memory ceiling exceeded

The combined `max_memory_mb` declared across all installed extensions' sandbox
policies exceeds the configured total memory ceiling. Reduce `max_memory_mb` in
one or more extension sandbox policies.

Owner: core
Level: warning
```

## W029

```
W029: Event never consumed

An `event` entity is produced by one or more behaviors but has no consumer,
meaning nothing reacts to it. Add a behavior that consumes the event, or remove
the unused production.

Owner: @specforge/formal
Level: warning
```

## W030

```
W030: Abstract behavior unrefined

A `behavior` marked `abstract true` has no concrete refinement — no behavior
declares `refines` against it, and no `refinement` entity names it as the
abstract entity. Add a concrete behavior with `refines`, or a `refinement`
entity naming this behavior as the `abstract_entity`.

Owner: @specforge/formal
Level: warning
```

## W031

```
W031: Refinement chain too deep

A behavior sits in a refinement chain deeper than the maximum allowed depth of 4
layers. Split the refinement chain, or collapse intermediate abstraction layers.

Owner: @specforge/formal
Level: warning
```

## W035

```
W035: Undischarged coverage items

One or more coverage-tracking items are not covered by any recorded test.
Annotate a test with the entity it proves and run `specforge collect` so its
result is recorded.

Owner: @specforge/formal
Level: warning
```

## W041

```
W041: Orphan feature

A `feature` entity has no incoming edges, meaning no `journey`, `milestone`, or
`module` references it. Link it from at least one referencing entity, or remove
it if it is no longer needed.

Owner: @specforge/product
Level: warning
```

## W042

```
W042: Orphan journey

A `journey` entity has no incoming edges, meaning no `deliverable` references
it. Reference the journey from a deliverable's `journeys` field, or remove it if
it is unused.

Owner: @specforge/product
Level: warning
```

## W044

```
W044: Orphan module

A `module` entity has no incoming edges, meaning no `deliverable` or `milestone`
references it. Reference the module from a deliverable or milestone, or remove
it if it is unused.

Owner: @specforge/product
Level: warning
```

## W045

```
W045: Feature dependency cycle

Two or more `feature` entities form a cycle through their `depends_on` edges.
Break the cycle by removing or restructuring one of the `depends_on` references.

Owner: @specforge/product
Level: warning
```

## W049

```
W049: Empty milestone

A `milestone` entity has neither `features` nor `modules` listed, so it may be
empty. Add at least one `features` or `modules` reference, or remove the
milestone.

Owner: @specforge/product
Level: warning
```

## W050

```
W050: Invalid decision status

A `decision` entity's `status` field is not one of the recognized values
(`proposed`, `accepted`, `deprecated`, `superseded`). Set `status` to one of
these values.

Owner: @specforge/governance
Level: warning
```

## W051

```
W051: Invalid failure mode severity

A `failure_mode` entity's `severity` or `post_severity` field is not one of the
recognized values (`critical`, `high`, `medium`, `low`). Set the field to one of
these values.

Owner: @specforge/governance
Level: warning
```

## W052

```
W052: Invalid failure mode occurrence

A `failure_mode` entity's `occurrence` or `post_occurrence` field is not one of
the recognized values (`certain`, `likely`, `occasional`, `unlikely`, `rare`).
Set the field to one of these values.

Owner: @specforge/governance
Level: warning
```

## W053

```
W053: Breaking schema change

A graph protocol schema change classified as breaking: a removed entity kind,
edge type or field, a new required field, or a field whose type changed.
`specforge export` compares the schema against the one the previous export
cached in `.specforge/schema-cache.json`, so an extension upgrade or removal
that breaks the schema warns on the next export; the export is still written and
the cache updated. `specforge migrate` compares the schema before and after a
migration. Update the agents and tools that read the export, keep the extension
versions that produced the old schema, or review the migration to preserve
backward compatibility.

Owner: core
Level: warning
```

## W054

```
W054: Migration structural drift

Comparing the entity graph before and after a migration found entities or edges
that appeared, disappeared, or changed unexpectedly. Review the migration logic
to ensure it preserves the entities and edges it did not intend to change.

Owner: core
Level: warning
```

## W057

```
W057: Missing milestone exit criteria

A `milestone` entity has `status: completed` but no `exit_criteria` field. Add
an `exit_criteria` field describing how completion was verified.

Owner: @specforge/product
Level: warning
```

## W060

```
W060: Cross-kind ID collision

The same entity ID is declared with two different entity kinds, either within
the same compilation pass or across different files. Entity IDs share one flat
namespace regardless of kind, so rename one of the conflicting declarations; the
first declaration encountered is retained and later ones are skipped.

Owner: core
Level: warning
```

## W061

```
W061: Reference cycle detected

The resolved reference graph contains a cycle among entity references. Break the
cycle by removing or inverting one of the references in the reported path.

Owner: core
Level: warning
```

## W062

```
W062: Malformed semver version

An extension manifest declares a peer dependency range or a version that is not
valid semver. Use a valid semver version (e.g. `1.0.0`) or range (e.g. `^1.0.0`,
`~1.2.0`, `>=1.0.0`).

Owner: core
Level: warning
```

## W077

```
W077: Invalid feature status

A `feature` entity's `status` field is not one of the recognized values
(`proposed`, `accepted`, `in_progress`, `done`, `deferred`, `deprecated`). Set
`status` to one of these values.

Owner: @specforge/product
Level: warning
```

## W078

```
W078: Invalid priority value

A `feature`, `journey`, `milestone`, or `constraint` entity's `priority` field
is not one of the recognized values (`critical`, `high`, `medium`, `low`). Set
`priority` to one of these values.

Owner: @specforge/product
Level: warning
```

## W079

```
W079: Invalid milestone status

A `milestone` entity's `status` field is not one of the recognized values
(`planned`, `in_progress`, `completed`, `blocked`). Set `status` to one of these
values.

Owner: @specforge/product
Level: warning
```

## W080

```
W080: Invalid deliverable artifact type

A `deliverable` entity's `artifact_type` field is not one of the recognized
values (e.g. `cli`, `service`, `library`, `web_app`, `mobile_app`, `api`,
`extension`, `documentation`, `package`). Set `artifact_type` to one of these
values.

Owner: @specforge/product
Level: warning
```

## W083

```
W083: Invalid persona status

A `persona` entity's `status` field is not one of the recognized values
(`active`, `deprecated`). Set `status` to one of these values.

Owner: @specforge/product
Level: warning
```

## W084

```
W084: Invalid channel status

A `channel` entity's `status` field is not one of the recognized values
(`active`, `deprecated`). Set `status` to one of these values.

Owner: @specforge/product
Level: warning
```

## W085

```
W085: Invalid deliverable status

A `deliverable` entity's `status` field is not one of the recognized values
(`draft`, `in_progress`, `shipped`, `deprecated`). Set `status` to one of these
values.

Owner: @specforge/product
Level: warning
```

## W092

```
W092: Release dependency cycle

Two or more `release` entities form a cycle through their `depends_on` edges.
Break the cycle by removing or restructuring one of the dependency references.

Owner: @specforge/product
Level: warning
```

## W093

```
W093: Invalid release version format

A `release` entity's `version` field does not match semver format (e.g.
`1.0.0`). Set `version` to a valid semver string.

Owner: @specforge/product
Level: warning
```

## W095

```
W095: Invalid feature effort

A `feature` entity's `effort` field is not one of the recognized values (`xs`,
`s`, `m`, `l`, `xl`). Set `effort` to one of these values.

Owner: @specforge/product
Level: warning
```

## W096

```
W096: Behavior requires without ensures

A `behavior` entity declares a `requires` clause (an obligation on callers) but
no `ensures` clause (a guarantee in return). Add an `ensures` clause describing
what the behavior guarantees when its `requires` is satisfied.

Owner: @specforge/formal
Level: warning
```

## W097

```
W097: Test record for an unknown entity

`specforge analyze` read a recorded test result (in `specforge-report.json`) for
an entity ID that no spec declares, so the result counts toward nothing; a `did
you mean` hint names a close match when there is one. Fix the entity ID the test
names, or declare the entity.

Owner: core
Level: warning
```

## W098

```
W098: SMT solver unavailable

The `z3` SMT solver could not be found on `PATH`, or failed to execute, so
formal entailment and consistency checks during `--prove` were skipped. Install
z3 (https://github.com/Z3Prover/z3) and ensure it is executable to enable these
checks.

Owner: core
Level: warning
```

## W110

```
W110: Refines non-abstract behavior

A behavior's `refines` field names a target behavior that is not marked
`abstract true`. Add `abstract true` to the target behavior, or point `refines`
at a behavior that is actually abstract.

Owner: @specforge/formal
Level: warning
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
Level: warning
```

## W113

```
W113: Circular file import

Two or more `.spec` files import each other, forming a cycle in the import
graph. Break the cycle by removing one of the `use` imports or extracting the
shared entities into a separate file.

Owner: core
Level: warning
```

## W114

```
W114: Integrity check skipped

A Wasm extension's integrity check was bypassed because the `--skip-verify` flag
was passed. Remove `--skip-verify` to re-enable hash verification of the
extension's `.wasm` binary.

Owner: core
Level: warning
```

## W115

```
W115: Invalid collector report

A collector reported tests for an entity ID that no spec declares (usually a
renamed entity or a typo in the test's annotation), or its
`total`/`passed`/`failed`/`skipped` stats are inconsistent. `specforge collect`
drops those results; fix the test annotation so it names a declared entity.

Owner: core
Level: warning
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
Level: warning
```

## W117

```
W117: Invalid query extension pattern

An extension's tree-sitter query extension pattern (for `highlights`, `locals`,
or `injections`) is empty or contains null bytes. Provide a non-empty query
pattern with no null bytes; the invalid pattern is skipped rather than loaded.

Owner: core
Level: warning
```

## W118

```
W118: Invalid provider configuration

A `providers` entry in `specforge.json` is missing its `alias`/`name` or
`scheme` field, or no installed extension contributes providers to back a
configured provider. Add the missing field, or install an extension that
contributes the provider.

Owner: core
Level: warning
```

## W119

```
W119: Partial install cleanup failed

Rolling back a failed extension install could not remove the partially-created
extension directory. Manually delete the leftover extension directory reported
in the message.

Owner: core
Level: warning
```

## W121

```
W121: Invalid failure mode detection

A `failure_mode` entity's `detection` or `post_detection` field is not one of
the recognized values (`certain`, `likely`, `moderate`, `unlikely`,
`undetectable`). Set the field to one of these values.

Owner: @specforge/governance
Level: warning
```

## W123

```
W123: Orphan property

A `property` entity is not referenced by any `behavior`, so it may be unused.
Reference the property from a behavior's `verify` block, or remove it if it is
no longer needed.

Owner: @specforge/formal
Level: warning
```

## W125

```
W125: Invalid property type

A `property` entity's `property_type` field is not one of the recognized values
(`safety`, `liveness`, `fairness`). Set `property_type` to one of these values.

Owner: @specforge/formal
Level: warning
```

## W126

```
W126: Orphan axiom

An `axiom` entity is not referenced by any other entity, so it may be unused.
Reference the axiom from a relevant entity, or remove it if it is no longer
needed.

Owner: @specforge/formal
Level: warning
```

## W128

```
W128: Orphan protocol

A `protocol` entity is not referenced by any `event`, so it may be unused.
Reference the protocol from an event, or remove it if it is no longer needed.

Owner: @specforge/formal
Level: warning
```

## W131

```
W131: Orphan refinement

A `refinement` entity is not referenced by anything, so it may be orphaned.
Reference the refinement from the entity it refines, or remove it if it is no
longer needed.

Owner: @specforge/formal
Level: warning
```

## W134

```
W134: Orphan process

A `process` entity is not referenced by any other entity, so it may be unused.
Reference the process from a relevant entity, or remove it if it is no longer
needed.

Owner: @specforge/formal
Level: warning
```

## W137

```
W137: Ambiguous convention mapping

`specforge collect` links a test that no annotation links by its name:
`entity_id__obligation_slug`, or a module named after an entity. This test's
name splits at more than one `__` into a declared entity ID (entity IDs aren't
meant to contain `__`), so it isn't linked. Rename the test or the entity, or
link the test explicitly (`#[specforge_test]`).

Owner: core
Level: warning
```

## W138

```
W138: Unknown manifest field

An extension's manifest.json has a top-level field the v2 manifest schema
doesn't define, so SpecForge ignores it. It is usually a misspelling
(`entityKnds` for `entityKinds`) that leaves the extension without what the
field was meant to declare. Fix the spelling or remove the field.

Owner: core
Level: warning
```

## W139

```
W139: Formal claim not entailed by declared bounds

`specforge analyze --prove` found that a `claim` isn't guaranteed by the
declared metric bounds: the SMT solver found a counterexample that satisfies
every bound while violating the claim. The claim isn't wrong; the bounds just
don't guarantee it yet, and its `verify property` obligation stays unproven.
Strengthen the declared constraint bounds or weaken the claim. Use `--strict` to
fail the run on it. This code was E047 until it was renumbered to match its
severity.

Owner: core
Level: warning
```

## W140

```
W140: Duplicate registry alias

Two entries in `registries` in `specforge.json` share an alias, so the alias
doesn't name one registry. Give each registry a unique alias.

Owner: core
Level: warning
```

## W141

```
W141: Invalid formatter configuration

The formatter's `.specforgefmt.toml` can't be read, isn't valid TOML, or sets
`indent_width` (an integer from 1 to 16), `use_tabs` (a boolean) or `max_width`
(an integer from 40 to 200) to an invalid value. The formatter uses the default
for anything it can't use. Fix the reported setting.

Owner: core
Level: warning
```

## W142

```
W142: Unparseable region left unformatted

The formatter hit a parse error in a `.spec` file. It keeps the reported line
range exactly as written and formats the rest. Fix the syntax there (see E001)
and format again.

Owner: core
Level: warning
```

## W143

```
W143: Define blocks are not supported

A `.spec` file has a `define <name> { ... }` block. Every entity kind comes from
an extension, so a project's kinds depend only on `specforge.json` (ADR 0005):
the block registers nothing and is left out of the graph. Declare the kind in an
extension (`specforge new --extension`) and enable it, then remove the block.
`define` stays a reserved word.

Owner: core
Level: warning
```

## Retired codes

These codes are no longer emitted, and are never reused for another meaning.

| Code | Replaced by |
|------|-------------|
| E029 | (nothing) |
| E037 | (nothing) |
| E038 | (nothing) |
| E047 | [W139](#w139) |
| W024 | (nothing) |
| W025 | (nothing) |
| W026 | (nothing) |
| W063 | (nothing) |
| W099 | (nothing) |
| W111 | (nothing) |
| W120 | (nothing) |
| W122 | (nothing) |
