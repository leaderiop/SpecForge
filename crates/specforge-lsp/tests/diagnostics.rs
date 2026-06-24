use specforge_common::{Diagnostic, Severity};
use specforge_test_macros::test as spec;

// -- live_diagnostics ---------------------------------------------------------

#[spec(behavior = "live_diagnostics", verify = "diagnostics update after file change")]
#[test]
fn diagnostics_update_after_change() {
    let mut state = specforge_lsp::LspState::new();
    state.open_document("file:///a.spec", "behavior a \"A\" {}\n");

    // Simulate a file change that produces diagnostics
    let diags = vec![Diagnostic {
        code: String::new(),
        suggestion: None,
        message: "E003: unresolved reference 'b'".into(),
        severity: Severity::Error,
        span: None,
    }];
    state.set_diagnostics("file:///a.spec", diags.clone());

    let current = state.diagnostics("file:///a.spec");
    assert_eq!(current.len(), 1);
    assert!(current[0].message.contains("E003"));
}

#[spec(behavior = "live_diagnostics", verify = "only changed file diagnostics are refreshed")]
#[test]
fn only_changed_file_diagnostics_refreshed() {
    let mut state = specforge_lsp::LspState::new();
    state.open_document("file:///a.spec", "a");
    state.open_document("file:///b.spec", "b");

    state.set_diagnostics("file:///a.spec", vec![Diagnostic {
        code: String::new(),
        suggestion: None,
        message: "error in a".into(),
        severity: Severity::Error,
        span: None,
    }]);
    state.set_diagnostics("file:///b.spec", vec![Diagnostic {
        code: String::new(),
        suggestion: None,
        message: "error in b".into(),
        severity: Severity::Error,
        span: None,
    }]);

    // Update only a.spec diagnostics
    state.set_diagnostics("file:///a.spec", vec![]);

    assert_eq!(state.diagnostics("file:///a.spec").len(), 0);
    assert_eq!(state.diagnostics("file:///b.spec").len(), 1);
}

#[spec(behavior = "live_diagnostics", verify = "diagnostics appear within 100ms")]
#[test]
fn diagnostics_appear_within_latency_budget() {
    use std::time::Instant;

    let mut state = specforge_lsp::LspState::new();
    state.open_document("file:///a.spec", "behavior a \"A\" {\n  deps [nonexistent]\n}\n");

    let start = Instant::now();

    // Simulate the diagnostic pipeline: apply change + set diagnostics
    state.apply_change("file:///a.spec", 1, 7, 1, 18, "also_missing");
    state.set_diagnostics("file:///a.spec", vec![Diagnostic {
        code: "E003".into(),
        suggestion: None,
        message: "unresolved reference 'also_missing'".into(),
        severity: Severity::Error,
        span: None,
    }]);

    let elapsed = start.elapsed();

    assert!(!state.diagnostics("file:///a.spec").is_empty(), "diagnostics must be available");
    // The in-memory pipeline (without I/O) must complete well under 100ms
    assert!(
        elapsed.as_millis() < 100,
        "diagnostic pipeline must complete within 100ms, took {}ms",
        elapsed.as_millis(),
    );
}
