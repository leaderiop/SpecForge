//! Diagnostic code registry: the single canonical catalog of every code the
//! compiler, the CLI, and the first-party extensions emit.
//!
//! `specforge explain <CODE>` prints an entry, `docs/diagnostics.md` is
//! generated from [`CATALOG`], and the tests at the bottom of this file fail
//! when an emitted code is missing from the catalog, is attributed to the
//! wrong owner, or when the catalog lists a code nothing emits.

/// One diagnostic code and what it means.
#[derive(Debug, Clone, Copy)]
pub struct CodeEntry {
    /// `E###` (error), `W###` (warning) or `I###` (info).
    pub code: &'static str,
    /// Short human-readable name.
    pub title: &'static str,
    /// `"core"` for the compiler/CLI, or the emitting extension
    /// (`"@specforge/formal"`, ...).
    pub owner: &'static str,
    /// What triggers the diagnostic and how to fix it.
    pub explanation: &'static str,
}

/// Width used when wrapping explanations for the terminal and the docs page.
const WRAP_WIDTH: usize = 80;

/// Look up a code (case-insensitive).
pub fn lookup(code: &str) -> Option<&'static CodeEntry> {
    let normalized = code.to_uppercase();
    CATALOG
        .binary_search_by(|entry| entry.code.cmp(normalized.as_str()))
        .ok()
        .map(|index| &CATALOG[index])
}

/// Print the explanation of a diagnostic code.
pub fn run(code: &str) -> i32 {
    match lookup(code) {
        Some(entry) => {
            println!("\x1b[1m{}\x1b[0m: {}\n", entry.code, entry.title);
            println!("{}", wrap(entry.explanation, WRAP_WIDTH));
            println!("\nOwner: {}", entry.owner);
            0
        }
        None => {
            eprintln!("unknown diagnostic code: {code}");
            eprintln!("hint: codes follow the pattern E### (error), W### (warning), I### (info)");
            eprintln!(
                "hint: E900-E998, W900-W998 and I900-I998 are reserved for third-party extensions; \
                 see the extension's own documentation"
            );
            1
        }
    }
}

/// Greedy word wrap; never splits a word.
fn wrap(text: &str, width: usize) -> String {
    let mut out = String::new();
    let mut line_len = 0;
    for word in text.split_whitespace() {
        if line_len > 0 && line_len + 1 + word.len() > width {
            out.push('\n');
            line_len = 0;
        } else if line_len > 0 {
            out.push(' ');
            line_len += 1;
        }
        out.push_str(word);
        line_len += word.len();
    }
    out
}

/// Render `docs/diagnostics.md` from [`CATALOG`] (see the `explain_docs_sync` test).
#[cfg(test)]
fn render_docs() -> String {
    let mut out = String::from(DOCS_HEADER);
    for entry in CATALOG {
        out.push_str(&format!(
            "\n## {code}\n\n```\n{code}: {title}\n\n{body}\n\nOwner: {owner}\n```\n",
            code = entry.code,
            title = entry.title,
            body = wrap(entry.explanation, WRAP_WIDTH),
            owner = entry.owner,
        ));
    }
    out
}

#[cfg(test)]
const DOCS_HEADER: &str = "# SpecForge Diagnostic Codes

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
";

/// Every diagnostic code emitted today, sorted by code.
pub const CATALOG: &[CodeEntry] = &[
    CodeEntry {
        code: "E001",
        title: "Parse error",
        owner: "core",
        explanation: "A `.spec` file could not be parsed — invalid syntax such as a missing brace, unclosed string, or malformed field at the reported location; the same code also covers an internal reparse failure in the language server. Fix the syntax at the reported span and re-save.",
    },
    CodeEntry {
        code: "E002",
        title: "Duplicate entity ID",
        owner: "core",
        explanation: "Two entities of the same kind declare the same ID; the message names the file where the ID was first declared. Rename one of the entities so each ID is unique within its kind.",
    },
    CodeEntry {
        code: "E003",
        title: "Unresolved reference",
        owner: "core",
        explanation: "A reference field names an entity ID that doesn't resolve to any declared entity. Fix the typo or add the missing entity; a `did you mean` suggestion is included when a close match exists.",
    },
    CodeEntry {
        code: "E004",
        title: "Unknown type in port method",
        owner: "@specforge/software",
        explanation: "A `port` entity's method field references a `type` ID that isn't declared anywhere in the spec. Declare the missing `type` entity or fix the reference to point at an existing one.",
    },
    CodeEntry {
        code: "E006",
        title: "Missing required field",
        owner: "core",
        explanation: "An entity is missing a field marked `required: true` for its kind in the field registry. Add the missing field to the entity.",
    },
    CodeEntry {
        code: "E007",
        title: "Module dependency cycle",
        owner: "@specforge/product",
        explanation: "The `depends_on` edges between `module` entities form a cycle. Break the cycle by removing or inverting one of the dependencies.",
    },
    CodeEntry {
        code: "E010",
        title: "Invalid milestone behavior range",
        owner: "@specforge/software",
        explanation: "A `milestone` entity's `behaviors` range field is malformed (the reported reason explains what's wrong, e.g. bad syntax or start after end). Correct the range to a valid form.",
    },
    CodeEntry {
        code: "E013",
        title: "Reserved entity ID",
        owner: "core",
        explanation: "An entity's ID is a reserved word — a structural DSL keyword (`spec`, `ref`, `use`, `define`) or an entity kind keyword contributed by an installed extension. Rename the entity, e.g. by appending a suffix like `_rule` or `_spec`.",
    },
    CodeEntry {
        code: "E014",
        title: "Entity ID length violation",
        owner: "core",
        explanation: "An entity ID is outside the 2-60 character identifier contract. Pick a descriptive identifier within that length.",
    },
    CodeEntry {
        code: "E015",
        title: "Milestone dependency cycle",
        owner: "@specforge/product",
        explanation: "The `depends_on` edges between `milestone` entities form a cycle. Break the cycle by removing or inverting one of the dependencies.",
    },
    CodeEntry {
        code: "E016",
        title: "Referenced file does not exist",
        owner: "core",
        explanation: "A file-reference field on an entity points at a path that doesn't exist under the spec root. Fix the path, or create the missing file; a similarly-named file is suggested when one is found.",
    },
    CodeEntry {
        code: "E017",
        title: "Entity enhancement conflict",
        owner: "core",
        explanation: "Two installed extensions both declare an entity-enhancement field with the same name on the same target entity kind, and no explicit override resolves it. Rename one extension's field, or add an override for that kind/field in `specforge.json`.",
    },
    CodeEntry {
        code: "E018",
        title: "Grammar contribution conflict",
        owner: "core",
        explanation: "Two extensions both contribute a tree-sitter grammar for the same entity kind. Only one extension may own an entity kind's grammar — uninstall one of the conflicting extensions or set a grammar conflict policy in the compiler config.",
    },
    CodeEntry {
        code: "E019",
        title: "Unsupported format version",
        owner: "core",
        explanation: "A `.spec` file's `// specforge-format: MAJOR.MINOR` header declares a version newer than this build supports, or the header itself doesn't parse. Lower the declared version or upgrade SpecForge.",
    },
    CodeEntry {
        code: "E020",
        title: "Missing Wasm export",
        owner: "core",
        explanation: "An extension's manifest declares a contribution (validator, renderer, parser, collector, grammar, or surface command/tool) whose required Wasm export function isn't present in the compiled module. Add the matching `#[export_name = \"...\"]` export to the extension's Wasm binary.",
    },
    CodeEntry {
        code: "E022",
        title: "Reference targets wrong kind",
        owner: "core",
        explanation: "A reference field is declared to only accept entities of a specific kind, but the target ID resolves to an entity of a different kind. Point the field at an entity of the expected kind.",
    },
    CodeEntry {
        code: "E023",
        title: "Entity kind conflicts with keyword",
        owner: "core",
        explanation: "An extension declares an entity kind keyword that collides with a structural DSL keyword (`spec`, `ref`, `use`, `define`). Choose a different keyword for the entity kind.",
    },
    CodeEntry {
        code: "E024",
        title: "Unknown entity kind",
        owner: "core",
        explanation: "An entity block uses a kind keyword that neither the core grammar nor any installed extension declares. Install the extension that provides the keyword, or fix a typo in the kind name.",
    },
    CodeEntry {
        code: "E025",
        title: "Import resolution failed",
        owner: "core",
        explanation: "A `use` import's target either can't be read from disk or doesn't resolve to any known `.spec` file. Fix the import path; a `did you mean` suggestion is included when a close match exists.",
    },
    CodeEntry {
        code: "E026",
        title: "Entity kind registration conflict",
        owner: "core",
        explanation: "Two extensions, or a project `define` block and an extension, register the same entity kind keyword; the first registration wins and the later one is rejected. Rename the conflicting kind keyword.",
    },
    CodeEntry {
        code: "E027",
        title: "Unsatisfiable peer dependency",
        owner: "core",
        explanation: "An extension's required peer dependency can't be satisfied: it isn't installed, the installed version doesn't match the required range, peer dependencies form a cycle, an uninstall would remove an extension others still require, or an upgrade would break a peer's requirement. Install or upgrade the named peer, or use `--force` where the command supports it.",
    },
    CodeEntry {
        code: "E028",
        title: "Extension load or execution failure",
        owner: "core",
        explanation: "An extension's Wasm module failed somewhere in its lifecycle — the binary is missing or unreadable, failed to load, trapped while running `initialize`/`validate`/a collector/body parser/surface command/MCP tool or resource, returned output that isn't valid JSON, a grammar cache path couldn't be written, or the extension's declared host API version isn't supported. Check the extension's logs or report the trap to its author, and confirm the extension is installed and up to date.",
    },
    CodeEntry {
        code: "E029",
        title: "Duplicate body parser registration",
        owner: "core",
        explanation: "Two extensions both register a body parser for the same entity kind. At most one body parser is allowed per entity kind — uninstall or reconfigure one of the conflicting extensions.",
    },
    CodeEntry {
        code: "E030",
        title: "Invalid extension manifest",
        owner: "core",
        explanation: "An extension's manifest is unreadable, isn't valid JSON, or fails schema validation — a wrong `manifestVersion`, a missing `name`/`version`/`wasmPath`, an empty grammar/body-parser/analyzer contribution field, or a sandbox policy that allowlists a code file extension for output. Fix the manifest according to the reported detail.",
    },
    CodeEntry {
        code: "E031",
        title: "Refinement drops ensures condition",
        owner: "@specforge/formal",
        explanation: "A behavior that `refines` an `abstract` behavior drops one or more of the abstract behavior's `ensures` conditions. A refinement may only strengthen its abstraction's postconditions, never weaken them — restore or strengthen the missing `ensures` condition(s) in the concrete behavior.",
    },
    CodeEntry {
        code: "E032",
        title: "Extension install or uninstall failed",
        owner: "core",
        explanation: "An install or uninstall step failed: the downloaded `.wasm` binary's SHA-256 hash didn't match the expected value (possible tampering or a bad download), or a filesystem step — creating the temp directory, writing the binary, finalizing the install, or removing the extension directory on uninstall — failed. Re-download the extension or check filesystem permissions.",
    },
    CodeEntry {
        code: "E033",
        title: "Lock file error",
        owner: "core",
        explanation: "`specforge.lock` couldn't be serialized, written, read, or parsed, or the hash it records for an installed extension no longer matches the binary on disk. Delete the lock file and reinstall extensions, or reinstall the specific extension whose binary changed.",
    },
    CodeEntry {
        code: "E035",
        title: "Reserved or invalid entity kind name",
        owner: "core",
        explanation: "An extension-declared entity kind name is a reserved structural keyword, doesn't match the identifier pattern `[a-z][a-z0-9_]{1,59}`, or is already reserved by another installed extension. Choose a different, valid entity kind name.",
    },
    CodeEntry {
        code: "E037",
        title: "Grammar ABI version mismatch",
        owner: "core",
        explanation: "A tree-sitter grammar `.wasm` binary was built against an ABI version that doesn't match the version this SpecForge build supports. Rebuild the grammar targeting the supported tree-sitter ABI version.",
    },
    CodeEntry {
        code: "E038",
        title: "Grammar binary too large",
        owner: "core",
        explanation: "A tree-sitter grammar `.wasm` binary exceeds the configured maximum size. Reduce the grammar's complexity or raise `max_size_bytes` in the compiler config.",
    },
    CodeEntry {
        code: "E039",
        title: "Duplicate surface contribution",
        owner: "core",
        explanation: "Two extensions register the same CLI surface command ID, the same MCP tool name, or the same MCP resource name. Rename one extension's contribution so the identifier is unique across installed extensions.",
    },
    CodeEntry {
        code: "E040",
        title: "Missing extension project file",
        owner: "core",
        explanation: "`specforge extension build` or `validate` was run against a directory that's missing its `Cargo.toml` or `manifest.json`. Run the command from a scaffolded extension project, or create the missing file.",
    },
    CodeEntry {
        code: "E041",
        title: "Refinement chain cycle",
        owner: "@specforge/formal",
        explanation: "The `refines` layering graph between behaviors contains a cycle. Break the cycle by removing or redirecting one of the `refines` edges.",
    },
    CodeEntry {
        code: "E042",
        title: "Process composition cycle",
        owner: "@specforge/formal",
        explanation: "A `process` entity composes, transitively, with itself through its composition edges. Remove or redirect one of the composition steps to break the cycle.",
    },
    CodeEntry {
        code: "E045",
        title: "Invalid test report",
        owner: "core",
        explanation: "`specforge collect` couldn't get a test report: the runner's command couldn't be started, it finished without writing a report at the collector's declared location (often because the tests didn't build), `--no-run` found no existing report, or a report file couldn't be read. Check the runner's output above the error and the report path.",
    },
    CodeEntry {
        code: "E046",
        title: "Metric bounds are contradictory",
        owner: "core",
        explanation: "`specforge prove`'s SMT solver found the declared `constraint` metric bounds mutually unsatisfiable; the cited bounds form the conflicting core. Relax or correct one of the listed bounds.",
    },
    CodeEntry {
        code: "E047",
        title: "Formal claim not entailed by declared bounds",
        owner: "core",
        explanation: "`specforge prove` found that a `claim` isn't guaranteed by the declared metric bounds — a counterexample satisfying the bounds while violating the claim was found. Strengthen the declared constraint bounds or weaken the claim.",
    },
    CodeEntry {
        code: "E051",
        title: "Invalid event trigger",
        owner: "@specforge/software",
        explanation: "An `event` entity's `trigger` field must reference a `behavior`, but it points at something else (or nothing resolvable). Point `trigger` at an existing `behavior` entity.",
    },
    CodeEntry {
        code: "E052",
        title: "Deliverable dependency cycle",
        owner: "@specforge/product",
        explanation: "The `depends_on` edges between `deliverable` entities form a cycle. Break the cycle by removing or inverting one of the dependencies.",
    },
    CodeEntry {
        code: "E053",
        title: "Host call denied by sandbox",
        owner: "core",
        explanation: "An extension's Wasm host call was refused by the sandbox: it was made from a call site that isn't allowed for that operation, the relevant policy flag (`file_system_access`/`network_access`) is disabled, a file path escaped `spec_root`/the output directory or used a `..` component, an output extension is blocked or not allowlisted, an HTTP domain isn't in `allowed_domains`, or a graph node/edge referenced an undeclared kind/label or a nonexistent node. Adjust the extension's `sandbox_policy` or the call itself to stay within the granted permissions.",
    },
    CodeEntry {
        code: "E054",
        title: "Invalid extension specifier",
        owner: "core",
        explanation: "The extension identifier passed to install couldn't be parsed — it was empty or didn't match `name@version`, a local path, or a `git+https://...` URL — or a local install path didn't exist on disk. Use a valid specifier format or check the local file path.",
    },
    CodeEntry {
        code: "E055",
        title: "Invalid surface contribution schema",
        owner: "core",
        explanation: "A surface contribution's schema is malformed: an MCP tool's `input_schema` or `output_schema` isn't a JSON object, or a CLI command declares an argument type outside the known set (`string`, `path`, `bool`, `enum`, `integer`). Fix the schema or argument type in the manifest.",
    },
    CodeEntry {
        code: "E056",
        title: "Failed to write collected report",
        owner: "core",
        explanation: "`specforge collect` ingested test results but couldn't write its aggregated coverage report file to disk. Check that the output path is writable.",
    },
    CodeEntry {
        code: "E057",
        title: "Provider scheme conflict",
        owner: "core",
        explanation: "Two extensions both register a `ref` provider for the same scheme. Configure distinct schemes for each provider extension.",
    },
    CodeEntry {
        code: "E058",
        title: "No test collector",
        owner: "core",
        explanation: "`specforge collect` found no collector to use: no enabled extension provides one, none of the enabled collectors' detection files (such as `Cargo.toml`) are present at the project root, the `--runner` name matches no collector, a collector declares a report path outside the project, or `--report` was passed while several collectors apply. Enable a runner extension (e.g. `specforge add @specforge/cargo-test`) or pick one with `--runner`.",
    },
    CodeEntry {
        code: "E059",
        title: "Test command not approved",
        owner: "core",
        explanation: "A runner extension declares the command that runs its tests, and `specforge collect` only runs it after you approve it for the project. The approval is asked at an interactive prompt and remembered per project, extension and command, in your user-level `~/.specforge/collector-consent.json`, never in the project. Without a terminal (CI, `--format json`) nothing is asked: pass `--yes` to run the command, or `--no-run` to parse a report the runner already wrote.",
    },
    CodeEntry {
        code: "I002",
        title: "Structural-only mode",
        owner: "core",
        explanation: "Emitted when no extensions are installed, or when every installed extension failed to load, so the compiler falls back to structural-only validation. Install an extension (for example `specforge add @specforge/software`) to enable kind-specific checks.",
    },
    CodeEntry {
        code: "I003",
        title: "No registry configured",
        owner: "core",
        explanation: "The registry configuration has no `registries` array, or none of the configured registries is marked as the default. Add a `registries` entry and set `\"default_registry\": true` on one of them.",
    },
    CodeEntry {
        code: "I004",
        title: "Extension not installed",
        owner: "core",
        explanation: "A `.spec` file uses a keyword, entity enhancement, or `@scope/name` extension import that maps to a known but not-installed extension. Install the missing extension with `specforge add <name>` to resolve the reference. An extension's enhancement of a kind owned by an extension the project doesn't use is skipped silently, not reported.",
    },
    CodeEntry {
        code: "I005",
        title: "Unknown provider scheme",
        owner: "core",
        explanation: "A `ref` entity's `scheme` field, or a `scheme:target` provider reference, doesn't match any provider scheme registered by an installed extension. Install an extension that contributes that provider, or configure it in `specforge.json`.",
    },
    CodeEntry {
        code: "I006",
        title: "Verify-capable kind not testable",
        owner: "core",
        explanation: "An extension registers an entity kind that supports `verify` statements but has not marked it `testable`, so its verify obligations won't count toward coverage. Set `testable: true` in the extension manifest if coverage tracking is desired.",
    },
    CodeEntry {
        code: "I007",
        title: "Older format version detected",
        owner: "core",
        explanation: "The `.spec` file's declared format version is older than the compiler's current format version. Run `specforge migrate` to upgrade the file to the current format.",
    },
    CodeEntry {
        code: "I010",
        title: "Unreferenced term",
        owner: "@specforge/product",
        explanation: "A `term` entity has no edges at all, meaning nothing links to or from it via `see_also` or similar references. Link the term from a relevant entity, or remove it if it's unused.",
    },
    CodeEntry {
        code: "I016",
        title: "Schema cache missing",
        owner: "core",
        explanation: "Prior exports exist but `.specforge/schema-cache.json` is missing, so breaking-change detection was skipped for this compilation. Run a full compilation to regenerate the schema cache.",
    },
    CodeEntry {
        code: "I017",
        title: "Command not auto-promoted to MCP tool",
        owner: "core",
        explanation: "An extension command would normally be auto-promoted to an MCP tool named `specforge.<ext>.<command>`, but an explicit MCP tool with that name already exists. The explicit tool definition takes precedence, so no action is needed unless the name collision was unintended.",
    },
    CodeEntry {
        code: "I046",
        title: "Unreferenced persona",
        owner: "@specforge/product",
        explanation: "A `persona` entity has no incoming edges, meaning no `journey` references it. Reference the persona from a journey, or remove it if it's no longer needed.",
    },
    CodeEntry {
        code: "I047",
        title: "Unreferenced channel",
        owner: "@specforge/product",
        explanation: "A `channel` entity has no incoming edges, meaning no `journey` references it. Reference the channel from a journey, or remove it if it's no longer needed.",
    },
    CodeEntry {
        code: "I059",
        title: "Deferred feature missing reason",
        owner: "@specforge/product",
        explanation: "A `feature` has `status: deferred` but no `reason` field explaining why. Add a `reason` field describing why the feature was deferred.",
    },
    CodeEntry {
        code: "I060",
        title: "Blocked milestone missing blockers",
        owner: "@specforge/product",
        explanation: "A `milestone` has `status: blocked` but no `blockers` field listing what's blocking it. Add a `blockers` field describing what is blocking progress.",
    },
    CodeEntry {
        code: "I066",
        title: "Deprecated deliverable missing reason",
        owner: "@specforge/product",
        explanation: "A `deliverable` has `status: deprecated` but no `reason` field explaining why. Add a `reason` field documenting why it was deprecated.",
    },
    CodeEntry {
        code: "I069",
        title: "Deprecated persona missing reason",
        owner: "@specforge/product",
        explanation: "A `persona` has `status: deprecated` but no `reason` field explaining why. Add a `reason` field documenting why it was deprecated.",
    },
    CodeEntry {
        code: "I070",
        title: "Deprecated channel missing reason",
        owner: "@specforge/product",
        explanation: "A `channel` has `status: deprecated` but no `reason` field explaining why. Add a `reason` field documenting why it was deprecated.",
    },
    CodeEntry {
        code: "I098",
        title: "Solver could not decide bounds",
        owner: "core",
        explanation: "The `specforge prove` SMT solver returned an undecided result rather than `sat`/`unsat` when checking combined metric bounds, or whether the declared bounds entail a claim. Simplify the constraint expressions or supply tighter bounds so the solver can decide.",
    },
    CodeEntry {
        code: "I200",
        title: "Stale inferred entities",
        owner: "core",
        explanation: "A source file has changed on disk since it was last analyzed by inference, so the entities inferred from it may no longer be accurate. Re-analyze the file to refresh its inferred entities.",
    },
    CodeEntry {
        code: "I202",
        title: "High inference density",
        owner: "core",
        explanation: "A source file produced an unusually high number of inferred entities relative to its line count, exceeding the configured density threshold. Review the file for over-eager inference, or adjust the density threshold if that density is expected.",
    },
    CodeEntry {
        code: "I999",
        title: "Diagnostic output truncated",
        owner: "core",
        explanation: "More diagnostics were produced than the 100-diagnostic display limit, so only the first batch is shown. Fix the listed diagnostics and rerun the compiler to see the rest.",
    },
    CodeEntry {
        code: "W001",
        title: "Behavior implements no feature",
        owner: "@specforge/software",
        explanation: "A `behavior` entity has no outgoing edge to any `feature`, meaning it doesn't implement anything declared. Add an `implements` reference to a feature, or remove the behavior if it's unused.",
    },
    CodeEntry {
        code: "W002",
        title: "Unreferenced type",
        owner: "@specforge/software",
        explanation: "A `type` entity has no incoming references from any `behavior`, `port`, or other `type`. Reference the type where it's used, or remove it if it's dead.",
    },
    CodeEntry {
        code: "W003",
        title: "Unenforced invariant",
        owner: "@specforge/software",
        explanation: "An `invariant` entity has no incoming edges from any `behavior`, meaning nothing enforces it. Add an `enforces` reference from a behavior, or remove the invariant if it no longer applies.",
    },
    CodeEntry {
        code: "W004",
        title: "Untested testable entity",
        owner: "@specforge/testing",
        explanation: "A testable entity (`behavior`, `invariant`, `event`, `type`, or `port`) declares no `verify` obligations and no Gherkin scenario, so it has no test linkage. Add a `verify` block or a Gherkin scenario covering it.",
    },
    CodeEntry {
        code: "W005",
        title: "Unreferenced port",
        owner: "@specforge/software",
        explanation: "A `port` entity is not referenced by any `behavior`. Reference the port from a behavior that uses it, or remove it if it's unused.",
    },
    CodeEntry {
        code: "W006",
        title: "Behavior missing category",
        owner: "@specforge/software",
        explanation: "A `behavior` entity has no `category` field, which agents rely on for task routing. Add a `category` field to the behavior.",
    },
    CodeEntry {
        code: "W007",
        title: "Event never produced",
        owner: "@specforge/software",
        explanation: "An `event` entity is not produced by any `behavior`. Add a `produces` reference from the behavior that emits it, or remove the event if it's unused.",
    },
    CodeEntry {
        code: "W008",
        title: "Unimplemented feature",
        owner: "@specforge/software",
        explanation: "A `feature` entity has no incoming edge from any `behavior`, meaning nothing implements it. Add a behavior that implements the feature, or remove it if it's not planned.",
    },
    CodeEntry {
        code: "W009",
        title: "Disallowed verify kind",
        owner: "@specforge/testing",
        explanation: "An entity uses a `verify` kind (for example `unit`, `contract`, `integration`) that isn't in the allowed set for its entity kind. Use one of the verify kinds listed as allowed in the diagnostic.",
    },
    CodeEntry {
        code: "W010",
        title: "Unknown field annotation",
        owner: "@specforge/software",
        explanation: "A `type` field carries an annotation that isn't recognized by the compiler. Remove the annotation or correct its spelling.",
    },
    CodeEntry {
        code: "W011",
        title: "Edge references missing node",
        owner: "core",
        explanation: "An edge was about to be added between two entities, but one or both endpoints don't exist in the graph, so the edge was dropped. Check the referenced entity IDs for typos or missing definitions.",
    },
    CodeEntry {
        code: "W012",
        title: "Unreferenced ref entity",
        owner: "core",
        explanation: "A `ref` entity has no incoming edges, meaning nothing in the project references it. Reference the `ref` from another entity, or remove it if it's unused.",
    },
    CodeEntry {
        code: "W017",
        title: "Testable kind lacks verify support",
        owner: "core",
        explanation: "An extension registers an entity kind as `testable` but does not set `supportsVerify: true`, so verify statements can't be declared on it. Set `supportsVerify: true` in the extension manifest.",
    },
    CodeEntry {
        code: "W018",
        title: "Duplicate edge type",
        owner: "core",
        explanation: "Two extensions register an edge type with the same label; the first-registered extension's definition wins and the later one is ignored. Rename one of the conflicting edge types to avoid the collision.",
    },
    CodeEntry {
        code: "W019",
        title: "Unknown field type",
        owner: "core",
        explanation: "An extension manifest declares a field whose `field_type` value the compiler doesn't recognize (it must be one of `string`, `integer`, `bool`, `enum`, `string_list`, `reference`, `reference_list`, or `block`). Correct the field's `field_type` in the manifest.",
    },
    CodeEntry {
        code: "W020",
        title: "Unrecognized field",
        owner: "core",
        explanation: "An entity sets a field that isn't declared for its kind by any installed extension. Remove the field, fix a typo in its name, or install the extension that declares it.",
    },
    CodeEntry {
        code: "W021",
        title: "Undeclared target kind or edge label",
        owner: "core",
        explanation: "A field or edge type references a `target_kind` or edge label that isn't declared — either in the extension's own manifest when it declares no peer dependencies, or in the compiler's global kind/edge registry once all extensions are loaded. Declare the missing kind or edge label, or add the appropriate peer dependency.",
    },
    CodeEntry {
        code: "W023",
        title: "Duplicate validation rule code",
        owner: "core",
        explanation: "Two extensions register a validation rule using the same diagnostic code. Change one extension's rule to use a unique code.",
    },
    CodeEntry {
        code: "W024",
        title: "Contribution targets an unregistered kind",
        owner: "core",
        explanation: "An extension's grammar or body-parser contribution targets an entity kind that no installed extension registers, so it is ignored. Register the target kind (or install the extension that does) before contributing to it.",
    },
    CodeEntry {
        code: "W025",
        title: "Inaccessible contribution asset",
        owner: "core",
        explanation: "An extension's grammar contribution points at a `.wasm` file that can't be found, or its body-parser contribution references an export that doesn't exist in the extension's wasm module. Fix the path or export name in the manifest.",
    },
    CodeEntry {
        code: "W026",
        title: "Invalid verify kind",
        owner: "core",
        explanation: "A `verify` statement uses a kind that no installed extension has registered, or uses a kind that is registered but not allowed for that entity's kind. Use one of the verify kinds listed as allowed in the diagnostic.",
    },
    CodeEntry {
        code: "W027",
        title: "Re-export binding not found",
        owner: "core",
        explanation: "A selective `pub use { A, B } from \"target\"` re-export names a binding that isn't actually exported by the target module. Correct the binding name or remove it from the re-export list.",
    },
    CodeEntry {
        code: "W028",
        title: "Extension memory ceiling exceeded",
        owner: "core",
        explanation: "The combined `max_memory_mb` declared across all installed extensions' sandbox policies exceeds the configured total memory ceiling. Reduce `max_memory_mb` in one or more extension sandbox policies.",
    },
    CodeEntry {
        code: "W029",
        title: "Event never consumed",
        owner: "@specforge/formal",
        explanation: "An `event` entity is produced by one or more behaviors but has no consumer, meaning nothing reacts to it. Add a behavior that consumes the event, or remove the unused production.",
    },
    CodeEntry {
        code: "W030",
        title: "Abstract behavior unrefined",
        owner: "@specforge/formal",
        explanation: "A `behavior` marked `abstract true` has no concrete refinement — no behavior declares `refines` against it, and no `refinement` entity names it as the abstract entity. Add a concrete behavior with `refines`, or a `refinement` entity naming this behavior as the `abstract_entity`.",
    },
    CodeEntry {
        code: "W031",
        title: "Refinement chain too deep",
        owner: "@specforge/formal",
        explanation: "A behavior sits in a refinement chain deeper than the maximum allowed depth of 4 layers. Split the refinement chain, or collapse intermediate abstraction layers.",
    },
    CodeEntry {
        code: "W035",
        title: "Undischarged coverage items",
        owner: "@specforge/formal",
        explanation: "One or more coverage-tracking items are not covered by any recorded test. Annotate a test with the entity it proves and run `specforge collect` so its result is recorded.",
    },
    CodeEntry {
        code: "W041",
        title: "Orphan feature",
        owner: "@specforge/product",
        explanation: "A `feature` entity has no incoming edges, meaning no `journey`, `milestone`, or `module` references it. Link it from at least one referencing entity, or remove it if it is no longer needed.",
    },
    CodeEntry {
        code: "W042",
        title: "Orphan journey",
        owner: "@specforge/product",
        explanation: "A `journey` entity has no incoming edges, meaning no `deliverable` references it. Reference the journey from a deliverable's `journeys` field, or remove it if it is unused.",
    },
    CodeEntry {
        code: "W044",
        title: "Orphan module",
        owner: "@specforge/product",
        explanation: "A `module` entity has no incoming edges, meaning no `deliverable` or `milestone` references it. Reference the module from a deliverable or milestone, or remove it if it is unused.",
    },
    CodeEntry {
        code: "W045",
        title: "Feature dependency cycle",
        owner: "@specforge/product",
        explanation: "Two or more `feature` entities form a cycle through their `depends_on` edges. Break the cycle by removing or restructuring one of the `depends_on` references.",
    },
    CodeEntry {
        code: "W049",
        title: "Empty milestone",
        owner: "@specforge/product",
        explanation: "A `milestone` entity has neither `features` nor `modules` listed, so it may be empty. Add at least one `features` or `modules` reference, or remove the milestone.",
    },
    CodeEntry {
        code: "W050",
        title: "Invalid decision status",
        owner: "@specforge/governance",
        explanation: "A `decision` entity's `status` field is not one of the recognized values (`proposed`, `accepted`, `deprecated`, `superseded`). Set `status` to one of these values.",
    },
    CodeEntry {
        code: "W051",
        title: "Invalid failure mode severity",
        owner: "@specforge/governance",
        explanation: "A `failure_mode` entity's `severity` or `post_severity` field is not one of the recognized values (`critical`, `high`, `medium`, `low`). Set the field to one of these values.",
    },
    CodeEntry {
        code: "W052",
        title: "Invalid failure mode occurrence",
        owner: "@specforge/governance",
        explanation: "A `failure_mode` entity's `occurrence` or `post_occurrence` field is not one of the recognized values (`certain`, `likely`, `occasional`, `unlikely`, `rare`). Set the field to one of these values.",
    },
    CodeEntry {
        code: "W053",
        title: "Breaking schema change",
        owner: "core",
        explanation: "Comparing the graph protocol schema before and after a migration found a change classified as breaking (e.g. a removed or incompatibly altered field). Review the migration to ensure it preserves backward compatibility, or accept the break intentionally.",
    },
    CodeEntry {
        code: "W054",
        title: "Migration structural drift",
        owner: "core",
        explanation: "Comparing the entity graph before and after a migration found entities or edges that appeared, disappeared, or changed unexpectedly. Review the migration logic to ensure it preserves the entities and edges it did not intend to change.",
    },
    CodeEntry {
        code: "W057",
        title: "Missing milestone exit criteria",
        owner: "@specforge/product",
        explanation: "A `milestone` entity has `status: completed` but no `exit_criteria` field. Add an `exit_criteria` field describing how completion was verified.",
    },
    CodeEntry {
        code: "W060",
        title: "Cross-kind ID collision",
        owner: "core",
        explanation: "The same entity ID is declared with two different entity kinds, either within the same compilation pass or across different files. Entity IDs share one flat namespace regardless of kind, so rename one of the conflicting declarations; the first declaration encountered is retained and later ones are skipped.",
    },
    CodeEntry {
        code: "W061",
        title: "Reference cycle detected",
        owner: "core",
        explanation: "The resolved reference graph contains a cycle among entity references. Break the cycle by removing or inverting one of the references in the reported path.",
    },
    CodeEntry {
        code: "W062",
        title: "Malformed semver version",
        owner: "core",
        explanation: "An extension manifest declares a peer dependency range, a version, or a `host_api_version` that is not valid semver. Use a valid semver version (e.g. `1.0.0`) or range (e.g. `^1.0.0`, `~1.2.0`, `>=1.0.0`).",
    },
    CodeEntry {
        code: "W063",
        title: "Circular peer dependency",
        owner: "core",
        explanation: "Two or more installed extensions declare peer dependencies on each other, forming a cycle. Break the cycle by removing one of the peer dependency declarations.",
    },
    CodeEntry {
        code: "W077",
        title: "Invalid feature status",
        owner: "@specforge/product",
        explanation: "A `feature` entity's `status` field is not one of the recognized values (`proposed`, `accepted`, `in_progress`, `done`, `deferred`, `deprecated`). Set `status` to one of these values.",
    },
    CodeEntry {
        code: "W078",
        title: "Invalid priority value",
        owner: "@specforge/product",
        explanation: "A `feature`, `journey`, `milestone`, or `constraint` entity's `priority` field is not one of the recognized values (`critical`, `high`, `medium`, `low`). Set `priority` to one of these values.",
    },
    CodeEntry {
        code: "W079",
        title: "Invalid milestone status",
        owner: "@specforge/product",
        explanation: "A `milestone` entity's `status` field is not one of the recognized values (`planned`, `in_progress`, `completed`, `blocked`). Set `status` to one of these values.",
    },
    CodeEntry {
        code: "W080",
        title: "Invalid deliverable artifact type",
        owner: "@specforge/product",
        explanation: "A `deliverable` entity's `artifact_type` field is not one of the recognized values (e.g. `cli`, `service`, `library`, `web_app`, `mobile_app`, `api`, `extension`, `documentation`, `package`). Set `artifact_type` to one of these values.",
    },
    CodeEntry {
        code: "W083",
        title: "Invalid persona status",
        owner: "@specforge/product",
        explanation: "A `persona` entity's `status` field is not one of the recognized values (`active`, `deprecated`). Set `status` to one of these values.",
    },
    CodeEntry {
        code: "W084",
        title: "Invalid channel status",
        owner: "@specforge/product",
        explanation: "A `channel` entity's `status` field is not one of the recognized values (`active`, `deprecated`). Set `status` to one of these values.",
    },
    CodeEntry {
        code: "W085",
        title: "Invalid deliverable status",
        owner: "@specforge/product",
        explanation: "A `deliverable` entity's `status` field is not one of the recognized values (`draft`, `in_progress`, `shipped`, `deprecated`). Set `status` to one of these values.",
    },
    CodeEntry {
        code: "W092",
        title: "Release dependency cycle",
        owner: "@specforge/product",
        explanation: "Two or more `release` entities form a cycle through their `depends_on` edges. Break the cycle by removing or restructuring one of the dependency references.",
    },
    CodeEntry {
        code: "W093",
        title: "Invalid release version format",
        owner: "@specforge/product",
        explanation: "A `release` entity's `version` field does not match semver format (e.g. `1.0.0`). Set `version` to a valid semver string.",
    },
    CodeEntry {
        code: "W095",
        title: "Invalid feature effort",
        owner: "@specforge/product",
        explanation: "A `feature` entity's `effort` field is not one of the recognized values (`xs`, `s`, `m`, `l`, `xl`). Set `effort` to one of these values.",
    },
    CodeEntry {
        code: "W096",
        title: "Behavior requires without ensures",
        owner: "@specforge/formal",
        explanation: "A `behavior` entity declares a `requires` clause (an obligation on callers) but no `ensures` clause (a guarantee in return). Add an `ensures` clause describing what the behavior guarantees when its `requires` is satisfied.",
    },
    CodeEntry {
        code: "W098",
        title: "SMT solver unavailable",
        owner: "core",
        explanation: "The `z3` SMT solver could not be found on `PATH`, or failed to execute, so formal entailment and consistency checks during `--prove` were skipped. Install z3 (https://github.com/Z3Prover/z3) and ensure it is executable to enable these checks.",
    },
    CodeEntry {
        code: "W099",
        title: "Reference outside import graph",
        owner: "core",
        explanation: "An entity references another entity that resolves only through the global entity index, not through the referencing file's own declarations or its `use` imports. Add a `use` import that makes the dependency explicit, even though the reference still resolves.",
    },
    CodeEntry {
        code: "W110",
        title: "Refines non-abstract behavior",
        owner: "@specforge/formal",
        explanation: "A behavior's `refines` field names a target behavior that is not marked `abstract true`. Add `abstract true` to the target behavior, or point `refines` at a behavior that is actually abstract.",
    },
    CodeEntry {
        code: "W111",
        title: "Grammar conflict resolved by policy",
        owner: "core",
        explanation: "Two installed extensions register a custom body-parser grammar for the same entity kind. Depending on the configured conflict policy, the first or the most recently registered grammar wins; uninstall one of the conflicting extensions or configure a different policy if the outcome is wrong.",
    },
    CodeEntry {
        code: "W112",
        title: "Validation rule cannot fire",
        owner: "core",
        explanation: "An extension-declared validation rule cannot work as declared: its `check` kind is unrecognized, it is missing a field or constraint its check needs, its values list is empty, its `matches` regex does not compile, or its `wasm_function` is absent or failed a probe call. Fix or remove the rule in the extension's manifest.",
    },
    CodeEntry {
        code: "W113",
        title: "Circular file import",
        owner: "core",
        explanation: "Two or more `.spec` files import each other, forming a cycle in the import graph. Break the cycle by removing one of the `use` imports or extracting the shared entities into a separate file.",
    },
    CodeEntry {
        code: "W114",
        title: "Integrity check skipped",
        owner: "core",
        explanation: "A Wasm extension's integrity check was bypassed because the `--skip-verify` flag was passed. Remove `--skip-verify` to re-enable hash verification of the extension's `.wasm` binary.",
    },
    CodeEntry {
        code: "W115",
        title: "Invalid collector report",
        owner: "core",
        explanation: "A collector reported tests for an entity ID that no spec declares (usually a renamed entity or a typo in the test's annotation), or its `total`/`passed`/`failed`/`skipped` stats are inconsistent. `specforge collect` drops those results; fix the test annotation so it names a declared entity.",
    },
    CodeEntry {
        code: "W116",
        title: "Extension discovery failure",
        owner: "core",
        explanation: "While scanning an extensions directory, a `manifest.json` could not be read or read directory itself failed, or a manifest failed to parse as valid JSON matching the manifest schema. Fix the directory permissions or correct the malformed `manifest.json`; discovery skips the broken entry and continues with the rest.",
    },
    CodeEntry {
        code: "W117",
        title: "Invalid query extension pattern",
        owner: "core",
        explanation: "An extension's tree-sitter query extension pattern (for `highlights`, `locals`, or `injections`) is empty or contains null bytes. Provide a non-empty query pattern with no null bytes; the invalid pattern is skipped rather than loaded.",
    },
    CodeEntry {
        code: "W118",
        title: "Invalid provider configuration",
        owner: "core",
        explanation: "A `providers` entry in `specforge.json` is missing its `alias`/`name` or `scheme` field, or no installed extension contributes providers to back a configured provider. Add the missing field, or install an extension that contributes the provider.",
    },
    CodeEntry {
        code: "W119",
        title: "Partial install cleanup failed",
        owner: "core",
        explanation: "Rolling back a failed extension install could not remove the partially-created extension directory. Manually delete the leftover extension directory reported in the message.",
    },
    CodeEntry {
        code: "W120",
        title: "Invalid ref target",
        owner: "core",
        explanation: "A `ref` entity's target string is empty or contains control characters. Provide a clean, non-empty target such as an issue number or ticket key (e.g. `\"42\"`, `\"PROJ-123\"`).",
    },
    CodeEntry {
        code: "W121",
        title: "Invalid failure mode detection",
        owner: "@specforge/governance",
        explanation: "A `failure_mode` entity's `detection` or `post_detection` field is not one of the recognized values (`certain`, `likely`, `moderate`, `unlikely`, `undetectable`). Set the field to one of these values.",
    },
    CodeEntry {
        code: "W122",
        title: "Duplicate entity ID across files",
        owner: "core",
        explanation: "The same entity ID and kind are declared in more than one `.spec` file. Use unique entity IDs across files, or use imports to share a single definition instead of redeclaring it.",
    },
    CodeEntry {
        code: "W123",
        title: "Orphan property",
        owner: "@specforge/formal",
        explanation: "A `property` entity is not referenced by any `behavior`, so it may be unused. Reference the property from a behavior's `verify` block, or remove it if it is no longer needed.",
    },
    CodeEntry {
        code: "W125",
        title: "Invalid property type",
        owner: "@specforge/formal",
        explanation: "A `property` entity's `property_type` field is not one of the recognized values (`safety`, `liveness`, `fairness`). Set `property_type` to one of these values.",
    },
    CodeEntry {
        code: "W126",
        title: "Orphan axiom",
        owner: "@specforge/formal",
        explanation: "An `axiom` entity is not referenced by any other entity, so it may be unused. Reference the axiom from a relevant entity, or remove it if it is no longer needed.",
    },
    CodeEntry {
        code: "W128",
        title: "Orphan protocol",
        owner: "@specforge/formal",
        explanation: "A `protocol` entity is not referenced by any `event`, so it may be unused. Reference the protocol from an event, or remove it if it is no longer needed.",
    },
    CodeEntry {
        code: "W131",
        title: "Orphan refinement",
        owner: "@specforge/formal",
        explanation: "A `refinement` entity is not referenced by anything, so it may be orphaned. Reference the refinement from the entity it refines, or remove it if it is no longer needed.",
    },
    CodeEntry {
        code: "W134",
        title: "Orphan process",
        owner: "@specforge/formal",
        explanation: "A `process` entity is not referenced by any other entity, so it may be unused. Reference the process from a relevant entity, or remove it if it is no longer needed.",
    },
];

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{BTreeMap, BTreeSet};
    use std::path::{Path, PathBuf};

    fn workspace_root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    /// Third-party extensions own E900–E998 / W900–W998 / I900–I998.
    fn is_third_party(code: &str) -> bool {
        let n: u32 = code[1..].parse().unwrap_or(0);
        (900..=998).contains(&n)
    }

    /// C4-10: docs/diagnostics.md is generated from [`CATALOG`] and linked
    /// from LSP `codeDescription.href`. Set `SPECFORGE_BLESS=1` to rewrite it.
    #[test]
    fn explain_docs_sync() {
        let path = workspace_root().join("docs/diagnostics.md");
        let rendered = render_docs();
        if std::env::var("SPECFORGE_BLESS").as_deref() == Ok("1") {
            std::fs::write(&path, &rendered).expect("write docs/diagnostics.md");
            return;
        }
        let on_disk = std::fs::read_to_string(&path).unwrap_or_default();
        assert!(
            on_disk == rendered,
            "docs/diagnostics.md is out of date with the explain catalog; regenerate it with \
             `SPECFORGE_BLESS=1 cargo test -p specforge-cli explain_docs_sync`"
        );
    }

    #[test]
    fn catalog_is_sorted_unique_and_well_formed() {
        let mut problems = Vec::new();
        for pair in CATALOG.windows(2) {
            if pair[0].code >= pair[1].code {
                problems.push(format!(
                    "{} must come before {} (CATALOG is sorted by code, without duplicates)",
                    pair[1].code, pair[0].code
                ));
            }
        }
        for entry in CATALOG {
            let b = entry.code.as_bytes();
            let shaped = b.len() == 4
                && matches!(b[0], b'E' | b'W' | b'I')
                && b[1..].iter().all(u8::is_ascii_digit);
            if !shaped {
                problems.push(format!("{}: not of the form E###/W###/I###", entry.code));
            } else if is_third_party(entry.code) {
                problems.push(format!(
                    "{}: the 900-998 range is reserved for third-party extensions",
                    entry.code
                ));
            }
            if entry.title.is_empty() || entry.explanation.is_empty() {
                problems.push(format!("{}: empty title or explanation", entry.code));
            }
            if entry.owner != "core" && !entry.owner.starts_with("@specforge/") {
                problems.push(format!(
                    "{}: owner `{}` must be `core` or `@specforge/<name>`",
                    entry.code, entry.owner
                ));
            }
        }
        assert!(
            problems.is_empty(),
            "CATALOG problems:\n  {}",
            problems.join("\n  ")
        );
    }

    #[test]
    fn lookup_is_case_insensitive() {
        assert_eq!(lookup("e001").map(|e| e.code), Some("E001"));
        assert!(lookup("E900").is_none());
    }

    /// One code literal found in production source.
    struct Site {
        code: String,
        owner: String,
        location: String,
    }

    /// Replace test-only items (`#[cfg(test)]` / `#[test]` and the item that
    /// follows, brace-matched) with blank lines so line numbers survive.
    fn strip_test_items(src: &str) -> Vec<&str> {
        let lines: Vec<&str> = src.lines().collect();
        let mut out = Vec::with_capacity(lines.len());
        let mut i = 0;
        while i < lines.len() {
            let trimmed = lines[i].trim();
            if trimmed.starts_with("#[cfg(test)]") || trimmed == "#[test]" {
                let mut j = i + 1;
                let mut depth: i64 = 0;
                let mut started = false;
                while j < lines.len() {
                    for ch in lines[j].chars() {
                        match ch {
                            '{' => {
                                depth += 1;
                                started = true;
                            }
                            '}' => depth -= 1,
                            _ => {}
                        }
                    }
                    if (started && depth <= 0) || (!started && lines[j].trim_end().ends_with(';')) {
                        break;
                    }
                    j += 1;
                }
                let end = j.min(lines.len() - 1);
                out.extend(std::iter::repeat_n("", end - i + 1));
                i = end + 1;
                continue;
            }
            out.push(lines[i]);
            i += 1;
        }
        out
    }

    /// Find `"E###"`-style literals on a line.
    fn code_literals(line: &str) -> Vec<&str> {
        let b = line.as_bytes();
        let mut found = Vec::new();
        for i in 0..b.len().saturating_sub(5) {
            if b[i] == b'"'
                && matches!(b[i + 1], b'E' | b'W' | b'I')
                && b[i + 2..i + 5].iter().all(u8::is_ascii_digit)
                && b[i + 5] == b'"'
            {
                found.push(&line[i + 1..i + 5]);
            }
        }
        found
    }

    fn walk(dir: &Path, files: &mut Vec<PathBuf>) {
        const SKIP: &[&str] = &[
            "tests",
            "target",
            "node_modules",
            ".git",
            "benches",
            "examples",
        ];
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().into_owned();
            if path.is_dir() {
                if !SKIP.contains(&name.as_str()) {
                    walk(&path, files);
                }
            } else {
                files.push(path);
            }
        }
    }

    /// Every code literal in production (non-test) source, with the owner
    /// implied by where it lives.
    fn emitted_sites() -> Vec<Site> {
        let root = workspace_root();
        let mut files = Vec::new();
        for top in ["crates", "xtask", "integrations", "extensions"] {
            walk(&root.join(top), &mut files);
        }
        let mut sites = Vec::new();
        for path in files {
            let rel = path.strip_prefix(&root).unwrap_or(&path);
            let rel_str = rel.to_string_lossy().replace('\\', "/");
            let parts: Vec<&str> = rel_str.split('/').collect();
            let file_name = *parts.last().unwrap_or(&"");
            let is_rust = file_name.ends_with(".rs");
            let owner = if parts[0] == "extensions" {
                if parts.len() < 3 || parts[2] != "src" {
                    continue;
                }
                if !is_rust && file_name != "describe_validation_rules.json" {
                    continue;
                }
                format!("@specforge/{}", parts[1])
            } else {
                if !is_rust || !parts.contains(&"src") {
                    continue;
                }
                "core".to_string()
            };
            if file_name == "tests.rs"
                || file_name.ends_with("_tests.rs")
                || rel_str == "crates/specforge-cli/src/explain.rs"
            {
                continue;
            }
            let Ok(src) = std::fs::read_to_string(&path) else {
                continue;
            };
            let lines = if is_rust {
                strip_test_items(&src)
            } else {
                src.lines().collect()
            };
            for (index, line) in lines.iter().enumerate() {
                for code in code_literals(line) {
                    if is_third_party(code) {
                        continue;
                    }
                    sites.push(Site {
                        code: code.to_string(),
                        owner: owner.clone(),
                        location: format!("{rel_str}:{}", index + 1),
                    });
                }
            }
        }
        sites
    }

    /// The catalog is enforced: every emitted code is registered under the
    /// right owner, and every registered code is emitted somewhere.
    #[test]
    fn explain_catalog_matches_emitted_codes() {
        let catalog: BTreeMap<&str, &CodeEntry> = CATALOG.iter().map(|e| (e.code, e)).collect();
        let sites = emitted_sites();
        assert!(
            !sites.is_empty(),
            "scanner found no diagnostic codes; is the workspace root right?"
        );

        let mut problems = Vec::new();
        let mut emitted = BTreeSet::new();
        for site in &sites {
            emitted.insert(site.code.as_str());
            match catalog.get(site.code.as_str()) {
                None => problems.push(format!(
                    "{} emitted at {} is not in explain.rs CATALOG; add a CodeEntry with owner `{}`",
                    site.code, site.location, site.owner
                )),
                Some(entry) if entry.owner != site.owner => problems.push(format!(
                    "{} emitted at {} belongs to `{}`, but CATALOG says owner `{}`; each code has one \
                     owner, so pick a free code for this emitter or fix the entry",
                    site.code, site.location, site.owner, entry.owner
                )),
                Some(_) => {}
            }
        }
        for code in catalog.keys() {
            if !emitted.contains(code) {
                problems.push(format!(
                    "{code} is in CATALOG but nothing emits it any more; remove the entry"
                ));
            }
        }
        assert!(
            problems.is_empty(),
            "diagnostic catalog out of sync with emitters ({} problems):\n  {}\n\nThen regenerate \
             docs with `SPECFORGE_BLESS=1 cargo test -p specforge-cli explain_docs_sync`.",
            problems.len(),
            problems.join("\n  ")
        );
    }
}
