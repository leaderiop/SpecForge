use crate::OutputFormat;
use serde_json::json;
use specforge_ops::doctor::{
    BinaryIssue, CredentialHealth, CredentialLevel, DoctorReport, FindingStatus, diagnose,
    user_credential_health,
};
use std::path::Path;

/// `specforge doctor`: the shared project health report (extensions and
/// their enhancements, conflicts, shadowed keywords, installed binaries)
/// plus registry credential health. Exit 1 on any error-level finding.
pub fn run(path: &Path, format: OutputFormat) -> i32 {
    let ctx = crate::pipeline::compile(path);
    let report = diagnose(path, &ctx.manifests, &ctx.diagnostics);
    let credentials = user_credential_health();
    let healthy = !report.has_errors();

    match format {
        OutputFormat::Json => {
            let mut output = serde_json::to_value(&report).expect("serialize doctor report");
            output["status"] = json!(if healthy { "healthy" } else { "issues_found" });
            output["credentials_failures"] = json!(credentials.failures);
            output["credentials"] = json!(credentials.lines);
            println!(
                "{}",
                serde_json::to_string_pretty(&output).expect("serialize JSON output")
            );
        }
        OutputFormat::Human => print!("{}", render_human(&report, &credentials)),
    }

    if healthy && credentials.failures == 0 {
        0
    } else {
        1
    }
}

/// The human report: one section per report part, then credentials.
fn render_human(report: &DoctorReport, credentials: &CredentialHealth) -> String {
    let mut out = String::new();
    macro_rules! line {
        () => { out.push('\n') };
        ($($arg:tt)*) => {{ out.push_str(&format!($($arg)*)); out.push('\n'); }};
    }
    line!("Extensions ({}):", report.extensions.len());
    if report.extensions.is_empty() {
        line!("  none enabled");
    }
    for ext in &report.extensions {
        line!(
            "  {} {} ({}) — {} enhancement(s)",
            ext.name,
            ext.version,
            ext.source,
            ext.enhancement_count
        );
    }

    line!();
    line!("Enhancements by entity kind:");
    if report.enhancements.is_empty() {
        line!("  none");
    }
    for (kind, entries) in &report.enhancements {
        line!("  {kind}:");
        for entry in entries {
            let mut contributed = entry.fields.clone();
            contributed.extend(entry.edge_types.iter().map(|e| format!("edge {e}")));
            if let Some(kinds) = &entry.verify_kinds {
                contributed.push(format!("verify [{}]", kinds.join(", ")));
            }
            line!("    {}: {}", entry.extension, contributed.join(", "));
        }
    }

    line!();
    line!("Conflicts:");
    if report.conflicts.is_empty() {
        line!("  none");
    }
    for conflict in &report.conflicts {
        line!("  [{}] {}", conflict.code, conflict.message);
        line!("    fix: {}", conflict.suggestion);
    }

    line!();
    line!("Shadowed constructs:");
    if report.shadowed.is_empty() {
        line!("  none");
    }
    for shadow in &report.shadowed {
        line!(
            "  '{}' [{}] {}",
            shadow.keyword,
            shadow.code,
            shadow.message
        );
    }

    line!();
    line!(
        "Installed binaries ({} lock entr{} checked):",
        report.extensions_checked,
        if report.extensions_checked == 1 {
            "y"
        } else {
            "ies"
        }
    );
    if report.issues.is_empty() {
        line!("  All installed binaries healthy.");
    }
    for issue in &report.issues {
        match issue {
            BinaryIssue::MissingBinary { name } => {
                line!("  [MISSING] {name} — .wasm binary not found");
            }
            BinaryIssue::StaleHash {
                name,
                expected,
                actual,
            } => line!(
                "  [STALE] {} — hash mismatch (expected {}, got {})",
                name,
                &expected[..8.min(expected.len())],
                &actual[..8.min(actual.len())]
            ),
            BinaryIssue::PeerMismatch {
                name,
                peer,
                required,
                installed,
            } => match installed {
                Some(version) => {
                    line!("  [PEER] {name} — requires {peer} {required}, installed '{version}'")
                }
                None => line!("  [PEER] {name} — requires {peer} {required}, not installed"),
            },
        }
    }

    line!();
    line!("Extension load failures:");
    if report.load_failures.is_empty() {
        line!("  none");
    }
    for failure in &report.load_failures {
        line!("  [{}] {}", failure.code, failure.message);
        line!("    fix: {}", failure.suggestion);
    }
    if !report.z3_available {
        line!();
        line!("[WARN] z3 not on PATH — `specforge analyze --prove` skips SMT checks (W098)");
    }

    let errors = report
        .findings
        .iter()
        .filter(|f| f.status == FindingStatus::Error)
        .count();
    line!();
    if errors == 0 {
        line!("No issues found.");
    } else {
        line!("{errors} issue(s) found.");
    }

    if !credentials.lines.is_empty() || credentials.signing_key_present {
        line!();
        line!("Registry credentials:");
        for credential in &credentials.lines {
            let tag = match credential.level {
                CredentialLevel::Error => "ERROR",
                CredentialLevel::Warning => "WARN",
                CredentialLevel::Ok => "ok",
            };
            line!("  [{tag}] {}", credential.text);
        }
        if credentials.signing_key_present {
            line!("  [ok] signing key present");
        } else {
            line!("  [ok] signing key: not created yet (generated on first publish)");
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use specforge_common::{Diagnostic, Severity};
    use specforge_test_macros::test as specforge_test;

    fn conflict(
        code: &str,
        severity: Severity,
        message: &str,
        suggestion: Option<&str>,
    ) -> Diagnostic {
        Diagnostic {
            code: code.into(),
            severity,
            message: message.into(),
            span: None,
            suggestion: suggestion.map(String::from),
            data: None,
        }
    }

    // No shipped builtin set produces an extension conflict (all nine
    // together compile clean), so the conflicts come in at the shared
    // report's seam: the diagnostics a compile hands `diagnose`.
    #[specforge_test(
        behavior = "run_doctor_check",
        verify = "doctor reports conflicts with resolution suggestions"
    )]
    fn doctor_reports_conflicts_with_a_resolution_suggestion() {
        let dir = tempfile::TempDir::new().unwrap();
        let diagnostics = [
            conflict(
                "E026",
                Severity::Error,
                "entity kind 'feature' registered by 'acme' conflicts with '@specforge/product' (first registration wins)",
                None,
            ),
            conflict(
                "W018",
                Severity::Warning,
                "edge type 'uses' declared by both 'acme' and '@specforge/software'",
                Some("rename one of the edge types"),
            ),
            conflict("W001", Severity::Warning, "an unrelated warning", None),
        ];

        let report = specforge_ops::doctor::diagnose_with(dir.path(), &[], &diagnostics, true);

        let codes: Vec<&str> = report.conflicts.iter().map(|c| c.code.as_str()).collect();
        assert_eq!(codes, ["E026", "W018"]);
        // No suggestion of its own: the catalogued explanation stands in.
        assert!(
            report.conflicts[0]
                .suggestion
                .ends_with("Rename the conflicting kind keyword."),
            "{:?}",
            report.conflicts[0]
        );
        // The diagnostic's own suggestion wins.
        assert_eq!(
            report.conflicts[1].suggestion,
            "rename one of the edge types"
        );
        assert!(report.has_errors(), "an error-level conflict fails doctor");

        let none = CredentialHealth {
            lines: Vec::new(),
            failures: 0,
            signing_key_present: false,
        };
        let human = render_human(&report, &none);
        let section = &human[human.find("Conflicts:").expect("conflicts section")..];
        assert!(section.contains("[E026] entity kind 'feature'"), "{human}");
        assert!(
            section.contains("fix: Two extensions register the same entity kind keyword"),
            "{human}"
        );
        assert!(section.contains("[W018]"), "{human}");
        assert!(
            section.contains("fix: rename one of the edge types"),
            "{human}"
        );
        assert!(human.contains("1 issue(s) found."), "{human}");
    }
}
