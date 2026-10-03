//! Diagnostic code registry: the single canonical catalog of every code the
//! compiler, the CLI, and the first-party extensions emit.
//!
//! `specforge explain <CODE>` and the MCP `specforge.explain` tool print an
//! entry, MCP diagnostics and the LSP carry its title and docs link, doctor
//! quotes its explanation, and `docs/diagnostics.md` is generated from
//! [`CATALOG`]. The tests at the bottom of this file fail when an emitted
//! code is missing from the catalog, is attributed to the wrong owner, or
//! when the catalog lists a code nothing emits.

/// One diagnostic code and what it means.
#[derive(Debug, Clone, Copy)]
pub struct CodeEntry {
    /// `E###` (error), `W###` (warning), `I###` (info), `A###` (an
    /// `analyze` pass finding, whose severity the pass sets), or the
    /// registry client's `R###` / `R-<AREA>-###`.
    pub code: &'static str,
    /// Short human-readable name.
    pub title: &'static str,
    /// `"core"` for the compiler/CLI, or the emitting extension
    /// (`"@specforge/formal"`, ...).
    pub owner: &'static str,
    /// The severity every emit site uses. An `E`/`W`/`I` prefix states it;
    /// an `A###` finding's severity is set by its pass.
    pub level: Level,
    /// What triggers the diagnostic and how to fix it.
    pub explanation: &'static str,
}

/// A catalogued code's severity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    Error,
    Warning,
    Info,
    /// `A###` analyze findings: the pass picks the severity per finding.
    SetByPass,
}

impl Level {
    /// How `specforge explain` and `docs/diagnostics.md` name the level.
    pub fn describe(self) -> &'static str {
        match self {
            Level::Error => "error",
            Level::Warning => "warning",
            Level::Info => "info",
            Level::SetByPass => "set by the analyze pass",
        }
    }
}

/// Width used when wrapping explanations for the terminal and the docs page.
pub const WRAP_WIDTH: usize = 80;

/// The page on the canonical repository (the workspace's Cargo
/// `repository`, ADR 0004 D6-b) that documents every catalogued code.
pub const DOCS_URL: &str = concat!(
    env!("CARGO_PKG_REPOSITORY"),
    "/blob/main/docs/diagnostics.md"
);

/// The docs link for `code`: its section of [`DOCS_URL`], or `None` when
/// the catalog has no entry for it (a third-party code, or anything
/// outside the catalog).
pub fn docs_href(code: &str) -> Option<String> {
    lookup(code).map(|entry| format!("{DOCS_URL}#{}", entry.code.to_lowercase()))
}

/// Look up a code (case-insensitive).
pub fn lookup(code: &str) -> Option<&'static CodeEntry> {
    let normalized = code.to_uppercase();
    CATALOG
        .binary_search_by(|entry| entry.code.cmp(normalized.as_str()))
        .ok()
        .map(|index| &CATALOG[index])
}

/// Greedy word wrap; never splits a word.
pub fn wrap(text: &str, width: usize) -> String {
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
pub fn render_docs() -> String {
    let mut out = String::from(DOCS_HEADER);
    for entry in CATALOG {
        out.push_str(&format!(
            "\n## {code}\n\n```\n{code}: {title}\n\n{body}\n\nOwner: {owner}\nLevel: {level}\n```\n",
            code = entry.code,
            title = entry.title,
            body = wrap(entry.explanation, WRAP_WIDTH),
            owner = entry.owner,
            level = entry.level.describe(),
        ));
    }
    out.push_str(
        "\n## Retired codes\n\nThese codes are no longer emitted, and are never reused for another \
         meaning.\n\n| Code | Replaced by |\n|------|-------------|\n",
    );
    for (old, new) in RETIRED {
        let new = match new {
            Some(code) => format!("[{code}](#{})", code.to_lowercase()),
            None => "(nothing)".to_string(),
        };
        out.push_str(&format!("| {old} | {new} |\n"));
    }
    out
}

const DOCS_HEADER: &str = "# SpecForge Diagnostic Codes

<!-- Generated file: do not edit by hand. -->

This page is generated from the catalog in `crates/specforge-diagnostics/src/lib.rs`,
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
SPECFORGE_BLESS=1 cargo test -p specforge-diagnostics explain_docs_sync
```
";

/// Every diagnostic code emitted today, sorted by code.
pub const CATALOG: &[CodeEntry] = &[
    CodeEntry {
        code: "A001",
        title: "Testable entity without obligations",
        owner: "@specforge/testing",
        level: Level::SetByPass,
        explanation: "`specforge analyze coverage` found a testable entity (one whose kind an extension declares testable) that declares no `verify` obligations, so nothing states what a test must prove about it. Add `verify unit \"...\"` (or another obligation kind) statements. What W004 exempts (union types, abstract entities, governance kinds) is not reported.",
    },
    CodeEntry {
        code: "A002",
        title: "Invariant without obligations",
        owner: "@specforge/testing",
        level: Level::SetByPass,
        explanation: "An invariant declares no `verify` obligations. It is an error when the invariant's `risk` is `high`, a warning otherwise. Add a `verify property` or `verify unit` statement stating how the guarantee is checked.",
    },
    CodeEntry {
        code: "A010",
        title: "Entity without contract obligations",
        owner: "core",
        level: Level::SetByPass,
        explanation: "`specforge analyze contracts` found an entity whose kind registers contract reference fields (`requires`, `ensures`, `maintains`, ...) that declares none of them, so no invariant constrains it. Add the references, or ignore this info-level finding for entities that need none.",
    },
    CodeEntry {
        code: "A014",
        title: "Failing tests",
        owner: "@specforge/testing",
        level: Level::SetByPass,
        explanation: "The recorded test results (`specforge-report.json`, written by `specforge collect`) include failing tests for this entity, so what it promises is not proven. Fix the code or the test and run `specforge collect` again.",
    },
    CodeEntry {
        code: "A015",
        title: "Unproven obligations",
        owner: "@specforge/testing",
        level: Level::SetByPass,
        explanation: "With recorded test results, some of an entity's `verify` obligations are named by no passing test. A test proves an obligation by naming its exact text: `verify = \"...\"` in `#[specforge_test(...)]`, or in a vitest test's `meta.specforge`. Link a test to each listed obligation, or write the missing tests.",
    },
    CodeEntry {
        code: "A016",
        title: "Test names an undeclared obligation",
        owner: "@specforge/testing",
        level: Level::SetByPass,
        explanation: "A recorded test names a `verify` obligation that its entity does not declare, usually a typo or an obligation reworded in the spec. The test proves nothing until its text matches the spec's statement exactly; fix whichever side is wrong.",
    },
    CodeEntry {
        code: "E001",
        title: "Parse error",
        owner: "core",
        level: Level::Error,
        explanation: "A `.spec` file could not be parsed — invalid syntax such as a missing brace, unclosed string, or malformed field at the reported location; the same code also covers an internal reparse failure in the language server, and the formatter's parser producing no syntax tree at all. Fix the syntax at the reported span and re-save.",
    },
    CodeEntry {
        code: "E002",
        title: "Duplicate entity ID",
        owner: "core",
        level: Level::Error,
        explanation: "Two entities of the same kind declare the same ID; the diagnostic points at the duplicate and its message names where the ID was first declared (file:line:col). Rename one of the entities so each ID is unique within its kind.",
    },
    CodeEntry {
        code: "E003",
        title: "Unresolved reference",
        owner: "core",
        level: Level::Error,
        explanation: "A reference field names an entity ID that doesn't resolve to any declared entity. Fix the typo or add the missing entity; a `did you mean` suggestion is included when a close match exists.",
    },
    CodeEntry {
        code: "E004",
        title: "Unknown type in port method",
        owner: "@specforge/software",
        level: Level::Error,
        explanation: "A `port` entity's method field references a `type` ID that isn't declared anywhere in the spec. Declare the missing `type` entity or fix the reference to point at an existing one.",
    },
    CodeEntry {
        code: "E006",
        title: "Missing required field",
        owner: "core",
        level: Level::Error,
        explanation: "An entity is missing a field marked `required: true` for its kind in the field registry. Add the missing field to the entity.",
    },
    CodeEntry {
        code: "E007",
        title: "Module dependency cycle",
        owner: "@specforge/product",
        level: Level::Error,
        explanation: "The `depends_on` edges between `module` entities form a cycle. Break the cycle by removing or inverting one of the dependencies.",
    },
    CodeEntry {
        code: "E010",
        title: "Invalid milestone behavior range",
        owner: "@specforge/software",
        level: Level::Error,
        explanation: "A `milestone` entity's `behaviors` range field is malformed (the reported reason explains what's wrong, e.g. bad syntax or start after end). Correct the range to a valid form.",
    },
    CodeEntry {
        code: "E013",
        title: "Reserved entity ID",
        owner: "core",
        level: Level::Error,
        explanation: "An entity's ID is a reserved word — a structural DSL keyword (`spec`, `ref`, `use`, `define`) or an entity kind keyword contributed by an installed extension. Rename the entity, e.g. by appending a suffix like `_rule` or `_spec`.",
    },
    CodeEntry {
        code: "E014",
        title: "Entity ID length violation",
        owner: "core",
        level: Level::Error,
        explanation: "An entity ID is outside the 2-60 character identifier contract. Pick a descriptive identifier within that length.",
    },
    CodeEntry {
        code: "E015",
        title: "Milestone dependency cycle",
        owner: "@specforge/product",
        level: Level::Error,
        explanation: "The `depends_on` edges between `milestone` entities form a cycle. Break the cycle by removing or inverting one of the dependencies.",
    },
    CodeEntry {
        code: "E016",
        title: "Referenced file does not exist",
        owner: "core",
        level: Level::Error,
        explanation: "A file-reference field on an entity points at a path that doesn't exist under the spec root. Fix the path, or create the missing file; a similarly-named file is suggested when one is found.",
    },
    CodeEntry {
        code: "E017",
        title: "Entity enhancement conflict",
        owner: "core",
        level: Level::Error,
        explanation: "Two installed extensions both declare an entity-enhancement field with the same name on the same target entity kind, and no explicit override resolves it. Rename one extension's field, or add an override for that kind/field in `specforge.json`.",
    },
    CodeEntry {
        code: "E018",
        title: "Grammar contribution conflict",
        owner: "core",
        level: Level::Error,
        explanation: "Two extensions both contribute a tree-sitter grammar for the same entity kind. Only one extension may own an entity kind's grammar — uninstall one of the conflicting extensions or set a grammar conflict policy in the compiler config.",
    },
    CodeEntry {
        code: "E019",
        title: "Unsupported format version",
        owner: "core",
        level: Level::Error,
        explanation: "A `.spec` file's `// specforge-format: MAJOR.MINOR` header declares a version newer than this build supports, or the header itself doesn't parse; `specforge migrate --target-version` reports the same for a target it can't parse or doesn't support. Lower the declared version or upgrade SpecForge.",
    },
    CodeEntry {
        code: "E020",
        title: "Missing Wasm export",
        owner: "core",
        level: Level::Error,
        explanation: "An extension's manifest declares a contribution (validator, renderer, parser, collector, grammar, or surface command/tool) whose required Wasm export function isn't present in the compiled module. Add the matching `#[export_name = \"...\"]` export to the extension's Wasm binary.",
    },
    CodeEntry {
        code: "E022",
        title: "Reference targets wrong kind",
        owner: "core",
        level: Level::Error,
        explanation: "A reference field is declared to only accept entities of a specific kind, but the target ID resolves to an entity of a different kind. Point the field at an entity of the expected kind.",
    },
    CodeEntry {
        code: "E023",
        title: "Entity kind conflicts with keyword",
        owner: "core",
        level: Level::Error,
        explanation: "An extension declares an entity kind keyword that collides with a structural DSL keyword (`spec`, `ref`, `use`, `define`). Choose a different keyword for the entity kind.",
    },
    CodeEntry {
        code: "E024",
        title: "Unknown entity kind",
        owner: "core",
        level: Level::Error,
        explanation: "An entity block uses a kind keyword that neither the core grammar nor any installed extension declares. Install the extension that provides the keyword, or fix a typo in the kind name.",
    },
    CodeEntry {
        code: "E025",
        title: "Import resolution failed",
        owner: "core",
        level: Level::Error,
        explanation: "A `use` import's target either can't be read from disk or doesn't resolve to any known `.spec` file. Fix the import path; a `did you mean` suggestion is included when a close match exists.",
    },
    CodeEntry {
        code: "E026",
        title: "Entity kind registration conflict",
        owner: "core",
        level: Level::Error,
        explanation: "Two extensions register the same entity kind keyword; the first registration wins and the later one is rejected. Rename the conflicting kind keyword.",
    },
    CodeEntry {
        code: "E027",
        title: "Unsatisfiable peer dependency",
        owner: "core",
        level: Level::Error,
        explanation: "An extension's required peer dependency can't be satisfied: it isn't installed, the installed version doesn't match the required range, peer dependencies form a cycle, an uninstall would remove an extension others still require, or an upgrade would break a peer's requirement. Install or upgrade the named peer, or use `--force` where the command supports it.",
    },
    CodeEntry {
        code: "E028",
        title: "Extension load or execution failure",
        owner: "core",
        level: Level::Error,
        explanation: "An extension's Wasm module failed somewhere in its lifecycle — the binary is missing or unreadable, failed to load, trapped while running `initialize`/`validate`/a check-phase compiler pass/a collector/body parser/surface command/MCP tool or resource, returned output that isn't valid JSON, a grammar cache path couldn't be written, or the extension's declared host API version isn't supported. Check the extension's logs or report the trap to its author, and confirm the extension is installed and up to date.",
    },
    CodeEntry {
        code: "E030",
        title: "Invalid extension manifest",
        owner: "core",
        level: Level::Error,
        explanation: "An extension's manifest is unreadable, isn't valid JSON, or fails schema validation — a wrong `manifestVersion`, a missing `name`/`version`/`wasmPath`, an empty grammar/body-parser/analyzer contribution field, or a sandbox policy that allowlists a code file extension for output. `specforge publish` reports it for a `manifest.json` that doesn't parse. Fix the manifest according to the reported detail.",
    },
    CodeEntry {
        code: "E031",
        title: "Refinement drops ensures condition",
        owner: "@specforge/formal",
        level: Level::Error,
        explanation: "A behavior that `refines` an `abstract` behavior drops one or more of the abstract behavior's `ensures` conditions. A refinement may only strengthen its abstraction's postconditions, never weaken them — restore or strengthen the missing `ensures` condition(s) in the concrete behavior.",
    },
    CodeEntry {
        code: "E032",
        title: "Extension install or uninstall failed",
        owner: "core",
        level: Level::Error,
        explanation: "An install or uninstall step failed: the downloaded `.wasm` binary's SHA-256 hash didn't match the expected value (possible tampering or a bad download), or a filesystem step — creating the temp directory, writing the binary, finalizing the install, or removing the extension directory on uninstall — failed. Re-download the extension or check filesystem permissions.",
    },
    CodeEntry {
        code: "E033",
        title: "Lock file error",
        owner: "core",
        level: Level::Error,
        explanation: "`specforge.lock` couldn't be serialized, written, read, or parsed, or the hash it records for an installed extension no longer matches the binary on disk; `specforge update` reports it when there is no lock file to update. Delete the lock file and reinstall extensions, reinstall the specific extension whose binary changed, or run `specforge add` first.",
    },
    CodeEntry {
        code: "E035",
        title: "Reserved or invalid entity kind name",
        owner: "core",
        level: Level::Error,
        explanation: "An extension-declared entity kind name is a reserved structural keyword, doesn't match the identifier pattern `[a-z][a-z0-9_]{1,59}`, or is already reserved by another installed extension. Choose a different, valid entity kind name.",
    },
    CodeEntry {
        code: "E039",
        title: "Duplicate surface contribution",
        owner: "core",
        level: Level::Error,
        explanation: "Two extensions register the same CLI surface command ID, the same MCP tool name, or the same MCP resource name. Rename one extension's contribution so the identifier is unique across installed extensions.",
    },
    CodeEntry {
        code: "E040",
        title: "Missing extension project file",
        owner: "core",
        level: Level::Error,
        explanation: "`specforge extension build`, `extension validate` or `publish` was run against a directory that's missing its `Cargo.toml` or `manifest.json`, or (`publish`) the Wasm binary the manifest's `wasmPath` names, or one of those files couldn't be read. Run the command from a scaffolded extension project, build the binary, or create the missing file.",
    },
    CodeEntry {
        code: "E041",
        title: "Refinement chain cycle",
        owner: "@specforge/formal",
        level: Level::Error,
        explanation: "The `refines` layering graph between behaviors contains a cycle. Break the cycle by removing or redirecting one of the `refines` edges.",
    },
    CodeEntry {
        code: "E042",
        title: "Process composition cycle",
        owner: "@specforge/formal",
        level: Level::Error,
        explanation: "A `process` entity composes, transitively, with itself through its composition edges. Remove or redirect one of the composition steps to break the cycle.",
    },
    CodeEntry {
        code: "E045",
        title: "Invalid test report",
        owner: "core",
        level: Level::Error,
        explanation: "`specforge collect` couldn't get a test report: the runner's command couldn't be started, it finished without writing a report at the collector's declared location (often because the tests didn't build), `--no-run` found no existing report, or a report file couldn't be read. Check the runner's output above the error and the report path. `specforge analyze` and the MCP coverage, inspect, query and analyze tools report the same code when `specforge-report.json` (or `--test-results`) exists but can't be read or parsed, rather than scoring the project as if no test ran: run `specforge collect` again to rewrite it, or fix or remove the file.",
    },
    CodeEntry {
        code: "E046",
        title: "Declared bounds are contradictory",
        owner: "core",
        level: Level::Error,
        explanation: "`specforge analyze --prove`'s SMT solver found the declared bounds mutually unsatisfiable; the cited bounds form the conflicting core. Bounds are the fields an extension declares with the `bound` proof role (a governance constraint's `metric`, a formal axiom's `expression`). Relax or correct one of the listed bounds.",
    },
    CodeEntry {
        code: "E048",
        title: "Proof coverage below the minimum",
        owner: "core",
        level: Level::Error,
        explanation: "`specforge analyze coverage --min N` found that fewer than N% of testable entities are proven, where an entity is proven when a passing test names each of its `verify` obligations or a formal claim discharges it. Prove more obligations, or lower the threshold. The run exits 1.",
    },
    CodeEntry {
        code: "E051",
        title: "Invalid event trigger",
        owner: "@specforge/software",
        level: Level::Error,
        explanation: "An `event` entity's `trigger` field must reference a `behavior`, but it points at something else (or nothing resolvable). Point `trigger` at an existing `behavior` entity.",
    },
    CodeEntry {
        code: "E052",
        title: "Deliverable dependency cycle",
        owner: "@specforge/product",
        level: Level::Error,
        explanation: "The `depends_on` edges between `deliverable` entities form a cycle. Break the cycle by removing or inverting one of the dependencies.",
    },
    CodeEntry {
        code: "E054",
        title: "Invalid extension specifier",
        owner: "core",
        level: Level::Error,
        explanation: "The extension identifier passed to install couldn't be parsed — it was empty or didn't match `name@version`, a local path, or a `git+https://...` URL — or a local install path didn't exist on disk. Use a valid specifier format or check the local file path.",
    },
    CodeEntry {
        code: "E055",
        title: "Invalid surface contribution schema",
        owner: "core",
        level: Level::Error,
        explanation: "A surface contribution's schema is malformed: an MCP tool's `input_schema` or `output_schema` isn't a JSON object, or a CLI command declares an argument type outside the known set (`string`, `path`, `bool`, `enum`, `integer`). Fix the schema or argument type in the manifest.",
    },
    CodeEntry {
        code: "E056",
        title: "Failed to write collected report",
        owner: "core",
        level: Level::Error,
        explanation: "`specforge collect` ingested test results but couldn't write its aggregated coverage report file to disk. Check that the output path is writable.",
    },
    CodeEntry {
        code: "E057",
        title: "Provider scheme conflict",
        owner: "core",
        level: Level::Error,
        explanation: "Two extensions both register a `ref` provider for the same scheme. Configure distinct schemes for each provider extension.",
    },
    CodeEntry {
        code: "E058",
        title: "No test collector",
        owner: "core",
        level: Level::Error,
        explanation: "`specforge collect` found no collector to use: no enabled extension provides one, none of the enabled collectors' detection files (such as `Cargo.toml`) are present at the project root, the `--runner` name matches no collector, a collector declares a report path outside the project, or `--report` was passed while several collectors apply. Enable a runner extension (e.g. `specforge add @specforge/cargo-test`) or pick one with `--runner`.",
    },
    CodeEntry {
        code: "E059",
        title: "Test command not approved",
        owner: "core",
        level: Level::Error,
        explanation: "A runner extension declares the command that runs its tests, and `specforge collect` only runs it after you approve it for the project. The approval is asked at an interactive prompt and remembered per project, extension and command, in your user-level `~/.specforge/collector-consent.json`, never in the project. Without a terminal (CI, `--format json`) nothing is asked: pass `--yes` to run the command, or `--no-run` to parse a report the runner already wrote.",
    },
    CodeEntry {
        code: "E060",
        title: "Resolved reference without a graph edge",
        owner: "core",
        level: Level::Error,
        explanation: "A reference list names an entity that exists, but the resolver never turned the reference into a graph edge. That is a SpecForge bug, not a mistake in your spec: queries, traces and coverage would miss the relationship. Please report it with the spec that triggers it.",
    },
    CodeEntry {
        code: "E061",
        title: "Field value is not the declared type",
        owner: "core",
        level: Level::Error,
        explanation: "The extension that registers a field declares its type, and the value given can't be that type: an integer field got something other than an integer, a bool field something other than true or false, an enum field a value outside its declared values (the suggestion names the closest one), or a field declared as a single value got a list. Values that can be read as the declared type are converted without a diagnostic: a single string or reference on a list field becomes a one-item list, and a quoted integer or boolean becomes the number or boolean. Fix the value, or check the field's type with `specforge schema --kind <kind>`.",
    },
    CodeEntry {
        code: "E062",
        title: "Token budget too small for the export",
        owner: "core",
        level: Level::Error,
        explanation: "`specforge export --max-tokens` (or the MCP `export` tool's `max_tokens`) keeps the most central entities that fit the budget, but some of the export never shrinks: the envelope (`format_version`, `schema_version`), the `token_budget` block listing the dropped entity IDs, and, with `--with-schema`, the embedded schema, which is never cut short. The budget is below that fixed part, so no export fits. Raise the budget, or drop `--with-schema` when the schema alone is over it.",
    },
    CodeEntry {
        code: "E063",
        title: "No registry configured",
        owner: "core",
        level: Level::Error,
        explanation: "`specforge add @scope/name@version`, `update`, `search`, `publish` and `login` (and the MCP `add_extension` tool) talk to an extension registry, and SpecForge has no built-in one: the only registries are those the project's `specforge.json` lists. None is listed, so the command stopped before making any network call. Add a `registries` array, for example `\"registries\": [{\"alias\": \"main\", \"url\": \"<registry URL>\", \"default_registry\": true}]`; an entry with `\"scope_filter\": \"@acme\"` serves only that scope, and the entry marked `default_registry` serves the rest. For `login`, `--registry <alias>` must name one of the entries, or one must be the default. Builtin extensions (`specforge add @specforge/product`) and local `.wasm` files need no registry.",
    },
    CodeEntry {
        code: "E064",
        title: "Unsupported extension source",
        owner: "core",
        level: Level::Error,
        explanation: "`specforge add` was given a `git+https://...` extension specifier. It parses, but installing from git isn't supported yet. Install from a registry (`name@version`) or from a local `.wasm` path instead.",
    },
    CodeEntry {
        code: "E065",
        title: "Invalid scaffold request",
        owner: "core",
        level: Level::Error,
        explanation: "`specforge new` can't scaffold what was asked: only `--extension` projects are supported, the extension name is empty or is a scoped name that isn't `@scope/name`, or the destination directory already exists. Pass `--extension`, fix the name, or pick a destination that doesn't exist yet.",
    },
    CodeEntry {
        code: "E066",
        title: "Extension scaffold failed",
        owner: "core",
        level: Level::Error,
        explanation: "`specforge new --extension` couldn't write the project: creating a directory or writing one of the generated files failed. Check permissions and free space at the destination, remove the partial project, and retry.",
    },
    CodeEntry {
        code: "E067",
        title: "Invalid registry configuration",
        owner: "core",
        level: Level::Error,
        explanation: "The registry configuration in `specforge.json` can't be read: the file isn't valid JSON, `registries` isn't an array, or an entry at the reported index is missing a required field or has the wrong type. That part of the configuration is ignored. Fix the reported entry.",
    },
    CodeEntry {
        code: "E068",
        title: "Coverage gate without the coverage pass",
        owner: "core",
        level: Level::Error,
        explanation: "`specforge analyze --min N` gates on proof coverage, which the `coverage` pass of `@specforge/testing` computes, but that pass didn't run: the extension isn't enabled, or `--pass` selected a different pass. Enable it with `specforge add @specforge/testing`, and run the `coverage` (or `all`) pass. The run exits 2.",
    },
    CodeEntry {
        code: "I002",
        title: "Structural-only mode",
        owner: "core",
        level: Level::Info,
        explanation: "Emitted when no extensions are installed, or when every installed extension failed to load, so the compiler falls back to structural-only validation. Install an extension (for example `specforge add @specforge/software`) to enable kind-specific checks.",
    },
    CodeEntry {
        code: "I003",
        title: "No registry configured",
        owner: "core",
        level: Level::Info,
        explanation: "The registry configuration has no `registries` array, or none of the configured registries is marked as the default. Add a `registries` entry and set `\"default_registry\": true` on one of them.",
    },
    CodeEntry {
        code: "I004",
        title: "Extension not installed",
        owner: "core",
        level: Level::Info,
        explanation: "A reference field targets a kind no enabled extension declares, an extension enhances a kind no enabled extension declares, or a `.spec` file has an `@scope/name` extension import for a known but not-installed extension. Install the missing extension with `specforge add <name>` to resolve the reference. An entity whose keyword no enabled extension declares is an error instead (E024), whose suggestion names the extension to install. An extension's enhancement of a kind owned by an extension the project doesn't use is skipped silently, not reported.",
    },
    CodeEntry {
        code: "I005",
        title: "Unknown provider scheme",
        owner: "core",
        level: Level::Info,
        explanation: "A `ref` entity's `scheme` field, or a `scheme:target` provider reference, doesn't match any provider scheme registered by an installed extension. Install an extension that contributes that provider, or configure it in `specforge.json`.",
    },
    CodeEntry {
        code: "I007",
        title: "Older format version detected",
        owner: "core",
        level: Level::Info,
        explanation: "The `.spec` file's declared format version is older than the compiler's current format version. Run `specforge migrate` to upgrade the file to the current format.",
    },
    CodeEntry {
        code: "I010",
        title: "Unreferenced term",
        owner: "@specforge/product",
        level: Level::Info,
        explanation: "A `term` entity has no edges at all, meaning nothing links to or from it via `see_also` or similar references. Link the term from a relevant entity, or remove it if it's unused.",
    },
    CodeEntry {
        code: "I016",
        title: "Schema cache missing",
        owner: "core",
        level: Level::Info,
        explanation: "Prior exports exist but `.specforge/schema-cache.json` is missing, so breaking-change detection was skipped for this compilation. Run a full compilation to regenerate the schema cache.",
    },
    CodeEntry {
        code: "I017",
        title: "Command not auto-promoted to MCP tool",
        owner: "core",
        level: Level::Info,
        explanation: "An extension command would normally be auto-promoted to an MCP tool named `specforge.<ext>.<command>`, but an explicit MCP tool with that name already exists. The explicit tool definition takes precedence, so no action is needed unless the name collision was unintended.",
    },
    CodeEntry {
        code: "I020",
        title: "Unknown entity kind in a filter",
        owner: "core",
        level: Level::Info,
        explanation: "A `kinds` filter passed to the `specforge.query` or `specforge.search` MCP tool names a kind that no loaded extension defines and no entity has. The kind matches nothing and is dropped from the filter; the report rides in the tool result's `_meta.diagnostics`, with a `did you mean` suggestion when a known kind is close. Fix the spelling, or enable the extension that defines the kind.",
    },
    CodeEntry {
        code: "I046",
        title: "Unreferenced persona",
        owner: "@specforge/product",
        level: Level::Info,
        explanation: "A `persona` entity has no incoming edges, meaning no `journey` references it. Reference the persona from a journey, or remove it if it's no longer needed.",
    },
    CodeEntry {
        code: "I047",
        title: "Unreferenced channel",
        owner: "@specforge/product",
        level: Level::Info,
        explanation: "A `channel` entity has no incoming edges, meaning no `journey` references it. Reference the channel from a journey, or remove it if it's no longer needed.",
    },
    CodeEntry {
        code: "I048",
        title: "Feature without acceptance criteria",
        owner: "@specforge/product",
        level: Level::Info,
        explanation: "A `feature` has no `acceptance` field, or an empty one. Add acceptance criteria describing when the feature is done; they can be added progressively.",
    },
    CodeEntry {
        code: "I050",
        title: "Journey with empty flow",
        owner: "@specforge/product",
        level: Level::Info,
        explanation: "A `journey` declares `flow []`: it has no steps. Add the steps the user goes through. (A journey with no `flow` field at all is the core's E006, since `flow` is required.)",
    },
    CodeEntry {
        code: "I053",
        title: "Milestone target date not YYYY-MM-DD",
        owner: "@specforge/product",
        level: Level::Info,
        explanation: "A `milestone`'s `target_date` doesn't match `YYYY-MM-DD` (for example `Q3 2026` or `June`). Write the date as an ISO 8601 calendar date, e.g. `2026-06-30`.",
    },
    CodeEntry {
        code: "I054",
        title: "Journey without persona",
        owner: "@specforge/product",
        level: Level::Info,
        explanation: "A `journey` has no `persona` field, so it names no user role. Set `persona` to the persona who takes the journey.",
    },
    CodeEntry {
        code: "I055",
        title: "Journey without channels",
        owner: "@specforge/product",
        level: Level::Info,
        explanation: "A `journey` has no edge to a `channel`: no `channels` field, an empty list, or only references that don't resolve. List the channels the journey happens through in `channels`.",
    },
    CodeEntry {
        code: "I057",
        title: "Blocked milestone without dependencies",
        owner: "@specforge/product",
        level: Level::Info,
        explanation: "A `milestone` has `status: blocked` but no `depends_on` entries, so nothing in the plan says what it waits for; the status may be stale. Add the milestones it depends on to `depends_on`, or update the status.",
    },
    CodeEntry {
        code: "I059",
        title: "Deferred feature missing reason",
        owner: "@specforge/product",
        level: Level::Info,
        explanation: "A `feature` has `status: deferred` but no `reason` field explaining why. Add a `reason` field describing why the feature was deferred.",
    },
    CodeEntry {
        code: "I060",
        title: "Blocked milestone missing blockers",
        owner: "@specforge/product",
        level: Level::Info,
        explanation: "A `milestone` has `status: blocked` but no `blockers` field listing what's blocking it. Add a `blockers` field describing what is blocking progress.",
    },
    CodeEntry {
        code: "I061",
        title: "Deliverable version not semver",
        owner: "@specforge/product",
        level: Level::Info,
        explanation: "A `deliverable`'s `version` isn't a Semantic Versioning 2.0.0 version (for example `v1.0`, `1.0` or `latest`). Use `MAJOR.MINOR.PATCH`, optionally with a pre-release tag (`1.0.0-alpha.1`) or build metadata (`1.0.0+build.42`).",
    },
    CodeEntry {
        code: "I062",
        title: "Non-standard module family",
        owner: "@specforge/product",
        level: Level::Info,
        explanation: "A `module`'s `family` isn't one of the standard families: core, platform, extension, integration, advisory. Custom families are allowed; use a standard one if it fits.",
    },
    CodeEntry {
        code: "I066",
        title: "Deprecated deliverable missing reason",
        owner: "@specforge/product",
        level: Level::Info,
        explanation: "A `deliverable` has `status: deprecated` but no `reason` field explaining why. Add a `reason` field documenting why it was deprecated.",
    },
    CodeEntry {
        code: "I067",
        title: "Module without features",
        owner: "@specforge/product",
        level: Level::Info,
        explanation: "A `module` has no edge to a `feature`: no `features` field, an empty list, or only references that don't resolve. A module that implements no features is likely incomplete. List the features it implements in `features`.",
    },
    CodeEntry {
        code: "I068",
        title: "Tag not lowercase-hyphenated",
        owner: "@specforge/product",
        level: Level::Info,
        explanation: "An entity of a product kind has a `tags` entry that isn't lowercase-hyphenated: 2 to 50 characters of a-z, 0-9 and `-`, not starting or ending with `-` (for example `Core`, `my_tag`, `a` or a tag with spaces). Empty entries are ignored. Rewrite the tag, e.g. `My Tag` as `my-tag`.",
    },
    CodeEntry {
        code: "I069",
        title: "Deprecated persona missing reason",
        owner: "@specforge/product",
        level: Level::Info,
        explanation: "A `persona` has `status: deprecated` but no `reason` field explaining why. Add a `reason` field documenting why it was deprecated.",
    },
    CodeEntry {
        code: "I070",
        title: "Deprecated channel missing reason",
        owner: "@specforge/product",
        level: Level::Info,
        explanation: "A `channel` has `status: deprecated` but no `reason` field explaining why. Add a `reason` field documenting why it was deprecated.",
    },
    CodeEntry {
        code: "I080",
        title: "Entity without owner",
        owner: "@specforge/product",
        level: Level::Info,
        explanation: "A `feature`, `milestone`, `deliverable` or `release` has no `owner` field. Set `owner` to the person or team responsible.",
    },
    CodeEntry {
        code: "I081",
        title: "Feature without effort estimate",
        owner: "@specforge/product",
        level: Level::Info,
        explanation: "A `feature` has no `effort` field. Set `effort` to one of xs, s, m, l, xl.",
    },
    CodeEntry {
        code: "I082",
        title: "Release without deliverables",
        owner: "@specforge/product",
        level: Level::Info,
        explanation: "A `release` has no edge to a `deliverable`: no `deliverables` field, an empty list, or only references that don't resolve. List the deliverables it ships in `deliverables`.",
    },
    CodeEntry {
        code: "I083",
        title: "Release without milestones",
        owner: "@specforge/product",
        level: Level::Info,
        explanation: "A `release` has no edge to a `milestone`: no `milestones` field, an empty list, or only references that don't resolve. List the milestones it completes in `milestones`.",
    },
    CodeEntry {
        code: "I086",
        title: "Release date not YYYY-MM-DD",
        owner: "@specforge/product",
        level: Level::Info,
        explanation: "A `release`'s `release_date` doesn't match `YYYY-MM-DD` (for example `June 2026`). Write the date as an ISO 8601 calendar date, e.g. `2026-06-01`.",
    },
    CodeEntry {
        code: "I087",
        title: "Milestone start date not YYYY-MM-DD",
        owner: "@specforge/product",
        level: Level::Info,
        explanation: "A `milestone`'s `start_date` doesn't match `YYYY-MM-DD` (for example `Jan 15`). Write the date as an ISO 8601 calendar date, e.g. `2026-01-15`.",
    },
    CodeEntry {
        code: "I089",
        title: "Recalled release without reason",
        owner: "@specforge/product",
        level: Level::Info,
        explanation: "A `release` has `status: recalled` but no `reason` field, or an empty one. Add a `reason` explaining why the release was recalled.",
    },
    CodeEntry {
        code: "I098",
        title: "Solver could not decide bounds",
        owner: "core",
        level: Level::Info,
        explanation: "The `specforge analyze --prove` SMT solver returned an undecided result rather than `sat`/`unsat` when checking the combined declared bounds, or whether they entail a declared claim. Simplify the bound or claim expressions, or supply tighter bounds, so the solver can decide.",
    },
    CodeEntry {
        code: "I200",
        title: "Stale inferred entities",
        owner: "core",
        level: Level::Info,
        explanation: "A source file has changed on disk since it was last analyzed by inference, so the entities inferred from it may no longer be accurate. Re-analyze the file to refresh its inferred entities.",
    },
    CodeEntry {
        code: "I202",
        title: "High inference density",
        owner: "core",
        level: Level::Info,
        explanation: "A source file produced an unusually high number of inferred entities relative to its line count, exceeding the configured density threshold. Review the file for over-eager inference, or adjust the density threshold if that density is expected.",
    },
    CodeEntry {
        code: "I999",
        title: "Diagnostic output truncated",
        owner: "core",
        level: Level::Info,
        explanation: "More diagnostics were produced than the 100-diagnostic display limit, so only the first batch is shown. Fix the listed diagnostics and rerun the compiler to see the rest.",
    },
    CodeEntry {
        code: "R-AUTH-020",
        title: "Stored registry token expired",
        owner: "core",
        level: Level::Error,
        explanation: "The token `specforge login` stored for this registry expired at the reported time. Log in again with a new token: `specforge login --registry <alias> --token <NEW_TOKEN>`.",
    },
    CodeEntry {
        code: "R-AUTH-021",
        title: "Stored registry token unreadable",
        owner: "core",
        level: Level::Error,
        explanation: "The OS keyring entry that holds this registry's token is missing or can't be read, although the credentials file refers to it. Log in again: `specforge login --registry <alias> --token <NEW_TOKEN>`.",
    },
    CodeEntry {
        code: "R-LOGIN-001",
        title: "No login token given",
        owner: "core",
        level: Level::Error,
        explanation: "`specforge login` was run without a token. Pass one with `--token <TOKEN>`.",
    },
    CodeEntry {
        code: "R-LOGIN-002",
        title: "Login token not stored",
        owner: "core",
        level: Level::Error,
        explanation: "`specforge login` couldn't store the token in the OS keyring or in the fallback file `~/.specforge/credentials.json`. Check that the keyring service is available and that `~/.specforge` is writable.",
    },
    CodeEntry {
        code: "R-OPS-001",
        title: "No registry for the package",
        owner: "core",
        level: Level::Error,
        explanation: "No configured registry serves this package: none has a scope that matches it, and none is marked as the default (or no registries are configured at all). Add a `registries` entry to `specforge.json` with a matching scope, or mark one `\"default_registry\": true`.",
    },
    CodeEntry {
        code: "R-OPS-002",
        title: "Package integrity check failed",
        owner: "core",
        level: Level::Error,
        explanation: "The SHA-256 hash of the downloaded package doesn't match the hash the registry published for it, so the download is corrupt or was tampered with. Retry the download; if it keeps failing, don't install the package.",
    },
    CodeEntry {
        code: "R-OPS-003",
        title: "Manifest not serializable",
        owner: "core",
        level: Level::Error,
        explanation: "`specforge publish` couldn't serialize the extension manifest to JSON for the upload. This is a SpecForge bug, not a mistake in your manifest; please report it.",
    },
    CodeEntry {
        code: "R-RES-001",
        title: "Package not in the registry",
        owner: "core",
        level: Level::Error,
        explanation: "The registry has no package with this name. Check the package name and which registry the configuration sends it to.",
    },
    CodeEntry {
        code: "R-RES-002",
        title: "No published versions",
        owner: "core",
        level: Level::Error,
        explanation: "The registry lists the package, but no version of it (or no version with a valid semver number). Ask the publisher to publish a release, or install from another source.",
    },
    CodeEntry {
        code: "R-RES-003",
        title: "Invalid version range",
        owner: "core",
        level: Level::Error,
        explanation: "The version range isn't valid semver range syntax. Use a range such as `^1.0`, `~2.3` or `>=1.0.0 <2.0.0`, or `latest`.",
    },
    CodeEntry {
        code: "R-RES-004",
        title: "No version satisfies the range",
        owner: "core",
        level: Level::Error,
        explanation: "The registry has versions of the package, but none inside the requested range; the message lists the available ones. Widen the range, or pick one of the listed versions.",
    },
    CodeEntry {
        code: "R-RES-005",
        title: "Unresolvable version diamond",
        owner: "core",
        level: Level::Error,
        explanation: "Several extensions require this package in ranges that no single published version satisfies (see ADR 0001). Upgrade the requirer with the narrowest range, or pin a compatible version manually.",
    },
    CodeEntry {
        code: "R-RES-006",
        title: "Locked peer breaks a version diamond",
        owner: "core",
        level: Level::Error,
        explanation: "The extension being added requires a peer at a version the lock file doesn't have, and the message names a version that would satisfy every requirer. No command pins peer versions yet: reinstall the peer at that version by hand, then run `specforge add` again.",
    },
    CodeEntry {
        code: "R-TRUST-001",
        title: "Unsigned package",
        owner: "core",
        level: Level::Error,
        explanation: "The registry package carries no publisher signature, so where it came from can't be verified. Install it anyway only if you trust the source, with `--allow-unsigned`.",
    },
    CodeEntry {
        code: "R-TRUST-002",
        title: "Invalid package signature",
        owner: "core",
        level: Level::Error,
        explanation: "The package's signature is malformed, or it doesn't verify: the Wasm binary or the manifest isn't what the publisher signed. Don't install the package.",
    },
    CodeEntry {
        code: "R-TRUST-003",
        title: "Publisher key changed",
        owner: "core",
        level: Level::Error,
        explanation: "The package is signed with a different key than the one pinned for it when it was first installed (trust on first use). That can be a key rotation or a compromised publisher. If you trust the new key, re-run with `--yes` or add it to `trusted_keys`.",
    },
    CodeEntry {
        code: "R-TRUST-004",
        title: "Signature metadata mismatch",
        owner: "core",
        level: Level::Error,
        explanation: "The registry's answer doesn't match: it describes another package or version than the one requested, or the key ID it reports differs from the key ID inside the signature, so the registry metadata was edited apart from the signature or is stale. Don't install the package, and check the registry.",
    },
    CodeEntry {
        code: "R-TRUST-005",
        title: "Publisher key denied",
        owner: "core",
        level: Level::Error,
        explanation: "The package is signed with a key listed in `denied_keys` in your known-keys file. Remove the key from `denied_keys` only if you trust it again.",
    },
    CodeEntry {
        code: "R-TRUST-006",
        title: "Known-keys file not writable",
        owner: "core",
        level: Level::Error,
        explanation: "The publisher key pinned for the package couldn't be saved to `~/.specforge/known-keys.json`. Check the permissions on that file and its directory.",
    },
    CodeEntry {
        code: "R001",
        title: "Registry authentication failed",
        owner: "core",
        level: Level::Error,
        explanation: "The registry rejected the request as unauthenticated (HTTP 401), or the credentials its `auth` configuration names couldn't be read; a request is retried once with re-read credentials first. Log in again with `specforge login --registry <alias> --token <TOKEN>`.",
    },
    CodeEntry {
        code: "R002",
        title: "Registry access forbidden",
        owner: "core",
        level: Level::Error,
        explanation: "The registry accepted the credentials but refused the request (HTTP 403). Check your permissions for the registry or the package scope.",
    },
    CodeEntry {
        code: "R003",
        title: "Registry rate limit",
        owner: "core",
        level: Level::Warning,
        explanation: "The registry is rate limiting requests (HTTP 429); the message says how long to wait. Retry after that delay.",
    },
    CodeEntry {
        code: "R004",
        title: "Registry request timed out",
        owner: "core",
        level: Level::Error,
        explanation: "A request to the registry didn't answer in time. Check the network connection and the registry URL, or try again later.",
    },
    CodeEntry {
        code: "R005",
        title: "Registry network error",
        owner: "core",
        level: Level::Error,
        explanation: "A request to the registry failed at the network level. Check the network connection and the registry URL.",
    },
    CodeEntry {
        code: "R006",
        title: "Package version not found",
        owner: "core",
        level: Level::Error,
        explanation: "The registry answered 404 for the package or version. Check the package name and version.",
    },
    CodeEntry {
        code: "R007",
        title: "Version already published",
        owner: "core",
        level: Level::Error,
        explanation: "`specforge publish` tried to publish a version that already exists for the package, and published versions are immutable. Bump the version in the manifest and publish again.",
    },
    CodeEntry {
        code: "R010",
        title: "Registry token variable not set",
        owner: "core",
        level: Level::Error,
        explanation: "The registry's `auth` configuration reads the token from an environment variable that isn't set. Set it (`export <VAR>=<token>`) or change the registry's `auth` configuration.",
    },
    CodeEntry {
        code: "R011",
        title: "Registry token file unreadable",
        owner: "core",
        level: Level::Error,
        explanation: "The registry's `auth` configuration reads the token from a file that can't be read. Check that the file exists and is readable, or change the registry's `auth` configuration.",
    },
    CodeEntry {
        code: "R012",
        title: "Credentials file unreadable",
        owner: "core",
        level: Level::Error,
        explanation: "`~/.specforge/credentials.json` can't be read, or isn't in the expected format. Check its permissions, or delete it and log in again.",
    },
    CodeEntry {
        code: "R013",
        title: "Credentials file not writable",
        owner: "core",
        level: Level::Error,
        explanation: "The credentials couldn't be saved: creating `~/.specforge`, serializing the credentials, or writing `~/.specforge/credentials.json` failed. Check the permissions on `~/.specforge`.",
    },
    CodeEntry {
        code: "W001",
        title: "Behavior implements no feature",
        owner: "@specforge/software",
        level: Level::Warning,
        explanation: "A `behavior` entity has no outgoing edge to any `feature`, meaning it doesn't implement anything declared. Add an `implements` reference to a feature, or remove the behavior if it's unused.",
    },
    CodeEntry {
        code: "W002",
        title: "Unreferenced type",
        owner: "@specforge/software",
        level: Level::Warning,
        explanation: "A `type` entity has no incoming references from any `behavior`, `port`, or other `type`. Reference the type where it's used, or remove it if it's dead.",
    },
    CodeEntry {
        code: "W003",
        title: "Unenforced invariant",
        owner: "@specforge/software",
        level: Level::Warning,
        explanation: "An `invariant` that nothing references: no behavior lists it in `invariants`, `requires`, `ensures` or `maintains`, so no code path is bound to preserve it. Reference it from the behaviors that must preserve it, or remove it if it no longer applies. `specforge analyze coverage` counts these invariants but does not report them again.",
    },
    CodeEntry {
        code: "W004",
        title: "Untested testable entity",
        owner: "@specforge/testing",
        level: Level::Warning,
        explanation: "A testable entity (`behavior`, `invariant`, `event`, `type`, or `port`) declares no `verify` obligations, so nothing states what a test must prove about it. Add `verify unit \"...\"` (or another obligation kind) statements. Union types and entities marked `abstract true` (through a flag their kind declares) are exempt; a struct member named `verify` is a field, not an obligation.",
    },
    CodeEntry {
        code: "W005",
        title: "Unreferenced port",
        owner: "@specforge/software",
        level: Level::Warning,
        explanation: "A `port` entity is not referenced by any `behavior`. Reference the port from a behavior that uses it, or remove it if it's unused.",
    },
    CodeEntry {
        code: "W006",
        title: "Behavior missing category",
        owner: "@specforge/software",
        level: Level::Warning,
        explanation: "A `behavior` entity has no `category` field, which agents rely on for task routing. Add a `category` field to the behavior.",
    },
    CodeEntry {
        code: "W007",
        title: "Event never produced",
        owner: "@specforge/software",
        level: Level::Warning,
        explanation: "An `event` entity is not produced by any `behavior`. Add a `produces` reference from the behavior that emits it, or remove the event if it's unused.",
    },
    CodeEntry {
        code: "W008",
        title: "Unimplemented feature",
        owner: "@specforge/software",
        level: Level::Warning,
        explanation: "A `feature` entity has no incoming edge from any `behavior`, meaning nothing implements it. Add a behavior that implements the feature, or remove it if it's not planned.",
    },
    CodeEntry {
        code: "W009",
        title: "Disallowed verify kind",
        owner: "@specforge/testing",
        level: Level::Warning,
        explanation: "An entity uses a `verify` kind (for example `unit`, `contract`, `integration`) that isn't in the allowed set for its entity kind. Use one of the verify kinds listed as allowed in the diagnostic.",
    },
    CodeEntry {
        code: "W010",
        title: "Unknown field annotation",
        owner: "@specforge/software",
        level: Level::Warning,
        explanation: "A `type` field carries an annotation that isn't recognized by the compiler. Remove the annotation or correct its spelling.",
    },
    CodeEntry {
        code: "W011",
        title: "Edge references missing node",
        owner: "core",
        level: Level::Warning,
        explanation: "An edge was about to be added between two entities, but one or both endpoints don't exist in the graph, so the edge was dropped. Check the referenced entity IDs for typos or missing definitions.",
    },
    CodeEntry {
        code: "W012",
        title: "Unreferenced ref entity",
        owner: "core",
        level: Level::Warning,
        explanation: "A `ref` entity has no incoming edges, meaning nothing in the project references it. Reference the `ref` from another entity, or remove it if it's unused.",
    },
    CodeEntry {
        code: "W017",
        title: "Testable kind lacks verify support",
        owner: "core",
        level: Level::Warning,
        explanation: "An extension registers an entity kind as `testable` but does not set `supportsVerify: true`, so verify statements can't be declared on it. Set `supportsVerify: true` in the extension manifest.",
    },
    CodeEntry {
        code: "W018",
        title: "Duplicate edge type",
        owner: "core",
        level: Level::Warning,
        explanation: "Two extensions register an edge type with the same label; the first-registered extension's definition wins and the later one is ignored. Rename one of the conflicting edge types to avoid the collision.",
    },
    CodeEntry {
        code: "W019",
        title: "Unknown field type",
        owner: "core",
        level: Level::Warning,
        explanation: "An extension manifest declares a field whose `field_type` value the compiler doesn't recognize (it must be one of `string`, `integer`, `bool`, `enum`, `string_list`, `reference`, `reference_list`, or `block`). Correct the field's `field_type` in the manifest.",
    },
    CodeEntry {
        code: "W020",
        title: "Unrecognized field",
        owner: "core",
        level: Level::Warning,
        explanation: "An entity sets a field that isn't declared for its kind by any installed extension. Remove the field, fix a typo in its name, or install the extension that declares it.",
    },
    CodeEntry {
        code: "W021",
        title: "Undeclared target kind or edge label",
        owner: "core",
        level: Level::Warning,
        explanation: "A field or edge type references a `target_kind` or edge label that isn't declared — either in the extension's own manifest when it declares no peer dependencies, or in the compiler's global kind/edge registry once all extensions are loaded. Declare the missing kind or edge label, or add the appropriate peer dependency. The same code reports a declaration the registry refuses: a `derived_from` that derives nothing, a `proof_role` other than `bound` or `claim`, or a `lifecycle_field` that is not one of the kind's fields.",
    },
    CodeEntry {
        code: "W023",
        title: "Duplicate validation rule code",
        owner: "core",
        level: Level::Warning,
        explanation: "Two extensions register a validation rule using the same diagnostic code. Change one extension's rule to use a unique code.",
    },
    CodeEntry {
        code: "W027",
        title: "Re-export binding not found",
        owner: "core",
        level: Level::Warning,
        explanation: "A selective `pub use { A, B } from \"target\"` re-export names a binding that isn't actually exported by the target module. Correct the binding name or remove it from the re-export list.",
    },
    CodeEntry {
        code: "W028",
        title: "Extension memory ceiling exceeded",
        owner: "core",
        level: Level::Warning,
        explanation: "The combined `max_memory_mb` declared across all installed extensions' sandbox policies exceeds the configured total memory ceiling. Reduce `max_memory_mb` in one or more extension sandbox policies.",
    },
    CodeEntry {
        code: "W029",
        title: "Event never consumed",
        owner: "@specforge/formal",
        level: Level::Warning,
        explanation: "An `event` entity is produced by one or more behaviors but has no consumer, meaning nothing reacts to it. Add a behavior that consumes the event, or remove the unused production.",
    },
    CodeEntry {
        code: "W030",
        title: "Abstract behavior unrefined",
        owner: "@specforge/formal",
        level: Level::Warning,
        explanation: "A `behavior` marked `abstract true` has no concrete refinement — no behavior declares `refines` against it, and no `refinement` entity names it as the abstract entity. Add a concrete behavior with `refines`, or a `refinement` entity naming this behavior as the `abstract_entity`.",
    },
    CodeEntry {
        code: "W031",
        title: "Refinement chain too deep",
        owner: "@specforge/formal",
        level: Level::Warning,
        explanation: "A behavior sits in a refinement chain deeper than the maximum allowed depth of 4 layers. Split the refinement chain, or collapse intermediate abstraction layers.",
    },
    CodeEntry {
        code: "W035",
        title: "Undischarged coverage items",
        owner: "@specforge/formal",
        level: Level::Warning,
        explanation: "One or more coverage items (invariants and testable entities) are not proven under @specforge/testing's coverage rule: some obligation has no passing recorded test that names it, and no entailed formal claim discharges it. Link a test to each obligation by its text and run `specforge collect`; `specforge analyze coverage` lists what is unproven (A001, A015).",
    },
    CodeEntry {
        code: "W041",
        title: "Orphan feature",
        owner: "@specforge/product",
        level: Level::Warning,
        explanation: "A `feature` entity has no incoming edges, meaning no `journey`, `milestone`, or `module` references it. Link it from at least one referencing entity, or remove it if it is no longer needed.",
    },
    CodeEntry {
        code: "W042",
        title: "Orphan journey",
        owner: "@specforge/product",
        level: Level::Warning,
        explanation: "A `journey` entity has no incoming edges, meaning no `deliverable` references it. Reference the journey from a deliverable's `journeys` field, or remove it if it is unused.",
    },
    CodeEntry {
        code: "W043",
        title: "Deliverable without journeys",
        owner: "@specforge/product",
        level: Level::Warning,
        explanation: "A `deliverable` has no edge to a `journey`: no `journeys` field, an empty list, or only references that don't resolve. Nothing says which user journeys it supports. List the journeys it serves in `journeys`.",
    },
    CodeEntry {
        code: "W044",
        title: "Orphan module",
        owner: "@specforge/product",
        level: Level::Warning,
        explanation: "A `module` entity has no incoming edges, meaning no `deliverable` or `milestone` references it. Reference the module from a deliverable or milestone, or remove it if it is unused.",
    },
    CodeEntry {
        code: "W045",
        title: "Feature dependency cycle",
        owner: "@specforge/product",
        level: Level::Warning,
        explanation: "Two or more `feature` entities form a cycle through their `depends_on` edges. Break the cycle by removing or restructuring one of the `depends_on` references.",
    },
    CodeEntry {
        code: "W046",
        title: "Deliverable without modules",
        owner: "@specforge/product",
        level: Level::Warning,
        explanation: "A `deliverable` has no edge to a `module`: no `modules` field, an empty list, or only references that don't resolve, so it has no structural decomposition. List the modules it ships in `modules`.",
    },
    CodeEntry {
        code: "W049",
        title: "Milestone without features",
        owner: "@specforge/product",
        level: Level::Warning,
        explanation: "A `milestone` entity has no `features` field, so it may be empty. Modules listed in `modules` don't count: the check reads only `features`. List the features the milestone delivers, or remove the milestone.",
    },
    CodeEntry {
        code: "W050",
        title: "Invalid decision status",
        owner: "@specforge/governance",
        level: Level::Warning,
        explanation: "A `decision` entity's `status` field is not one of the recognized values (`proposed`, `accepted`, `deprecated`, `superseded`). Set `status` to one of these values.",
    },
    CodeEntry {
        code: "W051",
        title: "Invalid failure mode severity",
        owner: "@specforge/governance",
        level: Level::Warning,
        explanation: "A `failure_mode` entity's `severity` or `post_severity` field is not one of the recognized values (`critical`, `high`, `medium`, `low`). Set the field to one of these values.",
    },
    CodeEntry {
        code: "W052",
        title: "Invalid failure mode occurrence",
        owner: "@specforge/governance",
        level: Level::Warning,
        explanation: "A `failure_mode` entity's `occurrence` or `post_occurrence` field is not one of the recognized values (`certain`, `likely`, `occasional`, `unlikely`, `rare`). Set the field to one of these values.",
    },
    CodeEntry {
        code: "W053",
        title: "Breaking schema change",
        owner: "core",
        level: Level::Warning,
        explanation: "A graph protocol schema change classified as breaking: a removed entity kind, edge type or field, a new required field, or a field whose type changed. `specforge export` compares the schema against the one the previous export cached in `.specforge/schema-cache.json`, so an extension upgrade or removal that breaks the schema warns on the next export; the export is still written and the cache updated. `specforge migrate` compares the schema before and after a migration. Update the agents and tools that read the export, keep the extension versions that produced the old schema, or review the migration to preserve backward compatibility.",
    },
    CodeEntry {
        code: "W054",
        title: "Migration structural drift",
        owner: "core",
        level: Level::Warning,
        explanation: "Comparing the entity graph before and after a migration found entities or edges that appeared, disappeared, or changed unexpectedly. Review the migration logic to ensure it preserves the entities and edges it did not intend to change.",
    },
    CodeEntry {
        code: "W057",
        title: "Missing milestone exit criteria",
        owner: "@specforge/product",
        level: Level::Warning,
        explanation: "A `milestone` entity has `status: completed` but no `exit_criteria` field. Add an `exit_criteria` field describing how completion was verified.",
    },
    CodeEntry {
        code: "W060",
        title: "Cross-kind ID collision",
        owner: "core",
        level: Level::Warning,
        explanation: "The same entity ID is declared with two different entity kinds, either within the same compilation pass or across different files. Entity IDs share one flat namespace regardless of kind, so rename one of the conflicting declarations; the first declaration encountered is retained and later ones are skipped.",
    },
    CodeEntry {
        code: "W061",
        title: "Reference cycle detected",
        owner: "core",
        level: Level::Warning,
        explanation: "The resolved reference graph contains a cycle among entity references. Break the cycle by removing or inverting one of the references in the reported path.",
    },
    CodeEntry {
        code: "W062",
        title: "Malformed semver version",
        owner: "core",
        level: Level::Warning,
        explanation: "An extension manifest declares a peer dependency range or a version that is not valid semver. Use a valid semver version (e.g. `1.0.0`) or range (e.g. `^1.0.0`, `~1.2.0`, `>=1.0.0`).",
    },
    CodeEntry {
        code: "W077",
        title: "Invalid feature status",
        owner: "@specforge/product",
        level: Level::Warning,
        explanation: "A `feature` entity's `status` field is not one of the recognized values (`proposed`, `accepted`, `in_progress`, `done`, `deferred`, `deprecated`). Set `status` to one of these values.",
    },
    CodeEntry {
        code: "W078",
        title: "Invalid priority value",
        owner: "@specforge/product",
        level: Level::Warning,
        explanation: "A `feature`, `journey`, `milestone`, or `constraint` entity's `priority` field is not one of the recognized values (`critical`, `high`, `medium`, `low`). Set `priority` to one of these values.",
    },
    CodeEntry {
        code: "W079",
        title: "Invalid milestone status",
        owner: "@specforge/product",
        level: Level::Warning,
        explanation: "A `milestone` entity's `status` field is not one of the recognized values (`planned`, `in_progress`, `completed`, `blocked`). Set `status` to one of these values.",
    },
    CodeEntry {
        code: "W080",
        title: "Invalid deliverable artifact type",
        owner: "@specforge/product",
        level: Level::Warning,
        explanation: "A `deliverable` entity's `artifact_type` field is not one of the recognized values (e.g. `cli`, `service`, `library`, `web_app`, `mobile_app`, `api`, `extension`, `documentation`, `package`). Set `artifact_type` to one of these values.",
    },
    CodeEntry {
        code: "W083",
        title: "Invalid persona status",
        owner: "@specforge/product",
        level: Level::Warning,
        explanation: "A `persona` entity's `status` field is not one of the recognized values (`active`, `deprecated`). Set `status` to one of these values.",
    },
    CodeEntry {
        code: "W084",
        title: "Invalid channel status",
        owner: "@specforge/product",
        level: Level::Warning,
        explanation: "A `channel` entity's `status` field is not one of the recognized values (`active`, `deprecated`). Set `status` to one of these values.",
    },
    CodeEntry {
        code: "W085",
        title: "Invalid deliverable status",
        owner: "@specforge/product",
        level: Level::Warning,
        explanation: "A `deliverable` entity's `status` field is not one of the recognized values (`draft`, `in_progress`, `shipped`, `deprecated`). Set `status` to one of these values.",
    },
    CodeEntry {
        code: "W092",
        title: "Release dependency cycle",
        owner: "@specforge/product",
        level: Level::Warning,
        explanation: "Two or more `release` entities form a cycle through their `depends_on` edges. Break the cycle by removing or restructuring one of the dependency references.",
    },
    CodeEntry {
        code: "W093",
        title: "Invalid release version format",
        owner: "@specforge/product",
        level: Level::Warning,
        explanation: "A `release` entity's `version` field does not match semver format (e.g. `1.0.0`). Set `version` to a valid semver string.",
    },
    CodeEntry {
        code: "W095",
        title: "Invalid feature effort",
        owner: "@specforge/product",
        level: Level::Warning,
        explanation: "A `feature` entity's `effort` field is not one of the recognized values (`xs`, `s`, `m`, `l`, `xl`). Set `effort` to one of these values.",
    },
    CodeEntry {
        code: "W096",
        title: "Behavior requires without ensures",
        owner: "@specforge/formal",
        level: Level::Warning,
        explanation: "A `behavior` entity declares a `requires` clause (an obligation on callers) but no `ensures` clause (a guarantee in return). Add an `ensures` clause describing what the behavior guarantees when its `requires` is satisfied.",
    },
    CodeEntry {
        code: "W097",
        title: "Test record for an unknown entity",
        owner: "core",
        level: Level::Warning,
        explanation: "`specforge analyze` read a recorded test result (in `specforge-report.json`) for an entity ID that no spec declares, so the result counts toward nothing; a `did you mean` hint names a close match when there is one. Fix the entity ID the test names, or declare the entity.",
    },
    CodeEntry {
        code: "W098",
        title: "SMT solver unavailable",
        owner: "core",
        level: Level::Warning,
        explanation: "The `z3` SMT solver could not be found on `PATH`, or failed to execute, so formal entailment and consistency checks during `--prove` were skipped. Install z3 (https://github.com/Z3Prover/z3) and ensure it is executable to enable these checks.",
    },
    CodeEntry {
        code: "W110",
        title: "Refines non-abstract behavior",
        owner: "@specforge/formal",
        level: Level::Warning,
        explanation: "A behavior's `refines` field names a target behavior that is not marked `abstract true`. Add `abstract true` to the target behavior, or point `refines` at a behavior that is actually abstract.",
    },
    CodeEntry {
        code: "W112",
        title: "Validation rule cannot fire",
        owner: "core",
        level: Level::Warning,
        explanation: "An extension-declared validation rule cannot work as declared: its `check` kind is unrecognized, it is missing a field or constraint its check needs, its values list is empty, its `matches` regex does not compile, or its `wasm_function` is absent or failed a probe call. Fix or remove the rule in the extension's manifest.",
    },
    CodeEntry {
        code: "W113",
        title: "Circular file import",
        owner: "core",
        level: Level::Warning,
        explanation: "Two or more `.spec` files import each other, forming a cycle in the import graph. Break the cycle by removing one of the `use` imports or extracting the shared entities into a separate file.",
    },
    CodeEntry {
        code: "W114",
        title: "Integrity check skipped",
        owner: "core",
        level: Level::Warning,
        explanation: "A Wasm extension's integrity check was bypassed because the `--skip-verify` flag was passed. Remove `--skip-verify` to re-enable hash verification of the extension's `.wasm` binary.",
    },
    CodeEntry {
        code: "W115",
        title: "Invalid collector report",
        owner: "core",
        level: Level::Warning,
        explanation: "A collector reported tests for an entity ID that no spec declares (usually a renamed entity or a typo in the test's annotation), or its `total`/`passed`/`failed`/`skipped` stats are inconsistent. `specforge collect` drops those results; fix the test annotation so it names a declared entity.",
    },
    CodeEntry {
        code: "W116",
        title: "Extension discovery failure",
        owner: "core",
        level: Level::Warning,
        explanation: "While scanning an extensions directory, a `manifest.json` could not be read or read directory itself failed, or a manifest failed to parse as valid JSON matching the manifest schema. Fix the directory permissions or correct the malformed `manifest.json`; discovery skips the broken entry and continues with the rest.",
    },
    CodeEntry {
        code: "W117",
        title: "Invalid query extension pattern",
        owner: "core",
        level: Level::Warning,
        explanation: "An extension's tree-sitter query extension pattern (for `highlights`, `locals`, or `injections`) is empty or contains null bytes. Provide a non-empty query pattern with no null bytes; the invalid pattern is skipped rather than loaded.",
    },
    CodeEntry {
        code: "W118",
        title: "Invalid provider configuration",
        owner: "core",
        level: Level::Warning,
        explanation: "A `providers` entry in `specforge.json` is missing its `alias`/`name` or `scheme` field, or no installed extension contributes providers to back a configured provider. Add the missing field, or install an extension that contributes the provider.",
    },
    CodeEntry {
        code: "W119",
        title: "Partial install cleanup failed",
        owner: "core",
        level: Level::Warning,
        explanation: "Rolling back a failed extension install could not remove the partially-created extension directory. Manually delete the leftover extension directory reported in the message.",
    },
    CodeEntry {
        code: "W121",
        title: "Invalid failure mode detection",
        owner: "@specforge/governance",
        level: Level::Warning,
        explanation: "A `failure_mode` entity's `detection` or `post_detection` field is not one of the recognized values (`certain`, `likely`, `moderate`, `unlikely`, `undetectable`). Set the field to one of these values.",
    },
    CodeEntry {
        code: "W123",
        title: "Orphan property",
        owner: "@specforge/formal",
        level: Level::Warning,
        explanation: "A `property` entity is not referenced by any `behavior`, so it may be unused. Reference the property from a behavior's `verify` block, or remove it if it is no longer needed.",
    },
    CodeEntry {
        code: "W125",
        title: "Invalid property type",
        owner: "@specforge/formal",
        level: Level::Warning,
        explanation: "A `property` entity's `property_type` field is not one of the recognized values (`safety`, `liveness`, `fairness`). Set `property_type` to one of these values.",
    },
    CodeEntry {
        code: "W126",
        title: "Orphan axiom",
        owner: "@specforge/formal",
        level: Level::Warning,
        explanation: "An `axiom` entity is not referenced by any other entity, so it may be unused. Reference the axiom from a relevant entity, or remove it if it is no longer needed.",
    },
    CodeEntry {
        code: "W128",
        title: "Orphan protocol",
        owner: "@specforge/formal",
        level: Level::Warning,
        explanation: "A `protocol` entity is not referenced by any `event`, so it may be unused. Reference the protocol from an event, or remove it if it is no longer needed.",
    },
    CodeEntry {
        code: "W131",
        title: "Orphan refinement",
        owner: "@specforge/formal",
        level: Level::Warning,
        explanation: "A `refinement` entity is not referenced by anything, so it may be orphaned. Reference the refinement from the entity it refines, or remove it if it is no longer needed.",
    },
    CodeEntry {
        code: "W134",
        title: "Orphan process",
        owner: "@specforge/formal",
        level: Level::Warning,
        explanation: "A `process` entity is not referenced by any other entity, so it may be unused. Reference the process from a relevant entity, or remove it if it is no longer needed.",
    },
    CodeEntry {
        code: "W137",
        title: "Ambiguous convention mapping",
        owner: "core",
        level: Level::Warning,
        explanation: "`specforge collect` links a test that no annotation links by its name: `entity_id__obligation_slug`, or a module named after an entity. This test's name splits at more than one `__` into a declared entity ID (entity IDs aren't meant to contain `__`), so it isn't linked. Rename the test or the entity, or link the test explicitly (`#[specforge_test]`).",
    },
    CodeEntry {
        code: "W138",
        title: "Unknown manifest field",
        owner: "core",
        level: Level::Warning,
        explanation: "An extension's manifest.json has a top-level field the v2 manifest schema doesn't define, so SpecForge ignores it. It is usually a misspelling (`entityKnds` for `entityKinds`) that leaves the extension without what the field was meant to declare. Fix the spelling or remove the field.",
    },
    CodeEntry {
        code: "W139",
        title: "Formal claim not entailed by declared bounds",
        owner: "core",
        level: Level::Warning,
        explanation: "`specforge analyze --prove` found that a declared claim (a field an extension gives the `claim` proof role, such as a formal property's or invariant's `expression`) isn't guaranteed by the declared bounds: the SMT solver found a counterexample that satisfies every bound while violating the claim. The claim isn't wrong; the bounds just don't guarantee it yet, and its `verify property` obligation stays unproven. Strengthen the declared bounds or weaken the claim. Use `--strict` to fail the run on it. This code was E047 until it was renumbered to match its severity.",
    },
    CodeEntry {
        code: "W140",
        title: "Duplicate registry alias",
        owner: "core",
        level: Level::Warning,
        explanation: "Two entries in `registries` in `specforge.json` share an alias, so the alias doesn't name one registry. Give each registry a unique alias.",
    },
    CodeEntry {
        code: "W141",
        title: "Invalid formatter configuration",
        owner: "core",
        level: Level::Warning,
        explanation: "The formatter's `.specforgefmt.toml` can't be read, isn't valid TOML, or sets `indent_width` (an integer from 1 to 16), `use_tabs` (a boolean) or `max_width` (an integer from 40 to 200) to an invalid value. The formatter uses the default for anything it can't use. Fix the reported setting.",
    },
    CodeEntry {
        code: "W142",
        title: "Unparseable region left unformatted",
        owner: "core",
        level: Level::Warning,
        explanation: "The formatter hit a parse error in a `.spec` file. It keeps the reported line range exactly as written and formats the rest. Fix the syntax there (see E001) and format again.",
    },
    CodeEntry {
        code: "W143",
        title: "Define blocks are not supported",
        owner: "core",
        level: Level::Warning,
        explanation: "A `.spec` file has a `define <name> { ... }` block. Every entity kind comes from an extension, so a project's kinds depend only on `specforge.json` (ADR 0005): the block registers nothing and is left out of the graph. Declare the kind in an extension (`specforge new --extension`) and enable it, then remove the block. `define` stays a reserved word.",
    },
    CodeEntry {
        code: "W144",
        title: "Invalid build cache",
        owner: "core",
        level: Level::Warning,
        explanation: "The project root has a `specforge-cache.json` that can't be read, isn't valid JSON, or declares a `format` other than 1. The build cache records each entity's status from the build `specforge check --cache` last wrote, and check-phase passes compare against it (status transitions); with the file invalid they get no previous statuses, so history rules stay silent. Rewrite it with `specforge check --cache`, or delete it.",
    },
];

/// Codes that are no longer emitted, with the code that replaced them (if
/// any). A retired code is never reused for another meaning.
pub const RETIRED: &[(&str, Option<&str>)] = &[
    ("E029", None),
    ("E037", None),
    ("E038", None),
    ("E047", Some("W139")),
    ("E053", None),
    ("I006", None),
    ("W024", None),
    ("W025", None),
    ("W026", None),
    ("W063", None),
    ("W099", None),
    ("W111", None),
    ("W120", None),
    ("W122", None),
];

/// Look up a retired code (case-insensitive): `Some(replacement)`.
pub fn retired(code: &str) -> Option<Option<&'static str>> {
    let normalized = code.to_uppercase();
    RETIRED
        .iter()
        .find(|(old, _)| *old == normalized)
        .map(|(_, new)| *new)
}

#[cfg(test)]
mod tests {
    use super::*;
    use specforge_test_macros::test as specforge_test;
    use std::collections::{BTreeMap, BTreeSet};
    use std::path::{Path, PathBuf};

    fn workspace_root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    /// Third-party extensions own E900–E998 / W900–W998 / I900–I998.
    /// The registry client's code areas (`R-RES-005`), catalogued as they
    /// are (ADR 0004 D6-c). The grammar is frozen: no new area, no new family.
    const REGISTRY_AREAS: &[&str] = &["AUTH", "LOGIN", "OPS", "RES", "TRUST"];

    /// `E###`/`W###`/`I###`/`A###`, or the registry client's `R###` and
    /// `R-<AREA>-###`.
    fn is_allowed_shape(code: &str) -> bool {
        let digits = |s: &str| s.len() == 3 && s.bytes().all(|c| c.is_ascii_digit());
        if let Some(rest) = code.strip_prefix("R-") {
            return rest
                .split_once('-')
                .is_some_and(|(area, n)| REGISTRY_AREAS.contains(&area) && digits(n));
        }
        code.len() == 4
            && matches!(code.as_bytes()[0], b'E' | b'W' | b'I' | b'A' | b'R')
            && digits(&code[1..])
    }

    fn is_third_party(code: &str) -> bool {
        let b = code.as_bytes();
        if b.len() != 4 || !matches!(b[0], b'E' | b'W' | b'I') {
            return false;
        }
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
             `SPECFORGE_BLESS=1 cargo test -p specforge-diagnostics explain_docs_sync`"
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
            if !is_allowed_shape(entry.code) {
                problems.push(format!(
                    "{}: not of the form E###/W###/I###/A###; R### and R-<AREA>-### (AREA one \
                     of {}) are kept only for the registry client, so a new code takes the \
                     next free E/W/I/A number",
                    entry.code,
                    REGISTRY_AREAS.join(", ")
                ));
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
        for (old, new) in RETIRED {
            if lookup(old).is_some() {
                problems.push(format!("{old} is retired, so it can't be in CATALOG"));
            }
            if new.is_some_and(|code| lookup(code).is_none()) {
                problems.push(format!(
                    "{old} is retired to {new:?}, which isn't in CATALOG"
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
        /// The literal is compared against (`d.code == "E059"`, a
        /// `matches!` pattern, a `const` list of codes to look for), so it
        /// doesn't keep a catalog entry alive.
        consumer: bool,
        /// The severity the emit site most likely uses (see [`site_level`]).
        level: Option<Level>,
    }

    /// Lines searched on each side of a Rust emit site for its severity.
    /// A struct literal sets `severity:` a line or two from `code:`, and a
    /// constructor names it on the code's line or the one before; five
    /// lines covers both without reaching the neighbouring emit site.
    const SEVERITY_WINDOW: usize = 5;

    /// Severity tokens on one line of Rust: `Severity::X`, a `::error(` /
    /// `::warning(` / `::info(` constructor, a `"severity": "x"` JSON key,
    /// or an `error[`/`warning[` prefix printed before a code.
    fn severity_tokens(line: &str) -> Vec<Level> {
        let mut found = Vec::new();
        for (needles, level) in [
            (
                &[
                    "Severity::Error",
                    "::error(",
                    "\"error[",
                    "\"severity\": \"error\"",
                ][..],
                Level::Error,
            ),
            (
                &[
                    "Severity::Warning",
                    "::warning(",
                    "\"warning[",
                    "\"severity\": \"warning\"",
                ][..],
                Level::Warning,
            ),
            (
                &["Severity::Info", "::info(", "\"severity\": \"info\""][..],
                Level::Info,
            ),
        ] {
            if needles.iter().any(|n| line.contains(n)) {
                found.push(level);
            }
        }
        found
    }

    /// The severity an emit site at `lines[index]` most likely uses.
    /// JSON rules: the `"severity"` of the same rule object (up to the next
    /// `"code"`). Rust: the nearest severity token within
    /// [`SEVERITY_WINDOW`] lines; `None` when there is none, or when the
    /// nearest tokens disagree.
    fn site_level(lines: &[&str], index: usize, is_json: bool) -> Option<Level> {
        if is_json {
            for (offset, line) in lines[index..].iter().enumerate() {
                if offset > 0 && line.contains("\"code\"") {
                    return None;
                }
                if let Some(rest) = line.trim().strip_prefix("\"severity\": ") {
                    return match rest.trim_end_matches(',') {
                        "\"error\"" => Some(Level::Error),
                        "\"warning\"" => Some(Level::Warning),
                        "\"info\"" => Some(Level::Info),
                        _ => None,
                    };
                }
            }
            return None;
        }
        for distance in 0..=SEVERITY_WINDOW {
            let mut levels = Vec::new();
            if let Some(before) = index.checked_sub(distance) {
                levels.extend(severity_tokens(lines[before]));
            }
            if distance > 0 && index + distance < lines.len() {
                levels.extend(severity_tokens(lines[index + distance]));
            }
            levels.dedup();
            match levels.as_slice() {
                [] => continue,
                [level] => return Some(*level),
                _ => return None,
            }
        }
        None
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

    /// The end of a code-shaped token starting at `b[i]`: one uppercase
    /// letter, optional `-AREA` segments, then three digits (`E001`, `R013`,
    /// `R-RES-005`, `E-PUB-001`), not followed by another word character.
    /// Any such family is found, so an emitter can't hide a code behind a
    /// new shape; the catalog decides which families are allowed.
    fn code_shape_end(b: &[u8], i: usize) -> Option<usize> {
        if !b.get(i)?.is_ascii_uppercase() {
            return None;
        }
        let mut j = i + 1;
        let mut segments = 0;
        while b.get(j) == Some(&b'-') && b.get(j + 1).is_some_and(u8::is_ascii_uppercase) {
            j += 1;
            while b.get(j).is_some_and(u8::is_ascii_uppercase) {
                j += 1;
            }
            segments += 1;
        }
        if segments > 0 {
            if b.get(j) != Some(&b'-') {
                return None;
            }
            j += 1;
        }
        if !(j..j + 3).all(|k| b.get(k).is_some_and(u8::is_ascii_digit)) {
            return None;
        }
        let end = j + 3;
        let word = |c: &u8| c.is_ascii_alphanumeric() || *c == b'_';
        (!b.get(end).is_some_and(word)).then_some(end)
    }

    /// Find diagnostic codes in string literals on a line, each with the
    /// byte range around it: a whole literal (`"E028"`, `"R-RES-005"`), a
    /// code printed in brackets after a severity (`"error[E048]: ..."`), or
    /// a literal that starts with the code (`"W097: ..."`). Single-letter
    /// families are limited to E, W, I, A, R and F, so strings like
    /// `"X509"` aren't codes.
    fn code_literals(line: &str) -> Vec<(&str, std::ops::Range<usize>)> {
        let b = line.as_bytes();
        let mut found = Vec::new();
        for i in 1..b.len() {
            let Some(end) = code_shape_end(b, i) else {
                continue;
            };
            let code = &line[i..end];
            if code.len() == 4 && !matches!(b[i], b'E' | b'W' | b'I' | b'A' | b'R' | b'F') {
                continue;
            }
            let after = b.get(end).copied();
            let quoted = b[i - 1] == b'"' && matches!(after, Some(b'"') | Some(b':'));
            let bracketed = b[i - 1] == b'['
                && after == Some(b']')
                && ["error", "warning", "info"]
                    .iter()
                    .any(|w| line[..i - 1].ends_with(w));
            if quoted || bracketed {
                found.push((code, i - 1..end + 1));
            }
        }
        found
    }

    /// Whether the code literal on `line` at `range` is only compared
    /// against (a consumer) rather than emitted: an operand of `==`/`!=`,
    /// a match or `matches!` pattern (`"E003" | "E025" =>`), or an item of
    /// a `const` array of codes to look for (`[&str; N]`). A value after
    /// `=>`, in a struct field, a call or a `&str` constant is emitted.
    fn is_consumer(line: &str, range: &std::ops::Range<usize>) -> bool {
        let before = line[..range.start].trim_end();
        let after = line[range.end..].trim_start();
        let alternative = |s: &str| s.starts_with('|') && !s.starts_with("||");
        before.ends_with("==")
            || before.ends_with("!=")
            || after.starts_with("==")
            || after.starts_with("!=")
            || (before.ends_with('|') && !before.ends_with("||"))
            || alternative(after)
            || (after.starts_with("=>") && !before.ends_with("=>"))
            || line.contains("matches!(")
            || (line.contains("const ") && line.contains(": [&str"))
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

    /// The catalog itself: every code appears in it, so the scanner skips it.
    const CATALOG_FILE: &str = "crates/specforge-diagnostics/src/lib.rs";

    /// Every code literal in production (non-test) source, with the owner
    /// implied by where it lives.
    fn emitted_sites() -> Vec<Site> {
        let root = workspace_root();
        let mut files = Vec::new();
        for top in ["crates", "xtask", "integrations", "extensions"] {
            walk(&root.join(top), &mut files);
        }
        let mut sites = Vec::new();
        // A stale CATALOG_FILE would scan the catalog as an emitter, and
        // every entry would look emitted: the skip must match one file.
        let mut catalog_skipped = 0;
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
                // The coverage rule is a shared crate owned by the testing
                // extension (ADR 0004, D2-f); its codes are that extension's.
                if rel_str.starts_with("crates/specforge-coverage/") {
                    "@specforge/testing".to_string()
                } else {
                    "core".to_string()
                }
            };
            if rel_str == CATALOG_FILE {
                catalog_skipped += 1;
                continue;
            }
            if file_name == "tests.rs" || file_name.ends_with("_tests.rs") {
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
                for (code, range) in code_literals(line) {
                    if is_third_party(code) {
                        continue;
                    }
                    sites.push(Site {
                        code: code.to_string(),
                        owner: owner.clone(),
                        location: format!("{rel_str}:{}", index + 1),
                        consumer: is_consumer(line, &range),
                        level: site_level(&lines, index, !is_rust),
                    });
                }
            }
        }
        assert_eq!(
            catalog_skipped, 1,
            "the scanner must skip exactly the catalog file {CATALOG_FILE}"
        );
        sites
    }

    /// Every code-shaped word in `text` (see [`code_shape_end`]; single-letter
    /// families E, W, I, A, R and F), with its line number.
    fn cited_codes(text: &str) -> Vec<(usize, &str)> {
        let word = |c: u8| c.is_ascii_alphanumeric() || c == b'_' || c == b'-';
        let mut found = Vec::new();
        for (index, line) in text.lines().enumerate() {
            let b = line.as_bytes();
            for i in 0..b.len() {
                if i > 0 && word(b[i - 1]) {
                    continue;
                }
                let Some(end) = code_shape_end(b, i) else {
                    continue;
                };
                let single = end - i == 4;
                if !single || matches!(b[i], b'E' | b'W' | b'I' | b'A' | b'R' | b'F') {
                    found.push((index + 1, &line[i..end]));
                }
            }
        }
        found
    }

    /// C3: hand-written docs cite only catalogued codes. `docs/diagnostics.md`
    /// is generated from the catalog, and ADRs are dated records that quote
    /// the codes of their time; the third-party ranges are never catalogued.
    #[test]
    fn hand_written_docs_cite_only_catalogued_codes() {
        let docs = workspace_root().join("docs");
        let mut files = Vec::new();
        walk(&docs, &mut files);
        files.sort();
        let mut problems = Vec::new();
        for path in files {
            let rel = path.strip_prefix(&docs).unwrap_or(&path);
            let rel = rel.to_string_lossy().replace('\\', "/");
            if !rel.ends_with(".md") || rel == "diagnostics.md" || rel.starts_with("adr/") {
                continue;
            }
            let text = std::fs::read_to_string(&path).unwrap_or_default();
            for (line, code) in cited_codes(&text) {
                if lookup(code).is_none() && !is_third_party(code) {
                    problems.push(format!("docs/{rel}:{line} cites {code}"));
                }
            }
        }
        assert!(
            problems.is_empty(),
            "docs cite codes the catalog doesn't have; use the catalogued code (see \
             docs/diagnostics.md) or drop the citation:\n  {}",
            problems.join("\n  ")
        );
    }

    /// C2: a code that is only compared against isn't emitted, so a catalog
    /// entry can't outlive its last emitter through a consumer.
    #[test]
    fn consumer_references_do_not_count_as_emitting() {
        let consumers = [
            r#"        Err(e) if e.code == "E059" => err_invalid("#,
            r#"            if d.code != "E001" {"#,
            r#"        if !matches!(diag.code.as_str(), "E003" | "E025") {"#,
            r#"        if matches!(diag.code.as_str(), "E003") {"#,
            r#"            "E003" | "E025" => fix(diag),"#,
            r#"pub const CONFLICT_CODES: [&str; 2] = ["E017", "W018"];"#,
        ];
        for line in consumers {
            for (code, range) in code_literals(line) {
                assert!(
                    is_consumer(line, &range),
                    "{code} in `{line}` is a consumer"
                );
            }
        }
        let emitters = [
            r#"            Diagnostic::warning("W139", format!("claim"))"#,
            r#"                code: "E028".to_string(),"#,
            r#"      "code": "W041","#,
            r#"const BUDGET_TOO_SMALL: &str = "E062";"#,
            r#"            Kind::Missing => "E025","#,
            r#"        fail("E034", "bad")"#,
        ];
        for line in emitters {
            for (code, range) in code_literals(line) {
                assert!(!is_consumer(line, &range), "{code} in `{line}` is emitted");
            }
        }
    }

    /// The catalog is enforced: every emitted code is registered under the
    /// right owner, and every registered code is emitted somewhere.
    #[specforge_test(
        invariant = "diagnostic_code_uniqueness",
        verify = "Diagnostic Code Uniqueness guarantee holds"
    )]
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
            if site.consumer {
                // A consumer may test for another owner's code, but the code
                // it looks for must exist.
                if !catalog.contains_key(site.code.as_str()) {
                    problems.push(format!(
                        "{} compared against at {} is not in the CATALOG; nothing emits it",
                        site.code, site.location
                    ));
                }
                continue;
            }
            emitted.insert(site.code.as_str());
            match catalog.get(site.code.as_str()) {
                None => problems.push(format!(
                    "{} emitted at {} is not in the CATALOG; add a CodeEntry with owner `{}`",
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
             docs with `SPECFORGE_BLESS=1 cargo test -p specforge-diagnostics explain_docs_sync`.",
            problems.len(),
            problems.join("\n  ")
        );
    }

    /// C4: an `E`/`W`/`I` prefix states the level; `A###` findings leave
    /// it to their pass.
    #[test]
    fn catalog_level_matches_code_prefix() {
        let mut problems = Vec::new();
        for entry in CATALOG {
            let expected = match entry.code.as_bytes()[0] {
                b'E' => Level::Error,
                b'W' => Level::Warning,
                b'I' => Level::Info,
                b'A' => Level::SetByPass,
                _ => continue,
            };
            if entry.level != expected {
                problems.push(format!(
                    "{}: level {:?}, but its prefix says {:?}",
                    entry.code, entry.level, expected
                ));
            }
        }
        assert!(problems.is_empty(), "{}", problems.join("\n"));
    }

    /// C4: no emit site uses a severity its catalog entry contradicts. The
    /// site's severity is a heuristic (see [`site_level`]): the rule
    /// object's `"severity"` for JSON rules, else the nearest severity
    /// token within [`SEVERITY_WINDOW`] lines. Sites where it finds none
    /// are skipped; the test also requires most sites to be decided, so the
    /// heuristic can't quietly stop working.
    #[test]
    fn emit_sites_use_the_catalogued_level() {
        let sites = emitted_sites();
        let mut problems = Vec::new();
        let (mut decided, mut total) = (0, 0);
        for site in sites.iter().filter(|s| !s.consumer) {
            let Some(entry) = lookup(&site.code) else {
                continue;
            };
            if entry.level == Level::SetByPass {
                continue;
            }
            total += 1;
            let Some(level) = site.level else {
                continue;
            };
            decided += 1;
            if level != entry.level {
                problems.push(format!(
                    "{} at {} is emitted as {:?}, but the catalog level is {:?}",
                    site.code, site.location, level, entry.level
                ));
            }
        }
        assert!(
            decided * 3 >= total * 2,
            "the severity heuristic decided only {decided} of {total} emit sites"
        );
        assert!(
            problems.is_empty(),
            "emit sites contradict the catalog level; emit the catalogued severity or \
             move the code to the prefix of the severity it has:\n  {}",
            problems.join("\n  ")
        );
    }
}
