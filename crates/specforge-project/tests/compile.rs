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
    let json = specforge_emitter::emit_json(&compiled.graph);
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
