//! A held path keeps the caller's spelling (ADR 0272): a message names the path as the call wrote it, a
//! document names a held file by its project path. Each pin is `#[ignore]`d until the ticket named in its
//! reason flips it.

use crate::view_support::{Assembled, Viewed, registries};
use crate::writes::{candidates, project};
use specforge_ops::analysis::analyze::{AnalyzeField, AnalyzeOptions, ReportSource, analyze};
use specforge_ops::extension::{self, AddRequest};
use specforge_ops::{Containment, OpErrorKind};
use specforge_test_macros::test as specforge_test;
use std::path::Path;

fn analyzed(path: &str, root: &Path) -> specforge_ops::Refusal<AnalyzeField> {
    let project = Assembled::over(registries(&["behavior"], &[]))
        .source("behavior b \"B\" {\n  verify unit \"it works\"\n}\n");
    let options = AnalyzeOptions {
        report: ReportSource::Named {
            path: path.into(),
            containment: Containment::within(root),
        },
        ..Default::default()
    };
    analyze(&project.view(), &options).unwrap_err()
}

/// The refusal of a bounded dry-run add of `specifier`, from the project at `root`.
fn added(root: &Path, specifier: &str) -> specforge_ops::OpError {
    let containment = Containment::within(root);
    extension::add(
        &AddRequest {
            root: &specforge_common::ProjectRoot::of(root).unwrap(),
            extensions: vec![specifier.to_string()],
            allow_unsigned: false,
            dry_run: true,
            caller: specforge_ops::testing::bounded(&containment),
        },
        &specforge_ops::registry::Unconfigured("test"),
        &candidates(),
    )
    .unwrap_err()
    .error
}

#[specforge_test(
    behavior = "provide_mcp_analyze_tool",
    verify = "a refusal of test_results names it as the call spelled it"
)]
fn a_bounded_named_report_is_named_as_the_call_wrote_it() {
    let dir = tempfile::tempdir().unwrap();
    for name in ["none.json", "sub/../none.json"] {
        let error = analyzed(name, dir.path()).error;
        assert_eq!(
            error.message,
            format!("cannot read test results {name}: No such file or directory (os error 2)")
        );
    }
}

#[specforge_test(
    behavior = "provide_mcp_add_extension_tool",
    verify = "a refusal of a local path in specifiers names it as the call spelled it"
)]
fn a_bounded_add_names_a_local_path_as_the_call_wrote_it() {
    let dir = project(&[], &[("ext/Cargo.toml", "[package]\nname = \"e\"\n")]);
    let root = dir.path();
    assert_eq!(
        added(root, "nope.wasm").message,
        "no extension binary at nope.wasm"
    );
    let outside = added(root, "../outside.wasm");
    assert_eq!(outside.code, "outside_root");
    assert!(
        outside
            .message
            .starts_with("'../outside.wasm' resolves to "),
        "{}",
        outside.message
    );
    assert_eq!(
        added(root, "ext").message,
        "no built component in ext/target/wasm32-wasip2/release: build the extension first"
    );
}

#[cfg(unix)]
#[specforge_test(
    behavior = "add_extension_to_existing_project",
    verify = "a local install is locked by its project path, a link by its own name"
)]
fn a_local_install_records_its_project_path_a_link_by_its_own_name() {
    let dir = project(&[], &[]);
    let root = dir.path();
    std::fs::create_dir_all(root.join("ext")).unwrap();
    std::fs::create_dir_all(root.join("sub")).unwrap();
    std::fs::write(root.join("ext/greet.wasm"), specforge_ops::testing::GREET).unwrap();
    std::os::unix::fs::symlink("../ext/greet.wasm", root.join("sub/alias.wasm")).unwrap();
    for (given, recorded) in [
        ("sub/alias.wasm", "local:sub/alias.wasm"),
        ("sub/../ext/greet.wasm", "local:ext/greet.wasm"),
    ] {
        let lock = root.join("specforge.lock");
        let _ = std::fs::remove_file(&lock);
        crate::writes::add(root, root.join(given)).unwrap();
        let text = std::fs::read_to_string(&lock).unwrap();
        assert!(
            text.contains(&format!("\"source\": \"{recorded}\"")),
            "{text}"
        );
    }
}

#[specforge_test(
    behavior = "provide_mcp_analyze_tool",
    verify = "a test_results that names a directory or nothing is invalid_input on test_results"
)]
fn a_named_report_that_is_a_directory_is_invalid_input_on_the_report() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("spec")).unwrap();
    for (name, message) in [
        (
            "spec",
            "'spec' is a directory: test results are a specforge-report.json file",
        ),
        ("", "an empty test_results names no file"),
    ] {
        let refusal = analyzed(name, dir.path());
        assert_eq!(refusal.error.kind, OpErrorKind::InvalidInput, "{name}");
        assert_eq!(refusal.error.code, "E045");
        assert_eq!(refusal.subject, Some(AnalyzeField::Report));
        assert_eq!(refusal.error.message, message);
    }
}
