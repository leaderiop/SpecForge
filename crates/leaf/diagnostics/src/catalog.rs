//! The diagnostic catalog: one table, one entry per code, sorted by code.
//!
//! Each entry states a code once, with its level, its owner, its title and
//! its explanation:
//!
//! ```text
//! W112: Warning core,
//!     "<title>",
//!     "<explanation>";
//! R_RES_005 = "R-RES-005": Error core, ...;   // `-` is spelled `_` in the constant
//! W077: Warning product, ...;                 // owner `@specforge/product`
//! ```
//!
//! `catalog!` turns the table into [`CATALOG`](crate::CATALOG) (what
//! `specforge explain`, MCP `specforge.explain`, diagnostics JSON titles,
//! doctor, the LSP hover and `docs/diagnostics.md` read) and into a typed
//! constant for every core code ([`codes`](crate::codes)). An `E`/`W`/`I`/`A`
//! prefix that contradicts the entry's level does not compile.
//!
//! To add a code: add its entry here, name it at the emit site through
//! `codes::X`, then regenerate the docs with
//! `SPECFORGE_BLESS=1 cargo test -p specforge-diagnostics explain_docs_sync`.

/// Generates `pub mod codes` (a constant for every core code) and
/// `pub const CATALOG: &[CodeEntry]` from a table of entries
/// `IDENT [= "ID"]: Level owner, "title", "explanation";`. `owner` is `core`
/// or an extension's short name (`product` for `@specforge/product`). The
/// one table is in this file; the macro is exported only so that its
/// compile-time checks can be shown in a doctest (see [`Code`](crate::Code)).
#[doc(hidden)]
#[macro_export]
macro_rules! catalog {
    (@id $ident:ident []) => {
        stringify!($ident)
    };
    (@id $ident:ident [$id:literal]) => {
        $id
    };
    (@owner core) => {
        "core"
    };
    (@owner $ext:ident) => {
        concat!("@specforge/", stringify!($ext))
    };
    // A core code whose pass sets its level.
    (@const $ident:ident [$($id:literal)?] SetByPass core $title:literal) => {
        #[doc = concat!("`", $crate::catalog!(@id $ident [$($id)?]), "`: ", $title, " (level set by its pass).")]
        pub const $ident: $crate::GradedCode =
            $crate::GradedCode::catalogued($crate::catalog!(@id $ident [$($id)?]));
    };
    // A core code at a fixed level.
    (@const $ident:ident [$($id:literal)?] $level:ident core $title:literal) => {
        #[doc = concat!("`", $crate::catalog!(@id $ident [$($id)?]), "`: ", $title, " (", stringify!($level), ").")]
        pub const $ident: $crate::Code =
            $crate::Code::catalogued($crate::catalog!(@id $ident [$($id)?]), $crate::Level::$level);
    };
    // An extension's code: no host constant, but its prefix is still checked.
    (@const $ident:ident [$($id:literal)?] $level:ident $ext:ident $title:literal) => {
        const _: () = assert!(
            $crate::prefix_states($crate::catalog!(@id $ident [$($id)?]), $crate::Level::$level),
            "an E/W/I/A prefix states the code's level"
        );
    };
    ($( $ident:ident $(= $id:literal)? : $level:ident $owner:ident, $title:literal, $explanation:literal; )*) => {
        /// A typed constant for every core code in [`CATALOG`]: `codes::W112`
        /// is a [`Code`](crate::Code) at the code's level, `codes::R_RES_005`
        /// names `R-RES-005` (`-` becomes `_`), and `codes::A010` is a
        /// [`GradedCode`](crate::GradedCode). Extension-owned codes have none:
        /// extensions report codes as text, checked where they enter the host.
        pub mod codes {
            $( $crate::catalog!(@const $ident [$($id)?] $level $owner $title); )*
        }

        /// Every catalogued code, sorted by code.
        pub const CATALOG: &[$crate::CodeEntry] = &[
            $( $crate::CodeEntry {
                code: $crate::catalog!(@id $ident [$($id)?]),
                title: $title,
                owner: $crate::catalog!(@owner $owner),
                level: $crate::Level::$level,
                explanation: $explanation,
            }, )*
        ];
    };
}

catalog! {
    A001: SetByPass testing,
        "Testable entity without obligations",
        "`specforge analyze coverage` found a testable entity (one whose kind an extension declares testable) that declares no `verify` obligations, so nothing states what a test must prove about it. Add `verify unit \"...\"` (or another obligation kind) statements. What W004 exempts (union types, abstract entities, governance kinds) is not reported.";
    A002: SetByPass testing,
        "Invariant without obligations",
        "An invariant declares no `verify` obligations. It is an error when the invariant's `risk` is `high`, a warning otherwise. Add a `verify property` or `verify unit` statement stating how the guarantee is checked.";
    A010: SetByPass core,
        "Entity without contract obligations",
        "`specforge analyze contracts` found an entity whose kind registers contract reference fields (`requires`, `ensures`, `maintains`, ...) that declares none of them, so no invariant constrains it. Add the references, or ignore this info-level finding for entities that need none.";
    A014: SetByPass testing,
        "Failing tests",
        "The recorded test results (`specforge-report.json`, written by `specforge collect`) include failing tests for this entity, so what it promises is not proven. Fix the code or the test and run `specforge collect` again.";
    A015: SetByPass testing,
        "Unproven obligations",
        "With recorded test results, some of an entity's `verify` obligations are named by no passing test. A test proves an obligation by naming its exact text: `verify = \"...\"` in `#[specforge_test(...)]`, or in a vitest test's `meta.specforge`. Link a test to each listed obligation, or write the missing tests.";
    A016: SetByPass testing,
        "Test names an undeclared obligation",
        "A recorded test names a `verify` obligation that its entity does not declare, usually a typo or an obligation reworded in the spec. The test proves nothing until its text matches the spec's statement exactly; fix whichever side is wrong.";
    E001: Error core,
        "Parse error",
        "A `.spec` file could not be parsed — invalid syntax such as a missing brace, unclosed string, or malformed field at the reported location; the same code also covers an internal reparse failure in the language server, and the formatter's parser producing no syntax tree at all. Fix the syntax at the reported span and re-save.";
    E002: Error core,
        "Duplicate entity ID",
        "Two entities of the same kind declare the same ID, or a project writes a second `spec` block (a project has one, whatever its name); the diagnostic points at the duplicate and its message names where the ID was first declared (file:line:col). Rename one of the entities so each name is unique within its scope and kind (a name is unique within its scope: the entity its kind's scope field names, or the project).";
    E003: Error core,
        "Unresolved reference",
        "A reference field names an entity ID that doesn't resolve to any declared entity: a `reference` field's value or a `reference_list` field's item, quoted or not, or a scheme ref ID in a string list. A `string_list` field's other items are strings, never references. A reference resolves from its holder outward (the holder itself, each enclosing scope, then the project; a leading `/` names the project only), a path such as `03/D-01` resolving at the nearest level where the whole path does. A name outside that chain lists the entities that bear it as candidates; a scope field or a surface's short form naming several entities is ambiguous. Fix the typo, write one of the candidates, or add the missing entity; a `did you mean` suggestion is included when a close match exists.";
    E004: Error core,
        "Unknown type name",
        "A name written as a type (a member's type or a member of its union, a port method's parameter or return type: any field an extension derives from type syntax) is no primitive and names no entity of the field's target kind. A union body's variants and a list's items may be an enum's words and are not checked. Declare the type, or name a declared type or a primitive (see `docs/entities/type.md`).";
    E006: Error core,
        "Missing required field",
        "An entity is missing a field marked `required: true` for its kind in the field registry, or an item of a typed block or block list is missing a member marked `required: true` (the message names the member's path, `requires[1].provides`). Add the missing field or member.";
    E007: Error product,
        "Module dependency cycle",
        "The `depends_on` edges between `module` entities form a cycle. Break the cycle by removing or inverting one of the dependencies.";
    E010: Error software,
        "Invalid milestone behavior range",
        "A `milestone` entity's `behaviors` range field is malformed (the reported reason explains what's wrong, e.g. bad syntax or start after end). Correct the range to a valid form.";
    E013: Error core,
        "Reserved entity ID",
        "An entity's ID is a reserved word — a structural DSL keyword (`spec`, `ref`, `use`, `define`) or an entity kind keyword contributed by an installed extension. Rename the entity, e.g. by appending a suffix like `_rule` or `_spec`.";
    E014: Error core,
        "Entity ID length violation",
        "An entity ID is outside the 2-60 character identifier contract. Pick a descriptive identifier within that length.";
    E015: Error product,
        "Milestone dependency cycle",
        "The `depends_on` edges between `milestone` entities form a cycle. Break the cycle by removing or inverting one of the dependencies.";
    E016: Error core,
        "Referenced file does not exist",
        "A file-reference field on an entity, or a file-reference member of a typed block or block list item, points at a path that doesn't exist under the field's base: the spec root, or, when the field declares `file_base` `project`, the project root. Fix the path, or create the missing file; a similarly-named file is suggested when one is found.";
    E019: Error core,
        "Unsupported format version",
        "A `.spec` file's `// specforge-format: MAJOR.MINOR` header declares a version newer than this build supports, or the header itself doesn't parse; `specforge migrate --target-version` reports the same for a target it can't parse or doesn't support. Lower the declared version or upgrade SpecForge.";
    E022: Error core,
        "Reference targets wrong kind",
        "A reference field is declared to only accept entities of a specific kind, but the target ID resolves to an entity of a different kind. Point the field at an entity of the expected kind. It is reported at the reference, by the linker, whether or not an extension declares the expected kind; the suggestion names a close id of that kind, or the extension that declares it.";
    E024: Error core,
        "Unknown entity kind",
        "An entity block uses a kind keyword that neither the core grammar nor any installed extension declares. Install the extension that provides the keyword, or fix a typo in the kind name.";
    E025: Error core,
        "Import resolution failed",
        "Either a `.spec` file discovery found can't be read (not UTF-8, no permission, not a regular file such as a named pipe or a device, or larger than 64 MiB): it is named, left out of the graph, and an import of it reports nothing more; or a `use` import names no source of the project: no `.spec` file by that path under the spec root that discovery keeps. A file an `exclude` entry leaves out, one under a skipped directory (`target`, `build`, ...), one inside a nested project (a directory below the spec root holding its own `specforge.json`), one above the spec root and one whose name differs only in case are no source. Fix the import path or the `exclude` entry; a `did you mean` suggestion is included when a close match exists.";
    E026: Error core,
        "Entity kind registration conflict",
        "Two extensions register the same entity kind keyword; the first registration wins and the later one is rejected. Rename the conflicting kind keyword.";
    E027: Error core,
        "Unsatisfiable peer dependency",
        "An extension's peer dependency isn't satisfied: a required peer isn't installed, or an installed peer (optional or not) is at a version the range doesn't accept, a version that isn't SemVer included; or required peers form a cycle, reported once naming its extensions by every compile. `specforge add`, `init` and `update` judge the project's peer set as their change leaves it (the extensions its `specforge.json` entries load: builtins, installed extensions and `.wasm` file entries) and refuse a change that would leave a peer it touches unsatisfied (from a registry, a version diamond is R-RES-005/R-RES-006 instead); `add` and `init` first enable the builtins what they add requires. `specforge remove` refuses to uninstall an extension others still require. Install or upgrade the named peer, install a version of the dependent that accepts the builtin this specforge embeds, or make one peer of a cycle optional.";
    E028: Error core,
        "Extension load or execution failure",
        "An extension failed somewhere in its lifecycle: its binary is missing, unreadable (no permission, not a regular file, larger than 64 MiB) or does not load as a component, or `specforge.lock` can't be read so nothing is known to be installed (a `.wasm` file entry of `specforge.json` also when the file declares another name than the entry writes, the name of a builtin, or an extension another entry already loads); its handshake or one of its describe categories failed or does not parse, so its declaration cannot be read (`specforge add` and `specforge publish` refuse such a binary); or a call the host makes on the loaded extension failed. The host calls thirteen exports: the handshake and describe, a command, an MCP tool, an MCP resource, a compiler pass, a collector, a custom validator, a scanner, the migration hook, an MCP prompt, the renderer of an agent's or skill's body and a hook. Each call fails when the export traps (a limit of its sandbox included: its time, its fuel or its memory, the trap naming which), when the extension does not route it, or when it answers output that is not the protocol type the operation owes (a collector's test whose status is not `passed`, `failed` or `skipped`, say); the message names the operation, the export and the extension (`command cmd__x() of '@acme/x' trapped: ...`). What the failure costs is the operation's: a check pass's is the compile's error, an analyze pass's a finding of that pass, a scanner's makes `infer` approximate. Report the failure to the extension's author, and confirm the extension is installed and up to date.";
    E030: Error core,
        "Invalid extension declaration",
        "An extension's declaration (its handshake and describe answers) can't be used as declared: its name or version is empty, its `ext_short` isn't lowercase kebab case (`[a-z][a-z0-9-]*`; it names the extension's CLI subcommand and MCP tool prefix), an analyzer has no language, no file extensions or an empty export name, or a `codes` entry is malformed: a code that is not `E`, `W` or `I` and three digits (a `/` included: the host adds the namespace), a code catalogued twice, an empty title or explanation, or a `docs` that is not an absolute `http(s)` URL; a first-party extension declares no `codes`, its codes are in this catalog. The registry build reports it on every load, and `specforge publish` refuses such a binary before any network call. Fix the declaration in the extension's source (with the SDK: `#[extension(name, version, short)]` and the builders) and rebuild it.";
    E031: Error formal,
        "Refinement drops ensures condition",
        "A behavior that `refines` an `abstract` behavior drops one or more of the abstract behavior's `ensures` conditions. A refinement may only strengthen its abstraction's postconditions, never weaken them — restore or strengthen the missing `ensures` condition(s) in the concrete behavior.";
    E032: Error core,
        "Extension install or uninstall failed",
        "An install, update or uninstall step failed: creating the staging directory, writing the binary, or moving it into place or aside. Everything the change wrote was put back; when something couldn't be, the error names it. Check filesystem permissions and retry.";
    E033: Error core,
        "Lock file error",
        "`specforge.lock` couldn't be serialized, written, read, or parsed (every field of a lock entry is read where the lock is read: its version a SemVer version, its wasm_hash a SHA-256 in lowercase hex, its key_id a publisher key id, its recorded peers and source holding no character a terminal acts on; a source is `registry` or `local:<path>`, the path not empty); `specforge update` also reports it when there is no lock file to update (run `specforge add` first). A lock that is there and can't be read loads no installed extension (each entry that needs it is E028), and every change whose answer it would decide is refused with this code before anything is read or written: an `add` or `init` that installs, any `update`, a `remove` of an extension whose origin only the lock could tell. Enabling or disabling a builtin, or a `.wasm` file entry, still works. Fix specforge.lock, or delete it and install the extensions again with `specforge add`.";
    E034: Error formal,
        "Unmitigated event cycle",
        "`specforge analyze`'s `event_graph_analyze` pass found a cycle in the event flow graph (a behavior `produces` an event another behavior `consumes`, and so on back to the first) through two or more behaviors, and no behavior or event in it declares `sync`. The message gives one cycle path. Declare a `sync` constraint (a timeout, barrier or delivery bound) on one of its members, or break the cycle. The analysis is structural: a declared `sync` counts as the mitigation, what it says is not checked.";
    E039: Error core,
        "Duplicate contribution name",
        "Two contributions claim one name. Two commands route under one name, `specforge <short> <command>`: one short name, and ids that spell one command name (`_` and `-` alike), whether from two extensions or declared twice by one; a prompt is routed under the same names after the commands (ADR 0291), so a prompt whose name a command or an earlier prompt has is the one not routed; the first in load order is routed and the later runs on no surface. Or two agents, skills and slash commands claim one host name (ADR 0292): they are host file names (`.claude/agents/<name>.md`, `.claude/skills/<name>/SKILL.md`, and the same in `.codex` and `.cursor`), one namespace per project across kinds and extensions, and a slash command is named `<short>-<route>`; the first in load order is rendered and the later is not. Or two hooks route as one `<short>.<id>`; the later is not wired. Or two extensions register one MCP tool name, or one MCP resource name. Or two loaded extensions share one code namespace (the namespace is the package name's scope and name: `@gsd/core` and a local `gsd` are both `gsd`, `@a/b.c` and `@a.b/c` are both `a.b.c`): the first in load order owns the namespace and the later extension's codes are described by nobody. One command id under two short names is two names, not a collision. Rename the later contribution, or give its extension another short name (`ext_short`; the SDK's `short`).";
    E040: Error core,
        "Extension project not found or not built",
        "`specforge extension build`, `extension validate`, `publish`, or `add`/`init --extensions` given a local path, found no extension to work on: nothing is at the path, the directory has no `Cargo.toml`, the build (`cargo build --release --target wasm32-wasip2`) failed (the SDK version the crate names not on crates.io among them: a specforge built after its last release names one crates.io does not have yet, see \"Building against an unreleased SDK\" in docs/extension-sdk.md), no built component is in `target/wasm32-wasip2/release/` (or several are, and none is the crate's), or the `.wasm` file couldn't be read (over MCP, also a file with more than one name). Build the extension, fix the build error the message quotes, or name the component to use.";
    E041: Error formal,
        "Refinement chain cycle",
        "The `refines` layering graph between behaviors contains a cycle. Break the cycle by removing or redirecting one of the `refines` edges.";
    E042: Error formal,
        "Process composition cycle",
        "A `process` entity composes, transitively, with itself through its composition edges. Remove or redirect one of the composition steps to break the cycle.";
    E045: Error core,
        "Invalid test report",
        "`specforge collect` couldn't get a test report: the runner's command couldn't be started, it finished without writing a report at the collector's declared location (often because the tests didn't build), `--no-run` found no existing report, or a report file couldn't be read. Check the runner's output above the error and the report path. `specforge stats`, `specforge analyze` and every MCP tool that reads coverage report the same code when `specforge-report.json` exists but can't be read or parsed (a recorded test's status is `passed`, `failed` or `skipped`, and every test has a name: a report in another vocabulary, such as one written before 1.4, is not usable), or when a judged rule reads the report (ADR 0290), or when a check, compute or guard pass that declares `reads_tests` is sent it (protocol 1.11.0), or when a `--test-results` (`test_results`) file can't be read or doesn't exist (over MCP: `schema_mismatch`, `permission_denied`, `file_not_found`, `invalid_input` when `specforge-report.json` is a link that leads outside the project, or `internal_error`), rather than scoring the project as if no test ran: run `specforge collect` again to rewrite it, or fix or remove the file.";
    E046: Error core,
        "Declared bounds are contradictory",
        "`specforge analyze --prove`'s SMT solver found the declared bounds mutually unsatisfiable; the cited bounds form the conflicting core. Bounds are the fields an extension declares with the `bound` proof role (a governance constraint's `metric`, a formal axiom's `expression`). Relax or correct one of the listed bounds.";
    E048: Error core,
        "Proof coverage below the minimum",
        "`specforge analyze coverage --min N` found that fewer than N% of testable entities are proven, where an entity is proven when a passing test names each of its `verify` obligations or a formal claim discharges it. Prove more obligations, or lower the threshold. The run exits 1.";
    E052: Error product,
        "Deliverable dependency cycle",
        "The `depends_on` edges between `deliverable` entities form a cycle. Break the cycle by removing or inverting one of the dependencies.";
    E054: Error core,
        "Invalid extension specifier",
        "The argument given to `specforge add` (or `specforge.add_extension`, or `init --extensions`) isn't a builtin's name, a local path (ending in `.wasm`, or starting with `./`, `../` or `/`), a `git+https://...` URL, or a registry package `@scope/name` with an optional `@version` or `@requirement`. A registry package name is lowercase letters, digits, `.`, `_` or `-` in two parts that each start with a letter or digit. A local path that names nothing, or no readable component, is E040; a local path `specforge.lock` cannot record (one holding a control or bidirectional control character) is this code, and nothing is written. Nothing is asked of any registry.";
    E055: Error core,
        "Invalid surface contribution schema",
        "An extension's explicit MCP tool declares an `input_schema` or `output_schema` that isn't a JSON object, so the tool is not registered. Declare the schema as a JSON Schema object. (A command argument type outside `string`, `path`, `bool`, `enum` and `integer` fails the extension's load as E028: its surfaces don't parse.)";
    E056: Error core,
        "Failed to write collected report",
        "`specforge collect` ingested test results but couldn't write its aggregated coverage report file to disk. Check that the output path is writable.";
    E057: Error core,
        "Provider scheme conflict",
        "Two extensions both register a `ref` provider for the same scheme. Configure distinct schemes for each provider extension.";
    E058: Error core,
        "No test collector",
        "`specforge collect` found no collector to use: no enabled extension provides one, none of the enabled collectors' detection files (such as `Cargo.toml`) are present at the project root, the `--runner` name matches no collector, a collector declares a report path outside the project, or `--report` was passed while several collectors apply. Enable a runner extension (e.g. `specforge add @specforge/cargo-test`) or pick one with `--runner`. Over MCP an unknown `runner` is `invalid_input` naming `runner`; the other cases are `precondition_failed`.";
    E059: Error core,
        "Test command not approved",
        "A runner extension declares the command that runs its tests, and `specforge collect` only runs it after you approve it for the project. The approval is asked at an interactive prompt and remembered per project, extension and command, in your user-level `~/.specforge/collector-consent.json`, never in the project. Without a terminal (CI, `--format json`) nothing is asked: pass `--yes` to run the command, or `--no-run` to parse a report the runner already wrote. The MCP server never asks: approve the command once with `specforge collect` in a terminal, or call `specforge.collect` without `run` to parse the existing report.";
    E061: Error core,
        "Field value is not the declared type",
        "The extension that registers a field declares its type, and the value given can't be that type. A string field takes text, a bare word, a date, a type union or an `expr { }` group; an enum one of its declared values (the suggestion names the closest one; the closed vocabularies the builtins declare, a feature's `status`, a failure mode's `severity`, a property's `property_type`, and the like, are enum fields); an integer field an integer, an integer difference like `2-1` or a date (the message names the operand that is no number); a bool field true or false; a date field one calendar day, `YYYY-MM-DD`, that exists (`2026-02-30` is none); a reference field one name, a number or a date written as one (`10`, `2026-10-26`) included, never one starting with `-`; a string_list, reference_list or enum_list field a list (an enum_list's words are each one of its declared values and appear once); a block field a block, whose members, when it declares them, are each read by their own type, and a block_list field a list of blocks, `[ { a 1 } { a 2 } ]`; a reference field with `target_kinds` names an entity of any of the listed kinds; an integer field with a `minimum` or `maximum` an integer inside them. Values that can be read as the declared type are converted without a diagnostic: a single string, id, integer, boolean or date on a list field becomes a one-item list, a list is stored as written, a quoted integer, boolean or calendar day the number, boolean or date. Fix the value, or check the field's type with `specforge schema --kind <kind>`.";
    E062: Error core,
        "Token budget too small for the export",
        "`specforge export --max-tokens` (or the MCP `export` tool's `max_tokens`) keeps the most central entities that fit the budget, but some of the export never shrinks: the envelope (`format_version`, `schema_version`), the `token_budget` block listing the dropped entity IDs, and, with `--with-schema`, the embedded schema, which is never cut short, and, in a scoped export (`--scope`), the scope's own entity. The budget is below that fixed part, so no export fits. Raise the budget, or drop `--with-schema` when the schema alone is over it.";
    E063: Error core,
        "No registry configured",
        "`specforge add @scope/name@version`, `update`, `search`, `publish` and `login` (and the MCP `add_extension` tool) talk to an extension registry, and SpecForge has no built-in one: the only registries are those the project's `specforge.json` lists. None is listed, so the command stopped before making any network call. Add a `registries` array, for example `\"registries\": [{\"alias\": \"main\", \"url\": \"<registry URL>\", \"default_registry\": true}]`; an entry with `\"scope_filter\": \"@acme\"` serves only that scope, and the entry marked `default_registry` serves the rest. For `login`, `--registry <alias>` must name one of the entries, or one must be the default. Builtin extensions (`specforge add @specforge/product`) and local `.wasm` files need no registry.";
    E064: Error core,
        "Unsupported extension source",
        "`specforge add` was given a `git+https://...` extension specifier. It parses, but installing from git isn't supported yet. Install from a registry (`name@version`) or from a local `.wasm` path instead.";
    E065: Error core,
        "Invalid scaffold request",
        "`specforge extension init` can't scaffold what was asked: the extension name is empty or is a scoped name that isn't `@scope/name`, or the destination directory already exists. Fix the name, or pick a destination that doesn't exist yet.";
    E066: Error core,
        "Extension scaffold failed",
        "`specforge extension init` couldn't write the project: creating a directory or writing one of the generated files failed. Check permissions and free space at the destination, remove the partial project, and retry.";
    E067: Error core,
        "Invalid registry configuration",
        "The registry configuration in `specforge.json` can't be read: the file isn't valid JSON, `registries` isn't an array, or an entry at the reported index is missing a required field, has the wrong type, or has a `url` that isn't a registry's root (an `http` or `https` URL with no query, fragment or user name; write `https://registry.example`, not `https://registry.example/v1`: SpecForge adds the protocol's `/v1`), or has a `scope_filter` that isn't a scope (`@acme`: `@` and one name part; the message names the scope to write). No registry is asked while an entry can't be read, so a package whose scope that entry serves never goes to another registry. Fix the reported entry.";
    E068: Error core,
        "Coverage gate without a coverage figure",
        "`specforge analyze --min N` (and `specforge.analyze`'s `min`) gates on proof coverage, which the `coverage` pass of `@specforge/testing` computes. Either that pass will not run (the extension isn't enabled, or the run selects a different pass), and the analysis is refused before any pass runs; or it ran and gave no figure this specforge reads (it failed, or `@specforge/testing` is a different version), and the run is unjudged. Enable it with `specforge add @specforge/testing`, run the `coverage` (or `all`) pass, and keep the extension at the version this specforge ships. The run exits 2.";
    E069: Error core,
        "Unusable specforge.json",
        "The project root has a `specforge.json` that isn't used as written. When it can't be read, isn't valid JSON, isn't a JSON object, or its `extensions` value isn't an array, the compile uses the default config: no extension loads and only structure is checked (I002 says so), and `specforge add`, `specforge update` and `specforge remove` refuse (config_invalid) without changing anything. `specforge format`, `specforge migrate` and the rename tools refuse the same way when the file can't be used or `spec_root`, `exclude` or `extensions` has the wrong type, since they could not tell which files are the project's sources; on any other problem they still run, report the E069 and fail. A `settings` value that is not an object is one too (the settings are then read as none); a setting that is not used as written is E086, E087 or W166. When a key has the wrong type (`name`, `version` or `spec_root` not a string, `exclude` not an array), that key's default is used; when an item of `extensions` or `exclude` isn't a string, or an `exclude` item can match no source (empty, starting with `!`, with a `.` or `..` name, or not a glob pattern), that item is ignored; the rest of the file is used. Each problem is one E069, and `check` fails on it, so a broken config can't pass a CI that checks nothing. Fix the file: the message names the JSON error's line and column, or the key and the item. A missing `specforge.json` is not this: it is a project with the default config (`specforge doctor` says so).";
    E070: Error core,
        "Installed extension does not match its lock entry",
        "An installed extension's binary, `.specforge/extensions/<name>/extension.wasm`, is not the one its `specforge.lock` entry pins: its SHA-256 differs from the recorded hash (it was changed or replaced after install), or it declares another extension, another version or other peers than the entry records. It is not loaded, and `check` fails. Reinstall it with the command the suggestion names (`specforge add <path>` for a local install, `specforge add <name>@<version>` for a registry one); `specforge doctor` lists a changed binary as `stale_hash`, and a binary declaring other than its entry records as an extension-load error.";
    E071: Error core,
        "Unusable inference manifest",
        "A file inference keeps at the project root is there and can't be used as written: `specforge-infer.json` (the analyzed source files and the inference sessions) or `specforge-anchors.json` (the source anchors navigation reads). It can't be read, isn't valid JSON, doesn't have the manifest's shape (a field missing or of the wrong type, a session status other than `active`, `paused` or `completed`), or, for `specforge-infer.json`, has a `version` other than 1. Nothing reads such a file as empty, and nothing writes over it: `specforge infer-status`, `specforge.infer_progress`, `specforge.infer_gaps`, the infer prompt's plan and every `specforge.infer_session` action refuse with this code, `specforge.find_implementation`, `specforge.find_spec_for_source`, `specforge.infer_gaps`, `specforge infer-status --gaps-detail` and the infer prompt's file scope refuse when it is the anchors manifest, and `specforge check --lint inferred` reports it as an error instead of I200 and I202. Fix the file where the message says (it names the line and column of a JSON or shape error), or move it aside to start over. A missing file is not this: inference has recorded nothing yet.";
    E072: Error core,
        "Invalid package name or version",
        "A name that must be a package name is not one, or a version that must be a SemVer version is not one. A package name is `@scope/name` (what a registry holds) or `name` alone (a local module); each part is lowercase letters, digits, `.`, `_` or `-`, and starts with a letter or digit, so it is always a directory inside `.specforge/extensions`. It is checked where a name does not come from the `add` argument (that is E054): the name and version a local module declares when `specforge add` installs it (a version `specforge.lock` records must be SemVer), a peer a declaration names (or one holding a character a terminal acts on, which `specforge.lock` cannot record), a `specforge.json` `extensions` entry (the entry is not loaded), the name a `.wasm` file entry's component declares (the entry is not loaded), the name `specforge extension validate` reads, the name `specforge remove` is given when no `specforge.json` entry or lock entry has it, and the name and version of the declaration `specforge publish` uploads (which must also be scoped and a full version). Nothing is written or deleted. Fix the extension's declared name or version (its SDK `name`/`version`) and rebuild, or fix the `specforge.json` entry.";
    E073: Error core,
        "Unreadable peer requirement",
        "An extension declares a peer dependency whose range is not a SemVer requirement as Cargo reads one (`^1.2`, `~1`, `>=1, <2`, `1.x`, `*`, or a bare version such as `1.2.0`, which means `^1.2.0`), so no version of the peer can satisfy it, whether the peer is installed or not. `specforge check` (and the LSP and MCP) and `doctor` report it, `specforge add` and `update` refuse to install or update around it, and `specforge publish` refuses to upload it. Fix the range in the extension's declaration (its SDK `peer` or `optional_peer` call) and rebuild, or install a version of the extension that declares one.";
    E074: Error core,
        "Publisher signing key unusable",
        "`specforge publish` signs every package with your publisher key, kept in `~/.specforge/signing-key.json` and created on your first publish. That file is there and can't be read as a key (it is not JSON, or its `secretKey` is not 32 hex-encoded bytes), or the key could not be created there (the directory or the file can't be written). Nothing was sent. Fix the file's permissions, or move it aside: the next publish creates a new key, and whoever pinned the old one then sees a changed publisher key (R-TRUST-003) for your packages.";
    E076: Error core,
        "Block has no title",
        "A block that becomes a graph node has no title, or a blank one: write it after the block's id (`type Task \"A unit of work\" { … }`, `type Status \"Task status\" = open | done`). The title is the one label every surface shows for the node; a `//` comment above the block never reaches the graph. `specforge migrate` writes a project's missing and blank titles: the comment directly above each block when there is one, else its id in words. A `spec` block's title is its name, which migrate never writes; `ref` requires one; `define` and `defaults` blocks are never nodes.";
    E077: Error core,
        "Default for a field its kind does not declare",
        "A `defaults <kind> { … }` block (ADR 0149) sets a field that `<kind>` does not declare, or names a kind no loaded extension declares, or a structural kind (`spec`, `ref`, `use`, `define`, `defaults`). A default only sets a field an extension declares for the kind: a field of an `open_fields` kind's members is not declared. The default is not applied, so no block of the file takes it. Write the field on the blocks that need it, or fix the kind or the field name. Reported by the graph build, once per default, at its value; a project whose extensions declare no kind checks none.";
    E078: Error core,
        "Malformed defaults block",
        "A `defaults <kind> { … }` block (ADR 0149) holds only fields, once each, and comes before the file's first entity block; a file has at most one per kind. This one has a title, a `verify` or `method` member, a field given twice (the first is applied), is a second block for its kind in the file (it is not applied; the message names the first), or comes after the file's first block (it still applies). Remove the title or member, merge the blocks, or move the block above every entity block.";
    E079: Error core,
        "Relation written on its other side",
        "An entity writes a field its kind does not declare, and that field is the other side of a relation another kind owns. A relation is written on its owning field only (ADR 0150): the one reference field an extension declares for it. The other side is derived, never written: `query` shows it as the incoming edges and `trace` as the upstream. The field is recognised in two ways: an owning field names it in its `inverse_of` (so a documented reverse such as `behaviors` on a feature is refused even when its ids do not resolve), or every id it lists is one other kind's entity and that kind owns a reference field into this one (`journeys [..]` on a feature). A self-relation's other side is not recognised, and a kind that accepts any field is not checked. The graph still carries the field's untyped edge, and `check` fails. Remove the field and write the owning field on each entity it listed; the suggestion names both.";
    E080: Error core,
        "Typed edit would add errors",
        "A typed edit (`specforge new`, `set`, `unset`, `link`, `unlink`, `delete`, or MCP `specforge.edit`) was planned, and the project it would leave reports errors the project does not report now: the edit is refused and nothing is written. The refusal lists each new error (an unresolved reference, a missing required field, a value its field does not take, …). Fix what they name first: create the referenced entity, give the required field, or make both changes in one `specforge.edit` call. `delete --unlink-references` removes the references a delete would leave dangling.";
    E081: Error core,
        "Field written twice in one block",
        "A block writes one field (an entry of a block field, a member of a type) more than once. A field holds one value: the graph keeps the first and every output, check and rule reads it; the message names the first's line. Delete the later line, or give every value in the first: a list field takes all its items in one list (`ports [a, b]`), a block field all its entries in one block. A typed edit that changes the field (`set`, `unset`, `link`, `unlink`) removes the later lines itself. A repeat in a `defaults` block is E078.";
    E082: Error core,
        "Computed field written in a spec file",
        "A spec file writes a field, or a member of a block item, that its extension declares `writer: computed` (ADR 0287). A compute pass of the extension gives its value when the project is checked; the value lives beside the graph and in every output, and is never in a `.spec` file. The pass's value wins; this code reports the written one at its value. Delete the line. A field declared `writer: verdict` (ADR 0290) is the same: the fresh verdict of the judged rule that binds it gives its outcome or value, and the message names that rule.";
    E083: Error core,
        "Harness value not recorded",
        "A field its extension declares `writer: harness` holds a value that no line of `specforge-harness` records (ADR 0287). Only a command's plan sets such a field, and the host writes the line with the value; a value typed in a spec file, or changed by hand, has none. When a line of the same field records this value under a path no entity has, the message names it: the entity was renamed by hand, and the ledger was not told. Restore the value from git, or set it through the command that owns it. A harness value is recorded by its line (the digest of the entity's path, the field and the value, with each reference resolved to its target's path), so spelling a reference another way does not change it.";
    E084: Error core,
        "Immutable value changed",
        "A field its extension declares `immutable` (or a member of a block item, or any authored field of an `append_only` kind) holds a value that differs from the one it had at the baseline, the project at `HEAD` (ADR 0287). The message names the old value and the commit. A field the baseline leaves unset may be set once; after that it never changes, and removing it is a change. A value is compared as the host reads it, with each reference resolved to its target's path, so a rename through SpecForge (which records a move) is not a change; the message adds `(as '<old path>')` when the entity was matched through one. Restore the value, or commit the change if it was meant: the next check judges against that commit. `specforge check --since <rev>` judges against the merge-base of `<rev>` and `HEAD` instead.";
    E085: Error core,
        "Append-only history rewritten",
        "A field its extension declares `append_only` holds a list that does not start with the baseline's list, the project at `HEAD` (ADR 0287): an item was removed, reordered or edited, and only appending is allowed. It is also reported for an entity of an `append_only` kind that the baseline holds and the project no longer does. Restore the items, or commit the rewrite if it was meant; the next check judges against that commit.";
    E086: Error core,
        "Setting not used as written",
        "A key of `settings` in `specforge.json` is not used as written (ADR 0288). `settings` is keyed by extension, then by flat dotted setting name (`\"@gsd/core\": { \"workflow.tdd_mode\": true }`), and each name is checked against the extension's declared settings when the project loads: an unknown name (the message suggests the nearest one, or the extension that declares it), a family key outside its list, a value of the wrong type or outside the setting's bounds and vocabulary, `null` on a setting that has a default, or an extension's value that is not an object. The message names the key, never the written value, and the diagnostic points at the key in `specforge.json`. A value that cannot be used is ignored: the preset, else the default, applies. `specforge settings` lists every setting with its type and effective value, `specforge settings set` and `unset` refuse the same way (E086, and E087 for a secret) before they write `specforge.json`, and `specforge schema --config` writes the project's config schema for an editor.";
    E087: Error core,
        "Secret in specforge.json",
        "A key under an extension's `settings` in `specforge.json` is a secret the extension declares, or is named like a credential (its last word is `token`, `secret`, `password` or `credentials`, or `key` after `api`, `private`, `secret` or `access`; words bounded by `_` or `-`, so `max_prompt_tokens` and `token_budget` are not one) and is no declared setting (ADR 0288). The second is also found under an extension that is not enabled or did not load. A secret is read from its environment variable only: SpecForge never reads it from a file a repository holds, and no message or output repeats a written value. The value is ignored. Remove it from `specforge.json`, set the variable the message names, and rotate the secret if it was committed. `specforge settings` shows whether each declared secret is set, and `specforge settings set` refuses to write one (ADR 0296).";
    I002: Info core,
        "Structural-only mode",
        "Emitted when no extensions are installed, or when every installed extension failed to load, so the compiler falls back to structural-only validation. Install an extension (for example `specforge add @specforge/software`) to enable kind-specific checks.";
    I003: Info core,
        "No registry configured",
        "The registry configuration has no `registries` array, or none of the configured registries is marked as the default. Add a `registries` entry and set `\"default_registry\": true` on one of them. Without a default, a package no `scope_filter` matches has no registry (R-OPS-001).";
    I004: Info core,
        "Extension not installed",
        "A reference field targets a kind no enabled extension declares, an extension enhances a kind no enabled extension declares, or a `.spec` file has an `@scope/name` extension import for a known but not-installed extension. Install the missing extension with `specforge add <name>` to resolve the reference; for a reference field the suggestion names the builtin extension that declares the kind, as E024's does. An entity whose keyword no enabled extension declares is an error instead (E024), whose suggestion names the extension to install. An extension's enhancement of a kind no enabled extension declares is skipped silently, not reported, when the extension declares an optional peer the project does not load.";
    I005: Info core,
        "Unknown provider scheme",
        "A `ref` entity's `scheme` field, or a `scheme:target` provider reference, doesn't match any provider scheme registered by an installed extension. Install an extension that contributes that provider, or configure it in `specforge.json`.";
    I007: Info core,
        "Older format version detected",
        "The `.spec` file's declared format version is older than the compiler's current format version. Run `specforge migrate` to upgrade the file to the current format.";
    I008: Info formal,
        "Coverage item proven by tests",
        "`specforge analyze`'s `coverage_tracking` pass found a coverage item (an entity that counts toward coverage: its kind is testable and it owes or declares obligations) whose every `verify` obligation a passing recorded test names, under @specforge/testing's coverage rule. The message names those tests. An item proven only by an entailed claim is not reported. Nothing to do.";
    I009: Info formal,
        "No unmitigated event cycle",
        "`specforge analyze`'s `event_graph_analyze` pass found no unmitigated cycle in the event flow graph: no cycle through two or more behaviors (E034) and no behavior re-producing an event it consumes (W032), unless a member declares `sync`. Reported once, when behaviors produce or consume events. The analysis is structural; cycles that depend on runtime conditions are not analyzed. Nothing to do.";
    I010: Info product,
        "Unreferenced term",
        "A `term` has no edges and names nothing: no entity links to it, and it names no term in `see_also` and no `module` (a name it writes that does not exist is E003 alone). Link the term from a relevant entity, or remove it if it's unused.";
    I011: Info formal,
        "Ensures without requires",
        "`specforge analyze`'s `condition_check` pass found a `behavior` that declares an `ensures` clause but no `requires` clause, so it guarantees its postconditions for every input. Add a `requires` clause naming what callers must establish first, or leave it out if the behavior accepts every input.";
    I014: Info formal,
        "Specification depth",
        "`specforge analyze`'s `coverage_tracking` pass reports a `behavior`'s specification depth, a ladder in which each level holds the one below it: `prose` (no edges), `entity_graph` (edges, no requires or ensures), `conditions` (level 2: requires or ensures), `invariants` (level 3: also names invariants in `maintains` or `invariants`) and `proofs` (level 4: also proven under @specforge/testing's coverage rule). A behavior at level 2 or deeper is reported with the step to the next level. When more than five behaviors sit at `prose` or `entity_graph`, one more I014 suggests adding requires/ensures to the critical ones.";
    I015: Info formal,
        "Formal analysis available",
        "Behaviors in the project declare `requires` or `ensures`, which the formal passes of `specforge analyze` (condition_check, layering_verify, event_graph_analyze, coverage_tracking) check. Reported once per compile by @specforge/formal's `analysis_available` pass. Run `specforge analyze`.";
    I016: Info core,
        "Schema cache unreadable",
        "`.specforge/schema-cache.json` exists but cannot be read: it is not JSON, not a recorded schema, or names a field type this host does not read. The schema is versioned as if no export had been made, at 1.0.0, by every reader of the schema, until the next `specforge export` records a new cache. Agents that read earlier exports must not take the version as continuing theirs.";
    I017: Info core,
        "Extension surface not served under its name",
        "MCP serves each name once (ADR 0017): the core tools and resources first, then each extension's explicit MCP tools and resources in extension load order, then its commands as the tools `specforge.<ext>.<command>`. A contribution not served under its name is reported here, with why: an explicit tool named as a core tool or an earlier extension's tool; an explicit tool whose input schema declares `path`, which the call target takes out of every call, or `settings`, which the host sends with every call (ADR 0288); a resource whose URIs a core resource or an earlier extension resource already serves; a command whose tool name an explicit tool (or a core tool) already has; a command the host refuses (an arg taking `--path`, `--format` or `--help`, two args spelling one option, or a default its declaration contradicts); a command another command routes first under its name (`specforge check` reports that one as E039). I017 itself only MCP reports. Rename the contribution if the collision is unintended; otherwise no action is needed.";
    I020: Info core,
        "Unknown entity kind in a filter",
        "A kind filter (`specforge export --kinds`, `specforge query --kinds`, `specforge explore --kind`, `specforge model --kinds`, and the `kinds` or `kind` of the `specforge.export`, `specforge.query`, `specforge.search`, `specforge.list`, `specforge.coverage` and `specforge.model` MCP tools, the explore prompt and the graph resources) names a kind that no loaded extension declares and no entity is written with. The kind matches nothing and is dropped from the filter; the report goes to stderr on the CLI and rides in the tool result's `_meta.diagnostics` over MCP, with a `did you mean` suggestion when a known kind is close (a kind equal but for case first: kind names are case-sensitive keywords). Fix the spelling, or enable the extension that defines the kind.";
    I046: Info product,
        "Unreferenced persona",
        "A `persona` entity has no incoming edges, meaning no `journey` references it. Reference the persona from a journey, or remove it if it's no longer needed.";
    I047: Info product,
        "Unreferenced channel",
        "A `channel` entity has no incoming edges, meaning no `journey` references it. Reference the channel from a journey, or remove it if it's no longer needed.";
    I048: Info product,
        "Feature without acceptance criteria",
        "A `feature` has no `acceptance` field, or an empty one. Add acceptance criteria describing when the feature is done; they can be added progressively.";
    I050: Info product,
        "Journey with empty flow",
        "A `journey` declares `flow []`: it has no steps. Add the steps the user goes through. (A journey with no `flow` field at all is the core's E006, since `flow` is required.)";
    I054: Info product,
        "Journey without persona",
        "A `journey` has no `persona` field, so it names no user role. Set `persona` to the persona who takes the journey.";
    I055: Info product,
        "Journey without channels",
        "A `journey` names no channel: no `channels` field or an empty list (a channel it names that does not exist is E003 alone). List the channels the journey happens through in `channels`.";
    I057: Info product,
        "Blocked milestone without dependencies",
        "A `milestone` has `status: blocked` but no `depends_on` entries, so nothing in the plan says what it waits for; the status may be stale. Add the milestones it depends on to `depends_on`, or update the status.";
    I059: Info product,
        "Deferred feature missing reason",
        "A `feature` has `status: deferred` but no `reason` field explaining why. Add a `reason` field describing why the feature was deferred.";
    I060: Info product,
        "Blocked milestone missing blockers",
        "A `milestone` has `status: blocked` but no `blockers` field listing what's blocking it. Add a `blockers` field describing what is blocking progress.";
    I061: Info product,
        "Deliverable version not semver",
        "A `deliverable`'s `version` isn't a Semantic Versioning 2.0.0 version (for example `v1.0`, `1.0` or `latest`). Use `MAJOR.MINOR.PATCH`, optionally with a pre-release tag (`1.0.0-alpha.1`) or build metadata (`1.0.0+build.42`).";
    I063: Info product,
        "Done feature depends on an unfinished feature",
        "A `feature` has `status: done` but a feature its `depends_on` names is not done. Reported once per such dependency, by the `lifecycle` pass of @specforge/product. Finish the dependency (or mark it done), or set the feature back to `in_progress`.";
    I064: Info product,
        "Milestone due before its dependency",
        "A `milestone`'s `target_date` is earlier than that of a milestone its `depends_on` names: it cannot realistically complete before what it waits on. Reported once per such pair; a milestone or dependency without a `YYYY-MM-DD` date is not compared. Move the date, or drop the dependency.";
    I065: Info product,
        "Shipped deliverable tracks an incomplete milestone",
        "A `deliverable` has `status: shipped` but a milestone its `milestones` names is not `completed`. Reported once per such milestone, by the `lifecycle` pass of @specforge/product. Complete the milestone, or set the deliverable back to `in_progress`.";
    I066: Info product,
        "Deprecated deliverable missing reason",
        "A `deliverable` has `status: deprecated` but no `reason` field explaining why. Add a `reason` field documenting why it was deprecated.";
    I067: Info product,
        "Module without features",
        "A `module` names no feature: no `features` field or an empty list (a feature it names that does not exist is E003 alone). A module that implements no features is likely incomplete. List the features it implements in `features`.";
    I068: Info product,
        "Tag not lowercase-hyphenated",
        "An entity of a product kind has a `tags` entry that isn't lowercase-hyphenated: 2 to 50 characters of a-z, 0-9 and `-`, not starting or ending with `-` (for example `Core`, `my_tag`, `a` or a tag with spaces). Empty entries are ignored. Rewrite the tag, e.g. `My Tag` as `my-tag`.";
    I069: Info product,
        "Deprecated persona missing reason",
        "A `persona` has `status: deprecated` but no `reason` field explaining why. Add a `reason` field documenting why it was deprecated.";
    I070: Info product,
        "Deprecated channel missing reason",
        "A `channel` has `status: deprecated` but no `reason` field explaining why. Add a `reason` field documenting why it was deprecated.";
    I071: Info product,
        "Done feature the recorded tests do not prove",
        "A `feature` has `status: done` but the project's recorded test report (`specforge collect`) does not prove it: no behavior implements it (names it in `features`), or not every implementing behavior is proven (an obligation no passing test names, or a failing test). A feature's status is a claim and its evidence is derived (ADR 0039). Reported by the `delivery_evidence` pass under `specforge analyze`, only when a report is recorded. Link tests to the unproven obligations, or name the feature in the behaviors that deliver it.";
    I080: Info product,
        "Entity without owner",
        "A `feature`, `milestone`, `deliverable` or `release` has no `owner` field. Set `owner` to the person or team responsible.";
    I081: Info product,
        "Feature without effort estimate",
        "A `feature` has no `effort` field. Set `effort` to one of xs, s, m, l, xl.";
    I082: Info product,
        "Release without deliverables",
        "A `release` names no deliverable: no `deliverables` field or an empty list (a deliverable it names that does not exist is E003 alone). List the deliverables it ships in `deliverables`.";
    I083: Info product,
        "Release without milestones",
        "A `release` names no milestone: no `milestones` field or an empty list (a milestone it names that does not exist is E003 alone). List the milestones it completes in `milestones`.";
    I089: Info product,
        "Recalled release without reason",
        "A `release` has `status: recalled` but no `reason` field, or an empty one. Add a `reason` explaining why the release was recalled.";
    I098: Info core,
        "Solver could not decide bounds",
        "The `specforge analyze --prove` SMT solver returned an undecided result rather than `sat`/`unsat` when checking the combined declared bounds, or whether they entail a declared claim. Simplify the bound or claim expressions, or supply tighter bounds, so the solver can decide.";
    I200: Info core,
        "Stale inferred entities",
        "A source file has changed on disk since it was last analyzed by inference, so the entities inferred from it may no longer be accurate. Re-analyze the file to refresh its inferred entities.";
    I202: Info core,
        "High inference density",
        "A source file produced an unusually high number of inferred entities relative to its line count, exceeding the configured density threshold. Review the file for over-eager inference, or adjust the density threshold if that density is expected.";
    I203: Info core,
        "Harness ledger not canonical",
        "`specforge-harness` is not the file the host would write: a line out of order, repeated, blank, a second header, a line for an entity or field that is gone, or a stale line for a value that has a current one. A union merge of two branches' ledgers leaves it so. Nothing is wrong with what it records. Run `specforge harness gc` to write it again; `check` never does.";
    I204: Info core,
        "Nothing judged against history",
        "The project declares a history flag (a field `immutable` or `append_only`, a kind `append_only`, or a pass that reads the baseline, ADR 0287), but there is no commit to judge against: the project is not in a git repository, or the repository has no commit yet. Nothing is judged, and nothing is wrong with the project. The first commit is the baseline from then on.";
    I205: Info core,
        "Test inputs not judged",
        "A judged rule (ADR 0290) reads the recorded tests of an entity as an input (a `test(..)` selector, or a verdict that cites tests), or a check, compute or guard pass declares `reads_tests` (protocol 1.11.0), but there is no `specforge-report.json`: run `specforge collect`. The rule's other inputs are judged as usual and the test input is left out of the comparison, so a verdict is not made stale by a report that is not there. Nothing is wrong with the project. A report that exists but cannot be used is E045.";
    I999: Info core,
        "Diagnostic output truncated",
        "More diagnostics were produced than the 100-diagnostic display limit, so only the first batch is shown. Fix the listed diagnostics and rerun the compiler to see the rest.";
    R_AUTH_020 = "R-AUTH-020": Error core,
        "Stored registry token expired",
        "The token `specforge login` stored for this registry expired at the reported time. Log in again with a new token: `specforge login --registry <alias> --token <NEW_TOKEN>`.";
    R_AUTH_021 = "R-AUTH-021": Error core,
        "Stored registry token unreadable",
        "The secret `specforge login --token` kept for this registry (a file under `~/.specforge/secrets/`) is missing or can't be read, although the credentials file refers to it. Log in again: `specforge login --registry <alias> --token <NEW_TOKEN>`. The file is 0600 and its directory 0700 on unix only; on Windows nothing sets a mode, so the secret relies on the user profile directory's default ACL (private to the user).";
    R_LOGIN_001 = "R-LOGIN-001": Error core,
        "No single login token source",
        "`specforge login` needs exactly one of `--token <TOKEN>`, `--token-env <VAR>` or `--token-file <PATH>`; none or several were given.";
    R_LOGIN_002 = "R-LOGIN-002": Error core,
        "Login token not stored",
        "`specforge login` couldn't write the token's secret file under `~/.specforge/secrets/`. Check that `~/.specforge` is writable.";
    R_OPS_001 = "R-OPS-001": Error core,
        "No registry for the package",
        "No configured registry serves this package: none has a scope that matches it, and none is marked as the default (or no registries are configured at all). Add a `registries` entry to `specforge.json` with a matching scope, or mark one `\"default_registry\": true`. `specforge add`, `update` and `publish` (and MCP `add_extension`) choose the registry by this one rule and ask no other registry; a refused name sends no request.";
    R_OPS_002 = "R-OPS-002": Error core,
        "Package integrity check failed",
        "The SHA-256 hash of the downloaded package doesn't match the hash the registry published for it, so the download is corrupt or was tampered with. Retry the download; if it keeps failing, don't install the package.";
    R_OPS_003 = "R-OPS-003": Error core,
        "Manifest not serializable",
        "`specforge publish` couldn't serialize the extension manifest to JSON for the upload. This is a SpecForge bug, not a mistake in your manifest; please report it.";
    R_OPS_004 = "R-OPS-004": Error core,
        "Package manifest unreadable",
        "The registry served no manifest for the package, or one that isn't a valid extension manifest. The manifest declares the package's peer dependencies, which `specforge add` checks against the installed extensions before installing anything, so a package whose manifest can't be read is refused rather than treated as having no peers. Nothing is installed and no publisher key is pinned. Don't install the package, and check the registry.";
    R_RES_001 = "R-RES-001": Error core,
        "Package not in the registry",
        "The registry has no package with this name. Check the package name and which registry the configuration sends it to.";
    R_RES_002 = "R-RES-002": Error core,
        "No published versions",
        "The registry lists the package, but no version of it (or no version with a valid semver number). Ask the publisher to publish a release, or install from another source.";
    R_RES_003 = "R-RES-003": Error core,
        "Invalid version range",
        "The version after `@` isn't `latest`, a full version or valid SemVer requirement syntax, so no registry was asked. Use a version such as `1.2.0`, a requirement such as `^1.0`, `~2.3`, `1.x` or `>=1.0.0, <2.0.0`, or `latest`.";
    R_RES_004 = "R-RES-004": Error core,
        "No version satisfies the range",
        "The registry has versions of the package, but none inside the requested range; the message lists the available ones (for a builtin, the one version this specforge embeds). Widen the range, or pick one of the listed versions; for a builtin, drop the version or use the specforge release that embeds the one you need.";
    R_RES_005 = "R-RES-005": Error core,
        "Unresolvable version diamond",
        "Several extensions require this package in ranges that no single published version satisfies (see ADR 0001). Upgrade the requirer with the narrowest range, or pin a compatible version manually.";
    R_RES_006 = "R-RES-006": Error core,
        "Locked peer breaks a version diamond",
        "An extension being installed or updated requires a peer at a version the lock does not have, or a locked extension requires a version of the package being installed or updated that it would not have. The message names a version of the peer that would satisfy every requirer. Install the peer at that version (`specforge add @scope/peer@<version>`) and retry; when a locked extension holds the peer back, `specforge update --major` moves them together.";
    R_TRUST_001 = "R-TRUST-001": Error core,
        "Unsigned package",
        "The registry package carries no publisher signature, so where it came from can't be verified. Install it anyway only if you trust the source, with `--allow-unsigned`; the MCP server never accepts an unsigned package.";
    R_TRUST_002 = "R-TRUST-002": Error core,
        "Invalid package signature",
        "The package's publisher signature doesn't verify: the Wasm binary or the manifest isn't what the publisher signed. Don't install the package. `--allow-unsigned` doesn't accept it. (A signature that isn't one at all makes the registry's answer unreadable: R009.)";
    R_TRUST_003 = "R-TRUST-003": Error core,
        "Publisher key changed",
        "The package is signed with a different key than the one expected for it: the key pinned for it when you first installed it (trust on first use), or, when you have no pin for it, the key the project's `specforge.lock` records. That can be a key rotation or a compromised publisher. If you trust the new key, accept it with `specforge add` or `specforge update` (yes at the prompt, or `--yes`), or add it to `trusted_keys`; the MCP server never accepts a key change.";
    R_TRUST_004 = "R-TRUST-004": Error core,
        "Registry answer mismatch",
        "The registry's answer doesn't match: it, or the manifest it serves, describes another package or version than the one requested, or the binary declares other than the served declaration. Don't install the package, and check the registry.";
    R_TRUST_005 = "R-TRUST-005": Error core,
        "Publisher key denied",
        "The package is signed with a key listed in `denied_keys` in your known-keys file. Remove the key from `denied_keys` only if you trust it again.";
    R_TRUST_006 = "R-TRUST-006": Error core,
        "Known-keys file unusable",
        "Your known-keys file (`~/.specforge/known-keys.json`) can't be read as the store of publisher pins, or the pins an install accepted can't be saved to it. Nothing is installed either way: a store read as empty would trust any key and lose your `denied_keys`, and an install whose pin can't be saved is put back. Fix the file's JSON, or move it aside (every package is then trusted on first use again), and check the permissions on the file and its directory.";
    R_TRUST_007 = "R-TRUST-007": Error core,
        "Unsigned package for a pinned publisher",
        "The registry package carries no publisher signature, but a publisher key is expected for its name: the key you pinned when you first installed it, or, when you have no pin for it, the key the project's `specforge.lock` records. A package its publisher signs does not arrive unsigned, so this can be a mirror or a compromised registry serving other bytes. Nothing is downloaded, and neither `--allow-unsigned`, `--yes`, an interactive yes nor `trusted_keys` accepts it. Check the registry that serves the name. If its publisher really stopped signing it, remove the extension (`specforge remove <name>`) and its pin in `~/.specforge/known-keys.json`, then add it again.";
    R001: Error core,
        "Registry authentication failed",
        "The registry rejected the request as unauthenticated (HTTP 401). Log in again with `specforge login --registry <alias> --token <TOKEN>`. `specforge publish` refuses before any request when there is no credential for the registry that serves the package: none stored for its alias and `SPECFORGE_REGISTRY_TOKEN` unset or blank.";
    R002: Error core,
        "Registry access forbidden",
        "The registry accepted the credentials but refused the request (HTTP 403). Check your permissions for the registry or the package scope.";
    R003: Warning core,
        "Registry rate limit",
        "The registry is rate limiting requests (HTTP 429); the message says how long to wait. Retry after that delay.";
    R004: Error core,
        "Registry request timed out",
        "A request to the registry didn't answer in time. Check the network connection and the registry URL, or try again later.";
    R005: Error core,
        "Registry network error",
        "A request to the registry failed at the network level. Check the network connection and the registry URL.";
    R006: Error core,
        "Package version not found",
        "The registry answered that it has no such package or version (HTTP 404 with the registry protocol's NOT_FOUND error). Check the package name and version.";
    R007: Error core,
        "Version already published",
        "`specforge publish` tried to publish a version that already exists for the package, and published versions are immutable. Bump the version in the manifest and publish again.";
    R008: Error core,
        "Registry refused the request",
        "The registry read the request and refused it (HTTP 400), naming its reason and code: for a publish, a package that isn't signed or whose signature isn't one or doesn't verify over the uploaded binary and manifest (INVALID_SIGNATURE), a binary that isn't a Wasm module, or a manifest that isn't an extension declaration or names another package. Nothing was stored, and the version can still be published. Fix what the message names and try again; sending the same request again is refused the same way.";
    R009: Error core,
        "Registry reply unreadable",
        "The registry's answer isn't one the registry protocol defines for that request: a body that isn't the request's JSON (such as a publisher signature that isn't an Ed25519 signature object, or a publish receipt that describes another upload than the one sent), a name that is no package name, a version that is no SemVer version, a digest that is not a SHA-256 in lowercase hex or a token expiry that is not an RFC 3339 timestamp (each refused before anything is downloaded or kept), a 404 without the protocol's NOT_FOUND error (no registry serves that address: the configured `url` isn't a SpecForge registry's root), a download URL that is neither an `http`/`https` URL nor a path at the registry's root, or an answer longer than the request admits (a download longer than the `size_bytes` its reply declares, or any answer over 64 MiB, the largest package a registry accepts; it is refused before more of it is read). The registry is broken, or the URL isn't a SpecForge registry's root. Nothing is installed and no key is pinned, and `--allow-unsigned` doesn't accept such a package. Check the registry and its `url` in `specforge.json`.";
    R010: Error core,
        "Registry token variable not set",
        "The registry's credential is a reference to an environment variable (`specforge login --token-env`, or a `token_env` entry in `~/.specforge/credentials.json`) that isn't set, or is blank, where the command runs. Set it, or log in again with another source.";
    R011: Error core,
        "Registry token file unreadable",
        "The registry's credential is a reference to a token file (`specforge login --token-file`, or a `token_file` entry) that can't be read, or is empty. Check the path and its permissions, or log in again with another source.";
    R012: Error core,
        "Credentials file unreadable",
        "`~/.specforge/credentials.json` can't be read, or isn't in the expected format. Check its permissions, or delete it and log in again.";
    R013: Error core,
        "Credentials file not writable",
        "The credentials couldn't be saved: creating `~/.specforge`, serializing the credentials, or writing `~/.specforge/credentials.json` failed. Check the permissions on `~/.specforge`.";
    R014: Error core,
        "Locked package served as other bytes",
        "The registry serves the version of a package that `specforge.lock` pins, but not the binary the lock pins: its SHA-256 is another. A published version never changes (R007), so the registry, or the address `specforge.json` sends the package to, now serves something else under that version. Nothing was downloaded or installed, no publisher key was pinned or asked about, and the lock is left as it was; `specforge add --dry-run` reports it too. Check the registry. If you trust the bytes it now serves, remove the extension (`specforge remove <name>`) and add it again, which pins them.";
    W001: Warning software,
        "Behavior implements no feature",
        "A `behavior` names no feature in `features` (no field, or an empty list), so it implements nothing declared. A feature it names that does not exist is E003 alone, never W001 too. List the features it implements in `features`, or remove the behavior if it's unused.";
    W002: Warning software,
        "Unreferenced type",
        "A `type` entity has no incoming references from any `behavior`, `port`, or other `type`. Reference the type where it's used, or remove it if it's dead.";
    W003: Warning governance,
        "Unenforced invariant",
        "An `invariant` that nothing references: no behavior enforces it (`invariants`), no decision or constraint protects it, no failure mode threatens it and no axiom or property relies on it. Reference it where it is kept, or remove it if it no longer applies. `specforge analyze coverage` counts these invariants but does not report them again.";
    W004: Warning testing,
        "Untested testable entity",
        "A testable entity (`behavior`, `invariant`, `event`, `type`, or `port`) declares no `verify` obligations, so nothing states what a test must prove about it. Add `verify unit \"...\"` (or another obligation kind) statements. Union types and entities marked `abstract true` (through a flag their kind declares) are exempt; a struct member named `verify` is a field, not an obligation.";
    W005: Warning software,
        "Unreferenced port",
        "A `port` entity is not referenced by any `behavior`. Reference the port from a behavior that uses it, or remove it if it's unused.";
    W006: Warning software,
        "Behavior missing category",
        "A `behavior` entity has no `category` field, which agents rely on for task routing. Add a `category` field to the behavior.";
    W007: Warning software,
        "Event never produced",
        "An `event` entity is not produced by any `behavior`. Add a `produces` reference from the behavior that emits it, or remove the event if it's unused.";
    W008: Warning software,
        "Unimplemented feature",
        "A `feature` entity has no incoming edge from any `behavior`, meaning nothing implements it. Add a behavior that implements the feature, or remove it if it's not planned.";
    W009: Warning testing,
        "Disallowed verify kind",
        "An entity uses a `verify` kind (for example `unit`, `contract`, `integration`) that isn't in the allowed set for its entity kind. Use one of the verify kinds listed as allowed in the diagnostic.";
    W010: Warning software,
        "Unknown field annotation",
        "A `type` field carries an annotation that isn't recognized by the compiler. Remove the annotation or correct its spelling.";
    W012: Warning core,
        "Unreferenced ref entity",
        "A `ref` entity has no incoming edges, meaning nothing in the project references it. Reference the `ref` from another entity, or remove it if it's unused.";
    W017: Warning core,
        "Testable kind lacks verify support",
        "An extension registers an entity kind as `testable` but does not set `supportsVerify: true`, so verify statements can't be declared on it. Set `supportsVerify: true` in the extension manifest.";
    W018: Warning core,
        "Duplicate edge type",
        "Two extensions register an edge type with the same label; the first-registered extension's definition wins and the later one is ignored. Rename one of the conflicting edge types to avoid the collision.";
    W019: Warning core,
        "Unknown field type",
        "An extension manifest declares a field whose `field_type` value the compiler doesn't recognize (it must be one of `string`, `integer`, `bool`, `date`, `enum`, `enum_list`, `identifier`, `string_list`, `reference`, `reference_list`, `block` or `block_list`); a member of a block or block list is a field and is read the same way, and one with an unknown type is dropped. Correct the field's `field_type` in the manifest.";
    W020: Warning core,
        "Unrecognized field",
        "An entity sets a field that isn't declared for its kind by any installed extension, or a typed block or block list item sets a key its field declares no member for. Remove it, fix a typo in its name, or install the extension that declares it.";
    W021: Warning core,
        "Undeclared target kind or edge label",
        "A field, edge type or validation rule references a `target_kind` or edge label that isn't declared — either in the extension's own manifest when it declares no peer dependencies, or in the compiler's global kind/edge registry once all extensions are loaded. Declare the missing kind or edge label, or add the appropriate peer dependency (a validation rule on another extension's kind that this one works without names that extension as its `target_extension` instead). The same code reports a declaration the registry refuses: a `derived_from` that derives nothing, a `proof_role` other than `bound` or `claim`, a `scoped_by` that is not one of the kind's single reference fields with a target kind, targets a kind no extension loads or the kind itself, or closes a scope cycle (the kind is then unscoped), an `enum` field that declares no values, or a `default_value` an enum field does not take. A field that writes a declared edge type from a kind outside its declared `source_kind`, or to a kind outside its declared `target_kind`, is W021 too, and its edges are of no edge type. So is a typed field the registry refuses and drops the attribute of: `members` on a type other than `block` or `block_list`, a `block_list` with none, a member named twice or not one identifier, a member that declares `derived_from`, `target_kinds` together with `target_kind`, on a type other than `reference` or `reference_list`, or naming one kind, a `minimum` or `maximum` on a type other than `integer` or a `minimum` above the `maximum`, and a `file_base` other than `spec` or `project`, or without `file_reference`; a `scoped_by` field with `target_kinds` scopes nothing. Who writes a field is checked too (ADR 0287): a `writer` other than `computed`, `harness`, `human` or `verdict`, `harness` or `verdict` on a member, a computed or verdict field that is also `required`, `immutable`, `append_only` or `derived_from`, `append_only` on a field that is not a list, a computed field no compute pass lists in `computes`, a `computes` entry that is not a computed field of the extension or that two passes list or whose pass is not in the `compute` phase, and a `baseline` naming a kind no loaded extension declares. Who writes a kind is checked as well (ADR 0289): a kind `writer` other than `human`, a `human` field marked `required` on a kind that is not human-only (an agent could then create no entity of the kind), and a `guard` pass that declares `computes`. The `settings` category is checked too (ADR 0288): a setting named twice or not a dotted name or a family (`*` segments), of a type the field forms do not have, with a default its type refuses, with `gates` on a type other than `bool` or on a family (only a setting named whole gates codes), or naming a code the extension does not catalog, or with `keys` that do not match its `*`s; a preset whose source is not an enum or bool setting of the extension or of a loaded peer, that names a value its source does not take, that sets a setting the extension does not declare to a value its type refuses, or that raises a code the extension does not catalog; a secret named twice or whose environment variable is not a name; and an item of a kind the host does not know. The `surfaces` category is checked too (ADR 0291): a prompt whose id is not snake_case or whose export is not `prompt__<id>`, that declares an argument twice or named for something the host owns (`path`, `format`, `help`, `settings`, `secrets`, `host`, `argv`), of a type an argument does not take, a reference argument naming no loaded kind, bounds on an argument that is no integer or a default its type refuses, a gate that is not a code the extension catalogs (a command's gates too), a `subject` that is not an argument it declares, a `when` that is not a bool setting of the extension, or a step that names a command or prompt neither the extension nor a declared peer declares; the prompt or command is dropped whole, since a gate or an argument the host cannot read must not be ignored, and a step's `agent` (until the host knows agents) or an attribution with an empty field is dropped alone. The item is dropped, the rest of the category is kept. The host contributions are checked too (ADR 0292): an agent, skill, slash command or hook the registry refuses and drops, alone — an agent or skill id that is not kebab-case or is declared twice, a hook id that is not snake_case, a tool class outside the host-neutral vocabulary (`read`, `write`, `edit`, `shell`, `search`, `web_fetch`, `web_search`, `ask_user`, `agent`, `skill`, `todo`, `mcp:<server>/<glob>`), a `model` or `effort` outside its tiers that is not a `setting:` naming a string or enum setting of the extension, variants without a `variant_setting` (or the reverse), a `when` that is not a bool setting of the extension, a slash command that names no single prompt or command the extension declares, `tools` or `paths` on a hook of an event that takes none, a `timeout_ms` of 0 or above the ceiling (30 s, or the sandbox's), `writes` on a `statusline` hook, a hook export that is not `hook__<id>`, an attribution with an empty field, and a prompt step whose `agent` is no agent of the extension or of a declared peer. A pass's or command's `reads` that the host would serve empty are W021 too (ADR 0293), the pass or command being kept: a read whose name is no word or is repeated, that names neither or both of `field` and `glob`, a `field` that is not `kind.field` or that no loaded extension declares, a `base` on a glob or other than `spec` or `project`, and a glob that is empty, absolute, climbs with `..`, or has more than four `**` segments.";
    W027: Warning core,
        "Import binding not found",
        "A selective import, `use { A, B } from \"target\"` or `pub use { A, B } from \"target\"`, names a binding the target file does not export: an entity it neither declares nor re-exports with `pub use`. A binding may be a path (`use { 03/D-01 }`). References resolve without `use`, so the binding changes nothing, but it names nothing either; a rename and a delete keep bindings in step with the entities they name (ADR 0233). Correct the binding's name, or remove it.";
    W029: Warning formal,
        "Event never consumed",
        "An `event` entity is produced by one or more behaviors but has no consumer, meaning nothing reacts to it. Add a behavior that consumes the event, or remove the unused production.";
    W030: Warning formal,
        "Abstract behavior unrefined",
        "A `behavior` marked `abstract true` has no concrete refinement — no behavior declares `refines` against it, and no `refinement` entity names it as the abstract entity. Add a concrete behavior with `refines`, or a `refinement` entity naming this behavior as the `abstract_entity`.";
    W031: Warning formal,
        "Refinement chain too deep",
        "A behavior sits in a refinement chain deeper than the maximum allowed depth of 4 layers. Split the refinement chain, or collapse intermediate abstraction layers.";
    W032: Warning formal,
        "Unmitigated retry cycle",
        "`specforge analyze`'s `event_graph_analyze` pass found a `behavior` that consumes an event and produces the same event again, and neither the behavior nor the event declares `sync`, so nothing bounds the retries. Declare a `sync` constraint (a timeout or backoff) on the event or the behavior.";
    W033: Warning formal,
        "Asymmetric port connectivity",
        "`specforge analyze`'s `event_graph_analyze` pass found a `port` that two or more behaviors use (`ports`) where the most-referenced of them has more than three times the incoming edges of the least-referenced, a behavior nothing references counting as one. This is a structural hint that one consumer dominates the port's access, not a fairness check. Review how the behaviors share the port.";
    W034: Warning formal,
        "Unbounded event channel",
        "`specforge analyze`'s `event_graph_analyze` pass found an `event` that a behavior produces and that declares no `sync` constraint, so nothing bounds how many of its messages can accumulate. Declare `sync` on the event: a timeout, a buffer limit or its delivery semantics.";
    W035: Warning formal,
        "Undischarged coverage items",
        "One or more coverage items are not proven under @specforge/testing's coverage rule. An item is an entity that counts toward coverage, as `specforge stats` and `specforge analyze coverage` count it: some obligation has no passing recorded test that names it and no entailed formal claim discharges it, or it declares none. Link a test to each obligation by its text and run `specforge collect`; `specforge analyze coverage` then lists each item that declares no obligation (A001) and, once results are recorded, each unproven obligation (A015).";
    W039: Warning formal,
        "Repeated precondition",
        "`specforge analyze`'s `condition_check` pass found a `behavior` whose `requires` names the same condition more than once. The repeat states nothing its first occurrence does not, so it is redundant. Remove it. Conditions are names and prose, so a precondition implied by a different one is not detected.";
    W040: Warning formal,
        "Invariant without expression",
        "`specforge analyze`'s `condition_check` pass found an `invariant` whose `guarantee` has no `expression`: the guarantee is prose only, so `specforge analyze --prove` has no claim to check. Add an `expression` stating the guarantee as a machine-checkable claim, or keep it prose and prove it with tests.";
    W041: Warning product,
        "Unreferenced feature",
        "A `feature` entity has no incoming edges, meaning no `journey`, `milestone`, or `module` references it. Link it from at least one referencing entity, or remove it if it is no longer needed.";
    W042: Warning product,
        "Unreferenced journey",
        "A `journey` entity has no incoming edges, meaning no `deliverable` references it. Reference the journey from a deliverable's `journeys` field, or remove it if it is unused.";
    W043: Warning product,
        "Deliverable without journeys",
        "A `deliverable` names no journey: no `journeys` field or an empty list (a journey it names that does not exist is E003 alone). Nothing says which user journeys it supports. List the journeys it serves in `journeys`.";
    W044: Warning product,
        "Unreferenced module",
        "A `module` entity has no incoming edges, meaning no `deliverable` or `milestone` references it. Reference the module from a deliverable or milestone, or remove it if it is unused.";
    W045: Warning product,
        "Feature dependency cycle",
        "Two or more `feature` entities form a cycle through their `depends_on` edges. Break the cycle by removing or restructuring one of the `depends_on` references.";
    W046: Warning product,
        "Deliverable without modules",
        "A `deliverable` names no module: no `modules` field or an empty list (a module it names that does not exist is E003 alone), so it has no structural decomposition. List the modules it ships in `modules`.";
    W049: Warning product,
        "Milestone without features",
        "A `milestone` entity has no `features` field, so it may be empty. Modules listed in `modules` don't count: the check reads only `features`. List the features the milestone delivers, or remove the milestone.";
    W053: Warning core,
        "Breaking schema change",
        "A graph protocol schema change classified as breaking since the last export: something a reader of the recorded schema relies on was removed or reshaped — a removed entity kind, edge type or field, a field added as required, a field whose type, required flag, edge type or target kind changed, an enum value lost, or an edge type that lost a source or target kind. `specforge export` compares every attribute of the schema against the one the previous export recorded in `.specforge/schema-cache.json`, bumps the major version, and names the versions in the warning; the export is still written and the cache updated. Update the agents and tools that read the export, or keep the extension versions that produced the old schema.";
    W054: Warning core,
        "Migration structural drift",
        "Comparing the entity graph before and after a migration found entities or edges that appeared, disappeared, or changed unexpectedly. Review the migration logic to ensure it preserves the entities and edges it did not intend to change.";
    W057: Warning product,
        "Missing milestone exit criteria",
        "A `milestone` entity has `status: completed` but no `exit_criteria` field. Add an `exit_criteria` field describing how completion was verified.";
    W060: Warning core,
        "Cross-kind ID collision",
        "The same entity ID is declared with two different entity kinds, either within the same compilation pass or across different files. Entity names share one namespace per scope regardless of kind, so rename one of the conflicting declarations; the first declaration encountered is retained and later ones are skipped.";
    W061: Warning core,
        "Reference cycle detected",
        "The resolved reference graph contains a cycle among entity references. Cycles that share an entity are one warning, whose path walks every entity on them. Break each cycle by removing or inverting one of its references.";
    W092: Warning product,
        "Release dependency cycle",
        "Two or more `release` entities form a cycle through their `depends_on` edges. Break the cycle by removing or restructuring one of the dependency references.";
    W093: Warning product,
        "Invalid release version format",
        "A `release` entity's `version` field does not match semver format (e.g. `1.0.0`). Set `version` to a valid semver string.";
    W096: Warning formal,
        "Behavior requires without ensures",
        "A `behavior` entity declares a `requires` clause (an obligation on callers) but no `ensures` clause (a guarantee in return). Add an `ensures` clause describing what the behavior guarantees when its `requires` is satisfied.";
    W097: Warning core,
        "Test record for an unknown entity",
        "`specforge analyze` read a recorded test result (in `specforge-report.json`) for an entity ID that no spec declares, so the result counts toward nothing; a `did you mean` hint names a close match when there is one. Fix the entity ID the test names, or declare the entity.";
    W098: Warning core,
        "SMT solver unavailable",
        "`specforge analyze --prove` could not ask the SMT solver z3 everything it had to: z3 is not on `PATH` or does not run (`specforge doctor` states the same warning as a toolchain finding), a solve ran past the timeout (`--prove-timeout`, 30 seconds by default), or z3 failed mid-run. The checks it could not answer were skipped: no bound was found contradictory and no claim was proved by them. Install z3 (https://github.com/Z3Prover/z3) and make sure `z3 --version` runs.";
    W110: Warning formal,
        "Refines non-abstract behavior",
        "A behavior's `refines` field names a target behavior that is not marked `abstract true`. Add `abstract true` to the target behavior, or point `refines` at a behavior that is actually abstract.";
    W112: Warning core,
        "Validation rule cannot fire",
        "An extension-declared validation rule cannot work as declared: its `check` kind is unrecognized, it is missing a field or constraint its check needs (a `cycle_detection` rule needs an `edge_type`, a `verify_kind_allowlist` rule a constraint with values; a rule that reads `verify` statements needs a target kind that accepts them), its values list is empty, its `matches` regex does not compile, or its `wasm_function` is absent or failed a probe call. A `judged` rule (ADR 0290) is W112 when its `judged` declaration is absent or sits on another check, its judge, outcomes or passing outcomes are not words (`passing` must be a non-empty subset of `outcomes`), a target kind is not loaded or is listed twice, an input name repeats or is `cited`, a selector does not parse or does not resolve (a field the kind does not declare, one that is not a reference, a computed or verdict field, a `file(...)` of a path that leaves the project, holds a glob or is a `.spec` source, or of a field that is not a file field), an `unless_set` field the kind does not declare, a `values` or `outcome_fields` entry that is not a `verdict`-written field of a target kind of the extension, its judge is no agent of its extension or of a declared peer (ADR 0292), is `prompt:<id>` of a prompt its extension does not declare (a peer's prompt is not a judge, ADR 0295), or the code is another judged rule's. Fix or remove the rule in the extension's manifest.";
    W113: Warning core,
        "Circular file import",
        "Two or more `.spec` files import each other, forming a cycle in the import graph. Break the cycle by removing one of the `use` imports or extracting the shared entities into a separate file.";
    W115: Warning core,
        "Invalid collector report",
        "A collector reported tests for an entity ID that no spec declares (usually a renamed entity or a typo in the test's annotation). `specforge collect` drops those results; fix the test annotation so it names a declared entity.";
    W118: Warning core,
        "Invalid provider configuration",
        "A `providers` entry in `specforge.json` is missing its `alias`/`name` or `scheme` field, or no installed extension contributes providers to back a configured provider. Add the missing field, or install an extension that contributes the provider.";
    W123: Warning formal,
        "Unreferenced property",
        "A `property` that no behavior names in `satisfies`, so nothing claims to keep it. The rule applies where a loaded extension declares behaviors (`@specforge/software`). Name the property in the `satisfies` of the behaviors that keep it, or remove it.";
    W124: Warning formal,
        "Empty property description",
        "A `property` entity writes a `description` that is empty or only whitespace, which makes the temporal assertion opaque to readers and agents. Write the description, or remove the field. A property that writes no description is not reported.";
    W126: Warning formal,
        "Unreferenced axiom",
        "An `axiom` entity is not referenced by any other entity, so it may be unused. Reference the axiom from a relevant entity, or remove it if it is no longer needed.";
    W127: Warning formal,
        "Empty axiom description",
        "An `axiom` entity writes a `description` that is empty or only whitespace, which leaves the assumption unexplained. Write the description, or remove the field. An axiom that writes no description is not reported.";
    W128: Warning formal,
        "Unreferenced protocol",
        "A `protocol` that no event names in `follows_protocol`, so nothing follows it. The rule applies where a loaded extension declares events (`@specforge/software`). Name the protocol in the `follows_protocol` of the events that follow it, or remove it.";
    W129: Warning formal,
        "Empty protocol description",
        "A `protocol` entity writes a `description` that is empty or only whitespace, which makes the synchronization contract opaque. Write the description, or remove the field. A protocol that writes no description is not reported.";
    W131: Warning formal,
        "Unreferenced refinement",
        "A `refinement` entity is not referenced by any other entity, so it may be unused. Reference the refinement from the entity it refines, or remove it if it is no longer needed.";
    W132: Warning formal,
        "Empty refinement description",
        "A `refinement` entity writes a `description` that is empty or only whitespace, which makes the abstract-to-concrete mapping opaque. Write the description, or remove the field. A refinement that writes no description is not reported.";
    W133: Warning formal,
        "Refinement without invariant deltas",
        "A `refinement` entity declares no `invariant_deltas` (the field is absent or an empty list), so it records no change between its abstract and concrete behaviors. List the invariants the refinement adds or relaxes in `invariant_deltas`.";
    W134: Warning formal,
        "Unreferenced process",
        "A `process` entity is not referenced by any other entity, so it may be unused. Reference the process from a relevant entity, or remove it if it is no longer needed.";
    W135: Warning formal,
        "Empty process description",
        "A `process` entity writes a `description` that is empty or only whitespace, which makes the communicating process opaque. Write the description, or remove the field. A process that writes no description is not reported.";
    W136: Warning formal,
        "Empty process alphabet",
        "A `process` entity writes an empty `alphabet` (`[]`), so it declares no events it can engage in. List the events in its alphabet. An absent `alphabet` is E006, since the field is required.";
    W137: Warning core,
        "Ambiguous convention mapping",
        "`specforge collect` links a test that no annotation links by its name: `entity_id__obligation_slug`, or a module named after an entity. This test's name splits at more than one `__` into a declared entity ID (entity IDs aren't meant to contain `__`), so it isn't linked. Rename the test or the entity, or link the test explicitly (`#[specforge_test]`).";
    W138: Warning core,
        "Unknown describe key",
        "An item of an extension's describe answer has a key the extension protocol doesn't define, so the host ignores it. It's usually a misspelling in a hand-written (`raw_category`) item, `testabel` for `testable`, which leaves the item without what the key was meant to declare; or the extension was built with a newer SDK than this host. The message names the extension, the category, the item and the key. Fix the spelling, or update SpecForge.";
    W139: Warning core,
        "Formal claim not entailed by declared bounds",
        "`specforge analyze --prove` found that a declared claim (a field an extension gives the `claim` proof role, such as a formal property's or invariant's `expression`) isn't guaranteed by the declared bounds: the SMT solver found a counterexample that satisfies every bound while violating the claim. The claim isn't wrong; the bounds just don't guarantee it yet, and its `verify property` obligation stays unproven. Strengthen the declared bounds or weaken the claim. Use `--strict` to fail the run on it. This code was E047 until it was renumbered to match its severity.";
    W140: Warning core,
        "Duplicate registry alias",
        "Two entries in `registries` in `specforge.json` share an alias, so the alias doesn't name one registry. Give each registry a unique alias.";
    W141: Warning core,
        "Invalid formatter configuration",
        "The formatter's `.specforgefmt.toml` can't be read, isn't valid TOML, or sets `indent_width` (an integer from 1 to 16), `use_tabs` (a boolean) or `max_width` (an integer from 40 to 200) to an invalid value. The formatter uses the default for anything it can't use. Fix the reported setting.";
    W142: Warning core,
        "Unparseable region left unformatted",
        "The formatter hit a parse error in a `.spec` file. It keeps the reported line range exactly as written and formats the rest. Fix the syntax there (see E001) and format again.";
    W143: Warning core,
        "Define blocks are not supported",
        "A `.spec` file has a `define <name> { ... }` block. Every entity kind comes from an extension, so a project's kinds depend only on `specforge.json` (ADR 0005): the block registers nothing and is left out of the graph. Declare the kind in an extension (`specforge extension init`) and enable it, then remove the block. `define` stays a reserved word.";
    W145: Warning core,
        "Pass order constraints form a cycle",
        "An extension's compiler passes declare `after`/`before` constraints that form a cycle, so no order satisfies them all. The registry build reports it, naming the passes in the cycle, and runs that extension's passes in the order it declares them. Remove the constraint that closes the cycle.";
    W147: Warning core,
        "Validation rule property ignored",
        "An extension-declared validation rule sets a property its check does not read — an `edge_type` on a field check, a `constraint` on an edge check, a `wasm_function` on a declarative check, a constraint kind, `pattern` or `values` its check does not read, or a `target_kind` or `field` on a `judged` rule (it judges the kinds its `judged` names). The rule is registered without it, so it does not do what its author meant. Remove the property, or use the check that reads it (`conditional_field_required` reads `constraint.pattern` as the condition field's name, `field_value_constraint` with `matches` as a regex).";
    W148: Warning core,
        "Custom rule could not check entities",
        "A `check: \"custom\"` rule's `wasm_function` failed (trapped, or answered something that is not a verdict) on some entities during this check, so they were not checked. Reported once per rule, with how many failed and the first one's error. The load-time probe (W112) calls the function on an empty entity only; fix the function so that it answers every entity of the rule's target kind. The diagnostic's data lists every entity that was not checked, with its error.";
    W150: Warning core,
        "Extension reports a code it may not use",
        "An extension declared a validation rule, or a pass reported a diagnostic, with a code it may not use. A first-party extension may report only the codes this catalog gives it, at the catalogued level: a code the catalog gives to core or to another extension, a retired code and an uncatalogued one are refused. Any other extension reports a local code, `E`, `W` or `I` and three digits, that its own `codes` catalog lists, at the level its letter states: a code it did not catalogue, a code that holds a `/` or is not three digits, and a code whose letter contradicts the level reported are refused. The rule still runs and the finding is still reported, under the extension's namespace (`acme.lint/W001`), so its title and docs link never describe another diagnostic. Catalogue the code in the extension's `codes` (`c.code` in the SDK), at its letter's level.";
    W151: Warning core,
        "Entity kinds left unchecked",
        "Extensions are loaded, but none of them declares an entity kind, so the entities' kinds, fields and identifiers are not checked: no E024 for an unknown kind, no W020, E013, E014, E022 or E061. The warning names how many entities that leaves unchecked and their kinds. Enable the extension that declares those kinds (the suggestion names it when it is a builtin), or remove the entities. A project with no extension at all is structural-only on purpose and gets I002 instead.";
    W153: Warning core,
        "Sandbox declaration not honoured",
        "An extension's declaration asks its sandbox for something the host does not give, so the extension runs without it. A component is granted no capability, whatever it declares: no directory, environment, arguments, stdin, socket or name lookup (ADR 0037). So a `sandbox_policy` key other than `max_execution_ms` and `max_memory_mb` that asks for something (`network_access: true`, a non-empty `allowed_paths`, a misspelled limit), and a surface's `sandbox` override, grant nothing. A declared limit above the host's ceiling (30000 ms per call, 512 MB of linear memory) is held to the ceiling. The message names the extension and the key. Remove the key, or lower the limit. An extension that needs a file's content gets it in its input (an analyzer is handed each file); one that needs the network can't run in SpecForge.";
    W154: Warning product,
        "Completed milestone delivers an unfinished feature",
        "A `milestone` has `status: completed` but a feature its `features` names is neither `done` nor `deprecated`, so the milestone's completion claims what its features do not. Reported once per such feature, by the `lifecycle` pass of @specforge/product. Mark the feature done if it was delivered; otherwise move it out of the milestone or set the milestone back to `in_progress`.";
    W155: Warning core,
        "Unsigned package accepted",
        "`specforge add` or `specforge update` accepted a registry package that carries no publisher signature, because `--allow-unsigned` was given; it is reported when the package is fetched, whether or not the install then goes ahead. Where it came from can't be verified, and no publisher key is pinned for it, so a later signed version is pinned on first use. Install signed packages where you can; ask the publisher to sign with `specforge publish`.";
    W156: Warning core,
        "Publisher key re-pinned",
        "A registry package is signed with a different key than the one pinned for it, the change was accepted (`--yes`, a yes at the prompt, or the new key on `trusted_keys`), and the install was written, so the new key is now pinned. It is reported when the pin is saved: an install that is refused or fails pins nothing and reports none. That is what a key rotation looks like, and also what a compromised publisher looks like: confirm the change with the publisher if you did not expect it. The message names both key ids.";
    W158: Warning core,
        "Verify statements on a kind that accepts none",
        "An entity declares `verify` statements, but its kind accepts none: no extension made the kind testable (`supports_verify`). `verify` is reserved syntax; what an obligation means comes from an extension, so a kind takes statements only once one enables them (ADR 0002, ADR 0094). The statements are not fields, so this is not W020. Enable an extension that makes the kind testable (for software kinds, `specforge add @specforge/testing`), or remove the statements.";
    W159: Warning core,
        "Value not written in its field's form",
        "A field's declared type decides how its value is written (ADR 0147): bare for an enum, identifier or reference value and each item of a reference list; quoted for text and each item of a string list; an integer, true or false and a calendar day bare; a list field's value in brackets, one item per value. A multi-line text on a list field is one item: split it, format cannot. This value is written the other way. It reads the same, so the graph is unchanged; run specforge format (or format the document in the editor) to rewrite every value in its form. A bare word on a kind that takes members (type, port) is left alone: it may be a member's type. A value a `defaults <kind>` block writes is reported once, at the default, however many blocks take it.";
    W161: Warning core,
        "Default that changes nothing",
        "Either a block writes a field with exactly its file's default value, so the line restates the default, or a `defaults <kind>` field reaches no block: every block of that kind in the file writes its own value, or the file declares none. Delete the line. A block that must keep its value whatever the default becomes may write it; a different value replaces the default whole (ADR 0149).";
    W162: Warning core,
        "Relation declared from both sides",
        "A relation has two fields: one extension declares it from both of its ends. Either a reference field's `inverse_of` names a field its target kind registers (`milestone.behaviors` naming `features`, when `behavior` has its own `features`), or two different kinds each register a reference field targeting the other (`parent.children` and `child.parent`). A relation is written from one side (ADR 0150): the other side is derived, so `query` and `trace` already show it. Both fields still register, and an entity can write either. Remove one field, or drop the `inverse_of` or name a word the target kind does not use. The builtin extensions declare none.";
    W163: Warning core,
        "Malformed harness ledger line",
        "A line of `specforge-harness` is not `path key digest` with a SHA-256 digest, or the file does not start with `# specforge-harness 1` (ADR 0287). git's conflict markers are such lines when the file's `merge=union` attribute is missing. The line is ignored; with a bad header the whole ledger is read as empty, so every harness value is E083. Run `specforge harness gc` to write the file again from what it records, or restore it from git.";
    W164: Warning core,
        "Baseline cannot be read",
        "The project declares a history flag (ADR 0287), and its baseline, the project at the commit `HEAD` names, cannot be read: git is not installed or does not answer in time, refuses the directory (dubious ownership), or cannot give the files at that commit (a shallow or partial clone). Nothing is judged against history, and the message says why. Fix what it names, then check again.";
    W165: Warning core,
        "Computed value refused",
        "A compute pass answered a value the host does not take, once per value: for an entity the project does not hold, for a field the pass does not list in its `computes`, for a member of a block item that is not written, or in a form the field refuses (the field's own type, bounds and enum values apply). The value is dropped. A check or analyze pass that answers `values` is W165 once, and its values are ignored: only a pass in the `compute` phase gives values.";
    W166: Warning core,
        "Settings for an extension that is not enabled",
        "`settings` in `specforge.json` holds an object for an extension that no entry of `extensions` enables (ADR 0288), so nothing reads it, as W118 says of a provider. Add the extension to `extensions` (`specforge add <name>`), or remove its settings. An enabled extension that failed to load keeps its settings unchecked and reports its own failure (E028, E070).";
    W167: Warning core,
        "Interrupted write pending rollback",
        "`.specforge/write-journal.json` exists (ADR 0289): a write of the project's files was interrupted after it recorded what it would replace and before it finished, so some of its files may hold part of it. The journal is the write's record, kept under the write lock: the next SpecForge write puts back each file that still holds what the interrupted one wrote, leaves a file that holds what it replaced, and names a file that holds neither, then removes the journal. A check never rolls it back, so it reports this once until a writer does. Run a write again, or `specforge harness gc`, to roll it back; then re-run the command that was interrupted, since its plan is stale.";
    W168: Warning core,
        "Verdict cannot be read",
        "An entity's `verdict` statement (ADR 0290) names a rule that is not a loaded judged rule, a rule that does not judge the entity's kind, an outcome the rule does not declare, a member missing or of the wrong form (`judge`, `model`, `at` and `inputs` are required), `values` for a field the rule does not bind or that the field's form refuses, or a second verdict of one rule. It counts as no verdict, so the rule reports the entity as missing. Record a verdict through `set_verdict`, or remove the statement.";
}
