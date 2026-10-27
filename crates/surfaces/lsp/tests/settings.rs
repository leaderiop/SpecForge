//! The language server shows what the project's report shows (ADR 0288).

use serde_json::json;
use specforge_project::testing::{follower_extension, settings_extension};
use tower_lsp::lsp_types::DiagnosticSeverity;

use crate::recorder::publications;
use crate::served::Served;

const GADGET: &str = "gadget widget \"W\" {}\n";

/// The `(code, severity)` of everything the server published for the files of the project, last publication of
/// each, without the extension's namespace.
fn published(settings: serde_json::Value) -> Vec<(String, Option<DiagnosticSeverity>)> {
    let served = Served::new(&[("a.spec", GADGET)])
        .extension("@t/s", |c| *c = settings_extension())
        .extension("@t/f", |c| *c = follower_extension())
        .settings(settings)
        .open(&["a.spec"]);
    let mut found = Vec::new();
    for file in ["a.spec", "specforge.json"] {
        let uri = served.uri(file);
        let sent = served.opening();
        if let Some(last) = publications(sent, &uri).last() {
            for d in last {
                if let Some(tower_lsp::lsp_types::NumberOrString::String(code)) = &d.code {
                    found.push((code.rsplit('/').next().unwrap().to_string(), d.severity));
                }
            }
        }
    }
    found
}

#[specforge_test_macros::test(
    behavior = "load_extension_settings",
    verify = "the language server publishes a raised severity and no gated code"
)]
fn the_lsp_publishes_raised_severities() {
    let off = published(json!({}));
    assert!(
        off.iter().all(|(c, _)| c != "W901" && c != "W903"),
        "{off:?}"
    );
    let strict = published(json!({"@t/s": {"profile": "strict"}}));
    assert!(
        strict.contains(&("W902".to_string(), Some(DiagnosticSeverity::ERROR))),
        "{strict:?}"
    );
    assert!(
        strict.contains(&("W903".to_string(), Some(DiagnosticSeverity::WARNING))),
        "{strict:?}"
    );
}
