use std::fs;
use std::path::Path;

use specforge_project::CompiledProject;
use specforge_test::prelude::*;
use tempfile::TempDir;

fn project(config: serde_json::Value, files: &[(&str, &str)]) -> TempDir {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("specforge.json"), config.to_string()).unwrap();
    for (path, text) in files {
        let path = dir.path().join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }
    dir
}

fn compile(root: &Path) -> CompiledProject {
    let runtime = specforge_component::project_runtime(root);
    CompiledProject::compile(root, Some(&runtime))
}

fn codes(diagnostics: &[specforge_common::Diagnostic]) -> Vec<&str> {
    diagnostics.iter().map(|d| d.code.as_str()).collect()
}

/// An extension that fails to load, a missing import and a duplicate ID:
/// one compile reports all three, each from its own layer, in `check`'s
/// order (environment, resolver, graph).
#[specforge_test(
    invariant = "multi_error_collection",
    verify = "the compiler does not halt after the first error"
)]
fn one_compile_reports_every_layers_errors() {
    let dir = project(
        serde_json::json!({
            "name": "p", "version": "0.1.0",
            "extensions": ["@specforge/software", "@specforge/no-such-extension"]
        }),
        &[
            ("a.spec", "use \"missing\"\n\nterm alpha \"Alpha\" {\n}\n"),
            ("b.spec", "term alpha \"Alpha again\" {\n}\n"),
        ],
    );

    let compiled = compile(dir.path());
    let diagnostics = compiled.diagnostics();
    let codes = codes(&diagnostics);
    let position = |code: &str| {
        codes
            .iter()
            .position(|c| *c == code)
            .unwrap_or_else(|| panic!("no {code} in {codes:?}"))
    };
    assert!(position("E028") < position("E025"), "{codes:?}");
    assert!(position("E025") < position("E002"), "{codes:?}");
}

/// The flat view carries the same diagnostics, and the spec root and
/// resolved files the compile read (MCP needs both: plan 01, D8).
#[test]
fn the_context_view_keeps_the_spec_root_and_the_resolved_files() {
    let dir = project(
        serde_json::json!({
            "name": "p", "version": "0.1.0",
            "extensions": ["@specforge/software"], "spec_root": "spec"
        }),
        &[("spec/a.spec", "term alpha \"Alpha\" {\n}\n")],
    );

    let compiled = compile(dir.path());
    let diagnostics = compiled.diagnostics();
    let ctx = compiled.into_context();

    assert_eq!(ctx.diagnostics, diagnostics);
    assert_eq!(ctx.spec_root, dir.path().join("spec"));
    let files: Vec<&str> = ctx.resolved.files.iter().map(|f| f.path.as_str()).collect();
    assert_eq!(files, ["a.spec"]);
}

/// No extension configured: the compile still runs, structurally, and
/// says so once, with an I002 info.
#[specforge_test(
    behavior = "graceful_degradation_without_extensions",
    verify = "no extensions installed emits I002 info"
)]
fn a_project_without_extensions_reports_structural_only_mode() {
    let dir = project(
        serde_json::json!({ "name": "p", "version": "0.1.0" }),
        &[(
            "a.spec",
            "thing alpha \"A\" {\n  refs [beta]\n}\n\nthing beta \"B\" {\n}\n",
        )],
    );

    let diagnostics = compile(dir.path()).diagnostics();

    assert_eq!(codes(&diagnostics), ["I002"], "{diagnostics:?}");
    assert_eq!(diagnostics[0].severity, specforge_common::Severity::Info);
    assert!(
        diagnostics[0].message.contains("no extensions configured"),
        "{diagnostics:?}"
    );
}

#[specforge_test(
    behavior = "graceful_degradation_without_extensions",
    verify = "Graceful Degradation Without Extensions: graceful degradation holds — registries_populated_fired, i002_emitted, structural_mode_operational, valid_export_produced"
)]
fn graceful_degradation_contract() {
    let dir = project(
        serde_json::json!({ "name": "p", "version": "0.1.0" }),
        &[(
            "a.spec",
            "thing alpha \"A\" {\n  refs [beta]\n}\n\nthing beta \"B\" {\n}\n",
        )],
    );

    let compiled = compile(dir.path());

    // registries_populated_fired: with no extension, the registries are
    // built, and empty.
    assert!(compiled.env.registries.kinds.is_empty());
    // i002_emitted
    assert_eq!(codes(&compiled.diagnostics()), ["I002"]);
    // structural_mode_operational: generic nodes, raw keywords, a
    // reference edge.
    let alpha = compiled.graph.node("alpha").unwrap();
    assert_eq!(alpha.kind.raw.as_str(), "thing");
    assert_eq!(compiled.graph.edges_from("alpha").len(), 1);
    // valid_export_produced
    let json = specforge_emitter::json::emit_json(&compiled.graph);
    let exported: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(exported["nodes"].as_array().unwrap().len(), 2);
}

/// Every configured extension fails to load: one E028 each, then the
/// I002 that says the compile went on structurally.
#[specforge_test(
    behavior = "handle_all_extensions_failed_to_load",
    verify = "all extensions failing produces per-extension E-level diagnostics"
)]
fn every_failed_extension_gets_its_own_error() {
    let dir = project(
        serde_json::json!({
            "name": "p", "version": "0.1.0",
            "extensions": ["@specforge/no-such-a", "@specforge/no-such-b"]
        }),
        &[("a.spec", "thing alpha \"A\" {\n}\n")],
    );

    let diagnostics = compile(dir.path()).diagnostics();

    let errors: Vec<&str> = diagnostics
        .iter()
        .filter(|d| d.code == "E028")
        .map(|d| d.message.as_str())
        .collect();
    assert_eq!(errors.len(), 2, "{diagnostics:?}");
    assert!(errors[0].contains("@specforge/no-such-a"), "{errors:?}");
    assert!(errors[1].contains("@specforge/no-such-b"), "{errors:?}");
}

#[specforge_test(
    behavior = "handle_all_extensions_failed_to_load",
    verify = "system transitions to structural-only mode after all failures"
)]
fn all_failed_extensions_leave_a_structural_compile() {
    let dir = project(
        serde_json::json!({
            "name": "p", "version": "0.1.0",
            "extensions": ["@specforge/no-such-a", "@specforge/no-such-b"]
        }),
        &[(
            "a.spec",
            "thing alpha \"A\" {\n  refs [beta]\n}\n\nthing beta \"B\" {\n}\n",
        )],
    );

    let compiled = compile(dir.path());
    let diagnostics = compiled.diagnostics();

    // The graph is still built, and nothing kind-specific is reported.
    assert_eq!(compiled.graph.node_count(), 2);
    assert_eq!(
        codes(&diagnostics),
        ["E028", "E028", "I002"],
        "{diagnostics:?}"
    );
    assert!(
        diagnostics[2]
            .message
            .contains("none of the 2 configured extensions loaded"),
        "{diagnostics:?}"
    );
}

/// `@specforge/formal` requires `@specforge/software`: without it the
/// compile fails with E027, which says what to install.
#[specforge_test(
    invariant = "peer_dependency_satisfaction",
    verify = "unsatisfied peer dependency produces an error diagnostic"
)]
fn a_missing_required_peer_fails_the_compile() {
    let dir = project(
        serde_json::json!({
            "name": "p", "version": "0.1.0",
            "extensions": ["@specforge/formal"]
        }),
        &[("a.spec", "term alpha \"Alpha\" {\n}\n")],
    );

    let diagnostics = compile(dir.path()).diagnostics();

    let e027: Vec<_> = diagnostics.iter().filter(|d| d.code == "E027").collect();
    assert_eq!(e027.len(), 1, "{diagnostics:?}");
    assert!(
        e027[0].message.contains("'@specforge/formal'")
            && e027[0].message.contains("'@specforge/software'"),
        "{e027:?}"
    );
    assert_eq!(
        e027[0].suggestion.as_deref(),
        Some("install it with: specforge add @specforge/software")
    );
}

/// Optional peers may be absent: software (optional product) and testing
/// (optional software, governance) load alone without a peer error.
#[specforge_test(
    invariant = "peer_dependency_satisfaction",
    verify = "satisfied peer dependencies pass validation"
)]
fn missing_optional_peers_are_not_reported() {
    for extension in ["@specforge/software", "@specforge/testing"] {
        let dir = project(
            serde_json::json!({
                "name": "p", "version": "0.1.0", "extensions": [extension]
            }),
            &[("a.spec", "spec \"p\" {\n}\n")],
        );

        let diagnostics = compile(dir.path()).diagnostics();

        assert!(
            !codes(&diagnostics).contains(&"E027"),
            "{extension}: {diagnostics:?}"
        );
    }
}

/// `feature` belongs to @specforge/product, which the project doesn't
/// enable: one E024 whose suggestion names product, and nothing else about
/// that keyword.
#[specforge_test(
    behavior = "resolve_soft_cross_extension_references",
    verify = "unknown keyword matching known extension gets E024 naming the extension"
)]
fn a_known_keyword_without_its_extension_is_one_e024_naming_it() {
    let dir = project(
        serde_json::json!({
            "name": "p", "version": "0.1.0", "extensions": ["@specforge/software"]
        }),
        &[("a.spec", "feature checkout \"Checkout\" {\n}\n")],
    );

    let diagnostics = compile(dir.path()).diagnostics();

    // (software's own rules may add their warnings about the feature.)
    let e024: Vec<_> = diagnostics.iter().filter(|d| d.code == "E024").collect();
    assert_eq!(e024.len(), 1, "{diagnostics:?}");
    assert_eq!(
        e024[0].suggestion.as_deref(),
        Some("install it with: specforge add @specforge/product")
    );
    assert!(!codes(&diagnostics).contains(&"I004"), "{diagnostics:?}");
}

#[specforge_test(
    behavior = "resolve_soft_cross_extension_references",
    verify = "Resolve Soft Cross-Extension References: soft cross-extension resolution holds — registries_populated_fired, known_extensions_catalog_available, suggestion_emitted, installed_extensions_resolved"
)]
fn soft_cross_extension_resolution_contract() {
    let dir = project(
        serde_json::json!({
            "name": "p", "version": "0.1.0", "extensions": ["@specforge/software"]
        }),
        &[(
            "a.spec",
            "feature checkout \"Checkout\" {\n}\n\ninvariant alpha \"Alpha\" {\n  refs [missing]\n}\n",
        )],
    );

    let compiled = compile(dir.path());
    let diagnostics = compiled.diagnostics();

    // registries_populated_fired: software's kinds are registered.
    assert!(compiled.env.registries.kinds.contains("invariant"));
    // known_extensions_catalog_available, suggestion_emitted: the unknown
    // keyword's E024 names its extension, once.
    let e024: Vec<_> = diagnostics.iter().filter(|d| d.code == "E024").collect();
    assert_eq!(e024.len(), 1, "{diagnostics:?}");
    assert!(
        e024[0]
            .suggestion
            .as_deref()
            .unwrap()
            .contains("@specforge/product")
    );
    assert!(!codes(&diagnostics).contains(&"I004"), "{diagnostics:?}");
    // installed_extensions_resolved: an installed kind's broken reference
    // is a plain E003.
    assert!(
        diagnostics
            .iter()
            .any(|d| d.code == "E003" && d.message.contains("'missing'")),
        "{diagnostics:?}"
    );
}

/// Define blocks are not supported: each is one W143, and none reaches the
/// graph, so its body is not read as references (no E003) and its name is
/// not checked as an ID (no E013 for `define behavior`).
#[specforge_test(
    behavior = "report_define_blocks",
    verify = "a define block produces one W143 and adds no node to the graph"
)]
fn a_define_block_is_one_warning_and_no_node() {
    let dir = project(
        serde_json::json!({
            "name": "p", "version": "0.1.0", "extensions": ["@specforge/software"]
        }),
        &[(
            "a.spec",
            "define user_story {\n  required [title]\n}\n\ndefine behavior {\n}\n",
        )],
    );

    let compiled = compile(dir.path());
    let diagnostics = compiled.diagnostics();

    assert_eq!(codes(&diagnostics), ["W143", "W143"], "{diagnostics:?}");
    assert!(diagnostics[0].message.contains("'user_story'"));
    assert!(diagnostics[1].message.contains("'behavior'"));
    assert!(
        diagnostics[0]
            .suggestion
            .as_deref()
            .unwrap()
            .contains("extension")
    );
    assert_eq!(compiled.graph.node_count(), 0);
}

/// A project with an extension loaded is not in structural-only mode.
#[test]
fn a_loaded_extension_means_no_i002() {
    let dir = project(
        serde_json::json!({
            "name": "p", "version": "0.1.0",
            "extensions": ["@specforge/software", "@specforge/no-such-extension"]
        }),
        &[("a.spec", "term alpha \"Alpha\" {\n}\n")],
    );

    let diagnostics = compile(dir.path()).diagnostics();

    assert!(!codes(&diagnostics).contains(&"I002"), "{diagnostics:?}");
}

/// Provider extensions, in process: each contributes providers (a raw
/// `providers` category, which raises the handshake flag) and nothing
/// else. No builtin contributes providers.
fn provider_extensions(
    names: &'static [&'static str],
) -> specforge_wasm::testing::InProcessRuntime {
    names.iter().fold(
        specforge_wasm::testing::InProcessRuntime::new(),
        |runtime, name| {
            runtime.with(move || {
                let mut b = specforge_extension_sdk::ContributionsBuilder::new(
                    specforge_extension_sdk::ExtensionMeta::new(name, "1.0.0"),
                );
                b.raw_category("providers", serde_json::json!([]));
                b
            })
        },
    )
}

/// The compile's diagnostics, without the W012 every unreferenced ref
/// gets.
fn compile_with_providers(
    root: &Path,
    extensions: &'static [&'static str],
) -> Vec<specforge_common::Diagnostic> {
    CompiledProject::compile(root, Some(&provider_extensions(extensions)))
        .diagnostics()
        .into_iter()
        .filter(|d| d.code != "W012")
        .collect()
}

/// A configured provider registers its scheme: a ref with that scheme is
/// known, and a ref with another scheme is I005.
#[specforge_test(
    behavior = "register_provider_schemes",
    verify = "Wasm-based provider scheme registered and validates ref"
)]
fn a_registered_provider_scheme_validates_refs() {
    let dir = project(
        serde_json::json!({
            "name": "p", "version": "0.1.0",
            "extensions": ["@test/gh-provider"],
            "providers": [
                { "scheme": "gh", "alias": "main", "extension": "@test/gh-provider" }
            ]
        }),
        &[(
            "a.spec",
            "ref gh.issue:42 \"Known scheme\"\nref jira.story:7 \"Unknown scheme\"\n",
        )],
    );

    let diagnostics = compile_with_providers(dir.path(), &["@test/gh-provider"]);

    assert_eq!(codes(&diagnostics), ["I005"], "{diagnostics:?}");
    assert!(diagnostics[0].message.contains("'jira'"), "{diagnostics:?}");
}

/// Two providers declaring one scheme: E057 reaches `check`, and the first
/// declared keeps the scheme.
#[specforge_test(
    behavior = "register_provider_schemes",
    verify = "duplicate scheme from two providers produces E057"
)]
fn a_scheme_declared_twice_is_e057_on_compile() {
    let dir = project(
        serde_json::json!({
            "name": "p", "version": "0.1.0",
            "extensions": ["@test/gh-a", "@test/gh-b"],
            "providers": [
                { "scheme": "gh", "alias": "a", "extension": "@test/gh-a" },
                { "scheme": "gh", "alias": "b", "extension": "@test/gh-b" }
            ]
        }),
        &[("a.spec", "ref gh.issue:42 \"Known scheme\"\n")],
    );

    let diagnostics = compile_with_providers(dir.path(), &["@test/gh-a", "@test/gh-b"]);

    assert_eq!(codes(&diagnostics), ["E057"], "{diagnostics:?}");
    assert!(
        diagnostics[0]
            .message
            .contains("already registered by provider 'a'"),
        "{diagnostics:?}"
    );
}

/// Without a provider configured, refs are not checked for their scheme.
#[test]
fn no_provider_means_no_scheme_check() {
    let dir = project(
        serde_json::json!({
            "name": "p", "version": "0.1.0", "extensions": ["@test/gh-provider"]
        }),
        &[("a.spec", "ref jira.story:7 \"Unknown scheme\"\n")],
    );

    let diagnostics = compile_with_providers(dir.path(), &["@test/gh-provider"]);

    assert!(diagnostics.is_empty(), "{diagnostics:?}");
}

/// A compile keeps the text it parsed: a file rewritten on disk after the
/// compile is still quoted as it was compiled.
#[test]
fn source_texts_are_what_was_compiled() {
    let compiled_text = "behavior alpha \"Alpha\" {\n  contract \"MUST work\"\n}\n";
    let dir = project(
        serde_json::json!({"name": "p", "version": "0.1.0", "extensions": []}),
        &[("a.spec", compiled_text), ("sub/b.spec", "// b\n")],
    );
    let compiled = CompiledProject::compile(dir.path(), None);
    fs::write(dir.path().join("a.spec"), "// rewritten\n").unwrap();

    let texts = compiled.resolved.source_texts();
    assert_eq!(texts["a.spec"], compiled_text);
    assert_eq!(texts["sub/b.spec"], "// b\n");
    assert_eq!(texts.len(), 2, "{texts:?}");
}

/// Characterization (plan 03 T0): the order `Environment::diagnostics()`
/// reports a missing extension's load failure, the loaded extensions'
/// declaration diagnostics and the registry build's. The registry build
/// owning declaration validation changes it (ADR 0012).
#[test]
fn environment_diagnostics_come_in_load_order() {
    let dir = project(
        serde_json::json!({
            "name": "p", "version": "0.1.0",
            "extensions": ["@specforge/formal", "@acme/missing", "@specforge/product"]
        }),
        &[],
    );
    let runtime = specforge_component::project_runtime(dir.path());
    let env = specforge_project::Environment::load(dir.path(), Some(&runtime));
    let codes: Vec<&str> = env.diagnostics().map(|d| d.code.as_str()).collect();
    // The missing extension's load failure, formal's missing peer, then the
    // registry build's: formal's enhancements of software's kinds.
    assert_eq!(
        codes,
        ["E028", "E027", "I004", "I004", "I004"],
        "{:#?}",
        env.diagnostics().collect::<Vec<_>>()
    );
}

mod declared_in_process {
    //! Extensions declared with the SDK and served in process: what they
    //! declare is what the environment loads.

    use super::project;
    use specforge_extension_sdk::prelude::*;
    use specforge_project::Environment;
    use specforge_registry::SurfaceType;
    use specforge_wasm::testing::InProcessRuntime;

    fn reports() -> ContributionsBuilder {
        let mut meta = ExtensionMeta::new("@acme/reports", "0.1.0");
        meta.short = Some("rep".to_string());
        let mut b = ContributionsBuilder::new(meta);
        b.kind("report", |k| {
            k.description("A report");
        });
        b.command("list", |c| {
            c.title("List")
                .description("List reports")
                .handler(|_| CommandOutput {
                    exit_code: 0,
                    stdout: String::new(),
                    stderr: String::new(),
                });
        });
        b
    }

    fn commands_only() -> ContributionsBuilder {
        let mut b = ContributionsBuilder::new(ExtensionMeta::new("@acme/cmds", "0.1.0"));
        b.command("hello", |c| {
            c.title("Hello")
                .description("Say hello")
                .handler(|_| CommandOutput {
                    exit_code: 0,
                    stdout: "hi".to_string(),
                    stderr: String::new(),
                });
        });
        b
    }

    fn load(extensions: &[&str], runtime: &InProcessRuntime) -> Environment {
        let dir = project(
            serde_json::json!({ "name": "p", "version": "0.1.0", "extensions": extensions }),
            &[],
        );
        Environment::load(dir.path(), Some(runtime))
    }

    #[specforge_test_macros::test(
        behavior = "load_extension_declaration",
        verify = "an extension that only declares commands registers its commands"
    )]
    fn a_commands_only_extension_registers_its_commands() {
        let runtime = InProcessRuntime::new().with(commands_only);
        let env = load(&["@acme/cmds"], &runtime);
        let registered: Vec<(&SurfaceType, &str)> = env
            .registries
            .surfaces
            .iter()
            .map(|s| (&s.surface_type, s.contribution_name.as_str()))
            .collect();
        assert!(
            registered.contains(&(&SurfaceType::Command, "hello")),
            "{registered:?}"
        );
        let errors: Vec<_> = env
            .diagnostics()
            .filter(|d| d.severity == specforge_common::Severity::Error)
            .collect();
        assert!(errors.is_empty(), "{errors:?}");
    }

    #[specforge_test_macros::test(
        behavior = "load_extension_declaration",
        verify = "the declared short name reaches the registry build"
    )]
    fn the_declared_short_name_reaches_the_registry_build() {
        let runtime = InProcessRuntime::new().with(reports);
        let env = load(&["@acme/reports"], &runtime);
        let declaration = env.registries.declaration("@acme/reports").expect("loaded");
        assert_eq!(declaration.short(), "rep");
    }

    #[specforge_test_macros::test(
        behavior = "validate_manifest_v2_schema",
        verify = "an unsupported protocol major version fails the load"
    )]
    fn an_unsupported_protocol_major_fails_the_load() {
        let handshake = serde_json::to_vec(&specforge_protocol_types::HandshakeResponse {
            protocol_version: "2.0.0".to_string(),
            name: "@acme/reports".to_string(),
            version: "0.1.0".to_string(),
            ..Default::default()
        })
        .unwrap();
        let runtime = InProcessRuntime::new().with(reports).answer_raw(
            "@acme/reports",
            "__handshake",
            specforge_wasm::WasmCallResult::Ok(handshake),
        );
        let env = load(&["@acme/reports"], &runtime);
        assert!(env.registries.declaration("@acme/reports").is_none());
        assert!(!env.registries.kinds.contains("report"));
        let e028: Vec<_> = env.diagnostics().filter(|d| d.code == "E028").collect();
        assert_eq!(e028.len(), 1, "{e028:?}");
        assert!(e028[0].message.contains("@acme/reports"), "{e028:?}");
        assert!(e028[0].message.contains("2.0.0"), "{e028:?}");
    }

    #[specforge_test_macros::test(
        behavior = "validate_manifest_v2_schema",
        verify = "an unknown describe key produces a warning"
    )]
    fn an_unknown_describe_key_produces_a_warning() {
        let typo = || {
            let mut b = ContributionsBuilder::new(ExtensionMeta::new("@acme/typo", "0.1.0"));
            b.raw_category(
                "entities",
                serde_json::json!([{ "name": "memo", "testabel": true }]),
            );
            b
        };
        let runtime = InProcessRuntime::new().with(typo);
        let env = load(&["@acme/typo"], &runtime);
        // The extension still loads, without the key it misspelled.
        assert!(env.registries.kinds.contains("memo"));
        let warnings: Vec<_> = env
            .diagnostics()
            .filter(|d| d.severity == specforge_common::Severity::Warning)
            .collect();
        assert_eq!(warnings.len(), 1, "{warnings:?}");
        assert_eq!(warnings[0].code, "W138");
        assert!(warnings[0].message.contains("'testabel'"), "{warnings:?}");
    }
}

mod passes_of_the_declaration {
    //! The passes an environment runs are the ones its one declaration load
    //! read: nothing describes `passes` again, and a passes answer that does
    //! not parse fails the extension's load.

    use super::project;
    use specforge_extension_sdk::prelude::*;
    use specforge_project::{CompiledProject, Environment};
    use specforge_wasm::WasmCallResult;
    use specforge_wasm::testing::InProcessRuntime;

    fn audit() -> ContributionsBuilder {
        let mut b = ContributionsBuilder::new(ExtensionMeta::new("@acme/audit", "0.1.0"));
        b.pass("audit", |p| {
            p.phase("check")
                .run(|_: &PassInput| Vec::<PassDiagnostic>::new());
        });
        b
    }

    /// The audit extension, whose `passes` answer is `items` instead of
    /// what it declares (no SDK guest can answer a description that does
    /// not parse, so it is given raw).
    fn malformed_passes(items: serde_json::Value) -> InProcessRuntime {
        let answer = serde_json::json!({ "category": "passes", "items": items });
        InProcessRuntime::new().with(audit).answer_raw_to(
            "@acme/audit",
            "__describe",
            serde_json::json!({ "category": "passes" }),
            WasmCallResult::Ok(answer.to_string().into_bytes()),
        )
    }

    #[specforge_test_macros::test(
        behavior = "build_registries_from_declarations",
        verify = "a passes description that does not parse fails the extension's load"
    )]
    fn a_passes_description_that_does_not_parse_fails_the_load() {
        let runtime = malformed_passes(serde_json::json!([{ "nam": "x" }]));
        let dir = project(
            serde_json::json!({ "name": "p", "version": "0.1.0", "extensions": ["@acme/audit"] }),
            &[],
        );
        let env = Environment::load(dir.path(), Some(&runtime));
        let e028: Vec<&str> = env
            .diagnostics()
            .filter(|d| d.code == "E028")
            .map(|d| d.message.as_str())
            .collect();
        assert_eq!(e028.len(), 1, "{e028:?}");
        assert!(
            e028[0].contains("describe 'passes' failed")
                && e028[0].contains("missing field `name`"),
            "{}",
            e028[0]
        );
        assert_eq!(env.registries.check_passes().count(), 0);
    }

    /// One environment load describes each category of each extension
    /// once; compiling with it runs the check passes it read, describing
    /// nothing again.
    #[test]
    fn an_environment_describes_each_category_once() {
        let runtime = InProcessRuntime::new().with(audit);
        let dir = project(
            serde_json::json!({ "name": "p", "version": "0.1.0", "extensions": ["@acme/audit"] }),
            &[("a.spec", "spec p \"P\" {\n}\n")],
        );
        let compiled = CompiledProject::compile(dir.path(), Some(&runtime));
        let calls: Vec<String> = runtime
            .calls()
            .iter()
            .map(|c| match c.input["category"].as_str() {
                Some(category) => format!("{}:{category}", c.export),
                None => c.export.clone(),
            })
            .collect();
        let describes = calls.iter().filter(|c| c.starts_with("__describe")).count();
        assert_eq!(
            describes,
            specforge_protocol_types::DECLARED_CATEGORIES.len(),
            "{calls:?}"
        );
        assert_eq!(calls.iter().filter(|c| *c == "__handshake").count(), 1);
        assert_eq!(calls.last().map(String::as_str), Some("__pass_audit"));
        assert!(
            compiled.diagnostics().iter().all(|d| d.code != "E028"),
            "{:?}",
            compiled.diagnostics()
        );
    }

    fn no_version() -> ContributionsBuilder {
        ContributionsBuilder::new(ExtensionMeta::new("@acme/noversion", ""))
    }

    /// The runtime's load failures (E028) come before the declarations'
    /// own diagnostics (E030), whatever the load order (ADR 0012): they
    /// used to interleave extension by extension.
    #[test]
    fn load_failures_come_before_declaration_diagnostics() {
        let runtime = InProcessRuntime::new().with(no_version);
        let dir = project(
            serde_json::json!({
                "name": "p", "version": "0.1.0",
                "extensions": ["@acme/noversion", "@acme/missing"]
            }),
            &[],
        );
        let env = Environment::load(dir.path(), Some(&runtime));
        let codes: Vec<&str> = env.diagnostics().map(|d| d.code.as_str()).collect();
        assert_eq!(codes, ["E028", "E030"], "{codes:?}");
    }
}
