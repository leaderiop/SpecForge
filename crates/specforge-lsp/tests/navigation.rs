//! The LSP's own navigation: span conversion and `use`-path definitions.
//! What definitions and references are is `specforge_ops::navigate`'s
//! (tested in specforge-ops); the LSP's answers are in `e2e_support`.

use specforge_common::{SourceSpan, Sym};
use specforge_test_macros::test as spec;
use std::fs;

// -- source_span_to_lsp_range (1-based → 0-based conversion) -----------------

#[spec(
    behavior = "go_to_definition",
    verify = "source spans convert from 1-based to 0-based for LSP"
)]
fn source_span_converts_1based_to_0based() {
    // Parser produces 1-based spans (line 3, col 1 means third line, first column)
    let span = SourceSpan {
        file: Sym::new("test.spec"),
        start_line: 3,
        start_col: 1,
        end_line: 5,
        end_col: 2,
    };
    let lsp = specforge_lsp::source_span_to_lsp_range(&span);
    // LSP protocol uses 0-based
    assert_eq!(lsp.start_line, 2);
    assert_eq!(lsp.start_col, 0);
    assert_eq!(lsp.end_line, 4);
    assert_eq!(lsp.end_col, 1);
}

#[spec(
    behavior = "go_to_definition",
    verify = "source spans convert from 1-based to 0-based for LSP"
)]
fn source_span_zero_saturates() {
    // Edge case: span with 0 values shouldn't underflow
    let span = SourceSpan {
        file: Sym::new("test.spec"),
        start_line: 0,
        start_col: 0,
        end_line: 0,
        end_col: 0,
    };
    let lsp = specforge_lsp::source_span_to_lsp_range(&span);
    assert_eq!(lsp.start_line, 0);
    assert_eq!(lsp.start_col, 0);
}

// -- goto_import_definition ---------------------------------------------------

/// Go to the target of `use "<import>"` in `main.spec` under `spec_root`.
fn goto_import(spec_root: &std::path::Path, import: &str) -> Option<specforge_common::SourceSpan> {
    specforge_lsp::goto_import_definition(
        import,
        "main.spec",
        spec_root,
        &specforge_resolver::ResolveConfig::default(),
    )
}

/// The resolver's cascade, not a hand-built `{root}/{import}.spec`: a
/// relative path from a nested file and a directory's `index.spec`.
#[spec(
    behavior = "goto_import_definition",
    verify = "go-to-def on use path navigates to target file"
)]
fn goto_import_resolves_relative_and_index_targets() {
    let tmp = tempfile::tempdir().unwrap();
    fs::create_dir_all(tmp.path().join("models")).unwrap();
    fs::create_dir_all(tmp.path().join("sub")).unwrap();
    fs::write(tmp.path().join("models/index.spec"), "term m \"M\" {}\n").unwrap();
    fs::write(tmp.path().join("types.spec"), "term t \"T\" {}\n").unwrap();
    let config = specforge_resolver::ResolveConfig::default();
    let goto = |import: &str| {
        specforge_lsp::goto_import_definition(import, "sub/main.spec", tmp.path(), &config)
            .map(|s| s.file.to_string())
    };
    assert_eq!(goto("../types").as_deref(), Some("types.spec"));
    assert_eq!(goto("models").as_deref(), Some("models/index.spec"));
    assert_eq!(goto("@specforge/software"), None);
}

#[spec(
    behavior = "goto_import_definition",
    verify = "go-to-def on use path navigates to target file"
)]
fn goto_import_navigates_to_file() {
    let tmp = tempfile::tempdir().unwrap();
    let behaviors_dir = tmp.path().join("behaviors");
    fs::create_dir_all(&behaviors_dir).unwrap();
    fs::write(
        behaviors_dir.join("auth.spec"),
        "behavior auth \"Auth\" {}\n",
    )
    .unwrap();

    let result = goto_import(tmp.path(), "behaviors/auth");
    let loc = result.expect("should resolve import");
    assert_eq!(loc.file.as_str(), "behaviors/auth.spec");
    assert_eq!(loc.start_line, 0);
}

#[spec(
    behavior = "goto_import_definition",
    verify = "go-to-def on non-existent use path returns no result"
)]
fn goto_import_returns_none_for_missing() {
    let tmp = tempfile::tempdir().unwrap();
    assert!(goto_import(tmp.path(), "nonexistent/path").is_none());
    // Nor does a path that leaves the spec root.
    fs::write(tmp.path().join("inside.spec"), "term inside \"I\" {}\n").unwrap();
    let root = tmp.path().join("spec");
    fs::create_dir_all(&root).unwrap();
    assert!(goto_import(&root, "../inside").is_none());
    assert!(goto_import(&root, "a/../../inside").is_none());
}

// -- goto_import_definition (LSP dispatch integration) ------------------------

#[spec(
    behavior = "goto_import_definition",
    verify = "go-to-def on use path navigates to target file"
)]
fn goto_definition_dispatches_to_import_on_use_line() {
    let tmp = tempfile::tempdir().unwrap();
    let behaviors_dir = tmp.path().join("behaviors");
    fs::create_dir_all(&behaviors_dir).unwrap();
    fs::write(
        behaviors_dir.join("auth.spec"),
        "behavior auth \"Auth\" {}\n",
    )
    .unwrap();

    // Simulate document content with a use line
    let content = "use \"behaviors/auth\"\n\nbehavior login \"Login\" {}\n";
    let line = content.lines().next().unwrap();

    // The line is a use statement, so extract the import path via import_path_on_line
    let import_path = specforge_lsp::backend::import_path_on_line(line)
        .expect("should extract import path from use line");

    // Dispatch to goto_import_definition (as the LSP handler would)
    let result = goto_import(tmp.path(), import_path);
    let loc = result.expect("should resolve import from use line");
    assert_eq!(loc.file.as_str(), "behaviors/auth.spec");
    assert_eq!(loc.start_line, 0);
}
