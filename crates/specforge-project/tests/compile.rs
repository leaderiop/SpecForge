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
