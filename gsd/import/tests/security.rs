//! A plan's `<threat_model>` through the command in process: `@gsd/security`'s trust boundaries and threats when the
//! manifest loads it, against the vault twin's `plans.spec`; one needs-extension entry per element when it does not.
//!
//! Known differences from the twin (13-RESEARCH D5): a boundary's title is its `Boundary` cell and a threat's its
//! `Component` (the twin's titles are hand-made); `data_crossing`, `files`, `status` and `evidence` come from
//! `SECURITY.md` (13-08), never from the plan.

mod common;

use serde_json::json;
use specforge_extension_sdk::prelude::*;

use common::{creates, entries, fact, feed, find, has, import, import_with, manifest, paths};
use gsd_import::limits::MAX_OPS;

#[test]
fn vault_threat_models_match_the_twin_with_security_loaded() {
    let out = import_with("vault", true);
    assert_eq!(out.exit_code, 0, "{}", out.stderr);
    let all = creates(&out);
    assert_eq!(
        paths(&all, "trust_boundary"),
        ["v1.0/01/01-01/TB1", "v1.0/01/01-02/TB1"]
    );
    for (path, boundary, description) in [
        (
            "v1.0/01/01-01/TB1",
            "process → vault.bin",
            "the sealed file is on a disk other users can read and write",
        ),
        (
            "v1.0/01/01-02/TB1",
            "shell → process",
            "the secret is typed as an argument and the key is an environment variable",
        ),
    ] {
        let tb = find(&all, "trust_boundary", path);
        let plan = path.rsplit_once('/').map(|(p, _)| p);
        assert_eq!(
            tb.fields,
            json!({ "plan": plan, "boundary": boundary, "description": description })
        );
        assert_eq!(tb.title, boundary);
    }
    let mut threats = paths(&all, "threat");
    threats.sort_unstable();
    assert_eq!(
        threats,
        [
            "v1.0/01/01-01/T-01-01",
            "v1.0/01/01-01/T-01-02",
            "v1.0/01/01-01/T-01-SC",
            "v1.0/01/01-02/T-01-02",
            "v1.0/01/01-02/T-01-03",
            "v1.0/01/01-02/T-01-SC",
        ]
    );
    let supply = (
        "tampering",
        "npm/pip/cargo installs",
        "high",
        "mitigate",
        "package-legitimacy gate + blocking human checkpoint for [ASSUMED]/[SUS]",
    );
    for (path, (category, component, severity, disposition, mitigation)) in [
        (
            "v1.0/01/01-01/T-01-01",
            (
                "information_disclosure",
                "src/vault.js seal",
                "high",
                "mitigate",
                "AES-256-GCM under a key that never reaches the disk",
            ),
        ),
        (
            "v1.0/01/01-01/T-01-02",
            (
                "tampering",
                "src/vault.js open",
                "high",
                "mitigate",
                "the GCM auth tag is verified; a tampered file throws",
            ),
        ),
        ("v1.0/01/01-01/T-01-SC", supply),
        (
            "v1.0/01/01-02/T-01-02",
            (
                "information_disclosure",
                "src/cli.js argv",
                "high",
                "mitigate",
                "accept the secret on stdin instead of argv",
            ),
        ),
        (
            "v1.0/01/01-02/T-01-03",
            (
                "information_disclosure",
                "src/cli.js list",
                "low",
                "accept",
                "list prints names only, never values",
            ),
        ),
        ("v1.0/01/01-02/T-01-SC", supply),
    ] {
        let threat = find(&all, "threat", path);
        let plan = path.rsplit_once('/').map(|(p, _)| p);
        // No `status`: the auditor's verdict is SECURITY.md's (13-08).
        assert_eq!(
            threat.fields,
            json!({
                "plan": plan,
                "category": category,
                "component": component,
                "severity": severity,
                "disposition": disposition,
                "mitigation": mitigation,
            }),
            "{path}"
        );
        assert_eq!(threat.title, component);
    }
}

#[test]
fn vault_threat_models_need_the_extension_without_it() {
    let out = import_with("vault", false);
    assert_eq!(out.exit_code, 0, "{}", out.stderr);
    let all = creates(&out);
    assert!(paths(&all, "trust_boundary").is_empty());
    assert!(paths(&all, "threat").is_empty());
    for (file, line) in [
        (".planning/phases/01-core/01-01-PLAN.md", 56),
        (".planning/phases/01-core/01-02-PLAN.md", 48),
    ] {
        let reported = entries(&out, file);
        let needs: Vec<_> = reported
            .iter()
            .filter(|(_, s, _, _)| s == "<threat_model>")
            .collect();
        assert_eq!(needs.len(), 1, "{reported:#?}");
        assert!(
            has(&reported, line, "needs-extension", "@gsd/security"),
            "{reported:#?}"
        );
    }
}

/// The import of a roadmap of phase 1 and the plan 01-01 whose body is `body`, `@gsd/security` loaded.
fn secure(body: &str) -> CommandOutput {
    let road = "# Roadmap: X\n\n### Phase 1: One\n**Goal**: g\n**Success Criteria**:\n  1. c\n";
    let plan = format!(
        "---\ntype: execute\nfiles_modified: [a.js]\n---\n\n<objective>\nDo it.\n</objective>\n\n{body}"
    );
    let facts = vec![fact(".planning/ROADMAP.md", road), fact(PLAN, &plan)];
    import(
        &CommandGraph::default(),
        &feed(
            facts,
            false,
            Some(&manifest(&["@gsd/core", "@gsd/security"])),
        ),
    )
}

const PLAN: &str = ".planning/phases/01-one/01-01-PLAN.md";

#[test]
fn a_hostile_register_reports_each_row_it_refuses() {
    // The body starts at line 10: the register's header at 13, its rows from 15, the other table at 23.
    let body = "<threat_model>\n## STRIDE Threat Register\n\n\
        | Threat ID | Category | Component | Severity | Disposition | Mitigation Plan | Owner |\n\
        |---|---|---|---|---|---|---|\n\
        | T 01 | S | a | high | mitigate | m | x |\n\
        | T-01-02 | X | b | high | mitigate | m | x |\n\
        | T-01-03 | Denial of service | c | High | accept | m | x |\n\
        | T-01-03 | E | d | low | transfer | m | x |\n\
        | T-01-04 | S | e | severe | mitigate | m | x |\n\
        \n## Other\n\n| A |\n|---|\n| b |\n</threat_model>\n";
    let out = secure(body);
    assert_eq!(out.exit_code, 0, "{}", out.stderr);
    let all = creates(&out);
    assert_eq!(paths(&all, "threat"), ["v1.0/01/01-01/T-01-03"]);
    assert_eq!(
        find(&all, "threat", "v1.0/01/01-01/T-01-03").fields,
        json!({
            "plan": "v1.0/01/01-01",
            "category": "denial_of_service",
            "component": "c",
            "severity": "high",
            "disposition": "accept",
            "mitigation": "m",
        })
    );
    let reported = entries(&out, PLAN);
    for (line, says) in [
        (13, "a column `Owner`"),
        (15, "the Threat ID `T 01`"),
        (16, "the Category `X`"),
        (18, "is given twice"),
        (19, "the Severity `severe`"),
        (23, "a table no row"),
    ] {
        assert!(
            has(&reported, line, "unmapped", says),
            "{line} {says}: {reported:#?}"
        );
    }
}

#[test]
fn a_register_past_max_ops_refuses_the_import() {
    let rows: String = (1..=10_001)
        .map(|n| format!("| T-{n} | S | c | low | accept | m |\n"))
        .collect();
    let body = format!(
        "<threat_model>\n## STRIDE Threat Register\n\n\
         | Threat ID | Category | Component | Severity | Disposition | Mitigation Plan |\n\
         |---|---|---|---|---|---|\n{rows}</threat_model>\n"
    );
    let out = secure(&body);
    assert_ne!(out.exit_code, 0);
    assert!(out.plan.is_none_or(|p| p.ops.is_empty()));
    assert!(
        out.stderr.contains(&format!("truncated at {MAX_OPS}")),
        "{}",
        out.stderr
    );
}
