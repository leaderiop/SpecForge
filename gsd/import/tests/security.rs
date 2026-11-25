//! A plan's `<threat_model>` through the command in process: `@gsd/security`'s trust boundaries and threats when the
//! manifest loads it, against the vault twin's `plans.spec`; one needs-extension entry per element when it does not.
//!
//! Then `01-SECURITY.md`, against the twin's `security.spec`: the audit, the statuses it records, its accepted risk and
//! its audit run.
//!
//! Known differences from the twin (13-RESEARCH D5, D11-D13): a boundary's title is its `Boundary` cell and a threat's
//! its `Component` (the twin's titles are hand-made); `data_crossing` and `status` come from `SECURITY.md`, never from
//! the plan, and only where a row names the entity: the twin's `files`, `evidence` and three more statuses have no
//! `.planning` source (D11); the audit's `asvs_level` and `created`, which the twin omits, are carried, and the run's
//! `at` is the heading's date at midnight with no `run_by` (D12); `01-02/TB1.data_crossing` is carried (D13).

mod common;

use serde_json::{Value, json};
use specforge_extension_sdk::prelude::*;

use common::{
    creates, entries, fact, feed, find, fixture, fixture_manifest, has, import, import_with,
    manifest, paths, planning_facts,
};
use gsd_import::limits::MAX_OPS;

const SECURITY: &str = ".planning/phases/01-core/01-SECURITY.md";

/// The threat statuses `01-SECURITY.md` records: a row matching the threat's id and component.
const STATUSES: &[(&str, &str)] = &[
    ("v1.0/01/01-01/T-01-01", "closed"),
    ("v1.0/01/01-02/T-01-02", "open"),
    ("v1.0/01/01-02/T-01-03", "closed"),
];

#[test]
fn vault_threat_models_match_the_twin_with_security_loaded() {
    let out = import_with("vault", true);
    assert_eq!(out.exit_code, 0, "{}", out.stderr);
    let all = creates(&out);
    assert_eq!(
        paths(&all, "trust_boundary"),
        ["v1.0/01/01-01/TB1", "v1.0/01/01-02/TB1"]
    );
    // `data_crossing` is SECURITY.md's; the twin lacks 01-02's (D13).
    for (path, boundary, description, crossing) in [
        (
            "v1.0/01/01-01/TB1",
            "process → vault.bin",
            "the sealed file is on a disk other users can read and write",
            "encrypted secrets",
        ),
        (
            "v1.0/01/01-02/TB1",
            "shell → process",
            "the secret is typed as an argument and the key is an environment variable",
            "secrets and the key, in the clear",
        ),
    ] {
        let tb = find(&all, "trust_boundary", path);
        let plan = path.rsplit_once('/').map(|(p, _)| p);
        assert_eq!(
            tb.fields,
            json!({
                "plan": plan,
                "boundary": boundary,
                "description": description,
                "data_crossing": crossing,
            })
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
        let mut want = json!({
            "plan": plan,
            "category": category,
            "component": component,
            "severity": severity,
            "disposition": disposition,
            "mitigation": mitigation,
        });
        // The status is SECURITY.md's, set only where a row names the threat's id and component.
        if let Some(status) = STATUSES.iter().find(|(p, _)| *p == path).map(|(_, s)| *s) {
            want["status"] = json!(status);
        }
        assert_eq!(threat.fields, want, "{path}");
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

/// The paths of the threats of `out` whose status is `status` (`Value::Null`: none).
fn with_status(out: &CommandOutput, status: Value) -> Vec<String> {
    let mut found: Vec<String> = creates(out)
        .into_iter()
        .filter(|c| c.kind == "threat" && c.fields["status"] == status)
        .map(|c| c.path)
        .collect();
    found.sort_unstable();
    found
}

#[test]
fn vault_closes_exactly_two_threats() {
    let out = import_with("vault", true);
    assert_eq!(out.exit_code, 0, "{}", out.stderr);
    assert_eq!(
        with_status(&out, json!("closed")),
        ["v1.0/01/01-01/T-01-01", "v1.0/01/01-02/T-01-03"]
    );
    assert_eq!(with_status(&out, json!("open")), ["v1.0/01/01-02/T-01-02"]);
    // No row names these by id and component: no status is invented (D11).
    assert_eq!(
        with_status(&out, Value::Null),
        [
            "v1.0/01/01-01/T-01-02",
            "v1.0/01/01-01/T-01-SC",
            "v1.0/01/01-02/T-01-SC"
        ]
    );
    let reported = entries(&out, SECURITY);
    for (line, says) in [
        (30, "threat v1.0/01/01-01/T-01-01: status closed"),
        (31, "threat v1.0/01/01-02/T-01-02: status open"),
        (32, "threat v1.0/01/01-02/T-01-03: status closed"),
        (21, "trust_boundary v1.0/01/01-01/TB1: data_crossing"),
        (22, "trust_boundary v1.0/01/01-02/TB1: data_crossing"),
    ] {
        assert!(
            has(&reported, line, "mapped", says),
            "{line} {says}: {reported:#?}"
        );
    }
}

#[test]
fn vault_security_audit_matches_the_twin() {
    let out = import_with("vault", true);
    assert_eq!(out.exit_code, 0, "{}", out.stderr);
    let all = creates(&out);
    let audit = find(&all, "security_audit", "v1.0/01/01-SECURITY");
    assert_eq!(
        audit.fields,
        json!({ "phase": "v1.0/01", "asvs_level": 1, "created": "2026-10-30T00:00:00Z" })
    );
    assert_eq!(audit.file, "spec/gsd/phases/01.spec");
    assert_eq!(paths(&all, "accepted_risk"), ["v1.0/01/01-SECURITY/AR1"]);
    assert_eq!(
        find(&all, "accepted_risk", "v1.0/01/01-SECURITY/AR1").fields,
        json!({
            "audit": "v1.0/01/01-SECURITY",
            "threat": "v1.0/01/01-02/T-01-03",
            "rationale": "the names are not secret; the values are never printed",
            "accepted_by": "maintainer",
            "at": "2026-10-30",
        })
    );
    assert_eq!(paths(&all, "audit_run"), ["v1.0/01/01-SECURITY/AU1"]);
    assert_eq!(
        find(&all, "audit_run", "v1.0/01/01-SECURITY/AU1").fields,
        json!({
            "audit": "v1.0/01/01-SECURITY",
            "at": "2026-10-29T00:00:00Z",
            "total": 3,
            "closed": 1,
            "open": 2,
        })
    );
    let reported = entries(&out, SECURITY);
    for (line, outcome, says) in [
        (2, "redundant", "the file name gives the phase"),
        (3, "redundant", "the file name gives the phase"),
        (4, "computed", "security_audit.status is computed"),
        (6, "computed", "security_audit.threats_open is computed"),
        (8, "filled", "the frontmatter gives a date only"),
        (11, "mapped", "security_audit v1.0/01/01-SECURITY"),
        (13, "ignored", "the template's instruction"),
        (17, "mapped", "each row sets data_crossing"),
        (26, "mapped", "each row sets status"),
        (34, "unmapped", "prose"),
        (40, "mapped", "each row is an accepted_risk"),
        (44, "mapped", "accepted_risk v1.0/01/01-SECURITY/AR1"),
        (46, "unmapped", "prose"),
        (50, "ignored", "a heading over the audit runs"),
        (52, "filled", "the heading gives a date only"),
        (52, "mapped", "audit_run v1.0/01/01-SECURITY/AU1"),
        (62, "unmapped", "prose"),
        (69, "unmapped", "prose"),
    ] {
        assert!(
            has(&reported, line, outcome, says),
            "{line} {says}: {reported:#?}"
        );
    }
}

#[test]
fn vault_security_needs_the_extension_without_it() {
    let out = import_with("vault", false);
    assert_eq!(out.exit_code, 0, "{}", out.stderr);
    let all = creates(&out);
    for kind in ["security_audit", "accepted_risk", "audit_run", "threat"] {
        assert!(paths(&all, kind).is_empty(), "{kind}");
    }
    let reported = entries(&out, SECURITY);
    assert_eq!(reported.len(), 1, "{reported:#?}");
    assert_eq!(reported[0].2, "needs-extension");
}

#[test]
fn a_hostile_security_audit_sets_nothing_it_cannot_match() {
    let text = "---\nphase: \"02\"\nslug: core\nasvs_level: 7\ncreated: someday\n---\n\n\
        # Phase 1 — Security\n\n\
        ## Trust Boundaries\n\n\
        | Boundary | Description | Data Crossing |\n|---|---|---|\n| nowhere → here | d | x |\n\n\
        ## Threat Register\n\n\
        | Threat ID | Category | Component | Severity | Disposition | Mitigation | Status |\n\
        |---|---|---|---|---|---|---|\n\
        | T-01-01\u{202e} | I | src/vault.js seal | high | mitigate | m | closed |\n\
        | T-01-02 | I | src/nowhere | high | mitigate | m | closed |\n\
        | T-01-SC | T | npm/pip/cargo installs | high | mitigate | x | closed |\n\
        | T-01-01 | I | src/vault.js seal | high | mitigate | m | fixed |\n\
        | T-01-03 | I | src/cli.js list | low | accept | another mitigation | closed |\n\n\
        ## Accepted Risks Log\n\n\
        | Risk ID | Threat Ref | Rationale | Accepted By | Date |\n|---|---|---|---|---|\n\
        | AR-1 | T-09-99 | r | me | 2026-10-30 |\n\
        | AR-2 | T-01-03 | r | me | 2026-13-45 |\n\
        | Risk 3 | T-01-03 | r | me | 2026-10-30 |\n\n\
        ## Security Audit 2026-13-45\n\n\
        | Metric | Count |\n|---|---|\n| Threats found | three |\n| Elapsed | 4 |\n";
    let facts: Vec<Value> = planning_facts(&fixture("vault"))
        .into_iter()
        .map(|f| {
            if f["path"] == SECURITY {
                fact(SECURITY, text)
            } else {
                f
            }
        })
        .collect();
    let out = import(
        &CommandGraph::default(),
        &feed(facts, false, Some(&fixture_manifest("vault", true))),
    );
    assert_eq!(out.exit_code, 0, "{}", out.stderr);
    assert_eq!(
        with_status(&out, json!("closed")),
        ["v1.0/01/01-02/T-01-03"]
    );
    assert!(with_status(&out, json!("open")).is_empty());
    let all = creates(&out);
    assert_eq!(
        find(&all, "security_audit", "v1.0/01/01-SECURITY").fields,
        json!({ "phase": "v1.0/01" })
    );
    assert_eq!(paths(&all, "accepted_risk"), ["v1.0/01/01-SECURITY/AR2"]);
    assert_eq!(
        find(&all, "accepted_risk", "v1.0/01/01-SECURITY/AR2").fields,
        json!({
            "audit": "v1.0/01/01-SECURITY",
            "threat": "v1.0/01/01-02/T-01-03",
            "rationale": "r",
            "accepted_by": "me",
        })
    );
    assert_eq!(
        find(&all, "audit_run", "v1.0/01/01-SECURITY/AU1").fields,
        json!({ "audit": "v1.0/01/01-SECURITY" })
    );
    let reported = entries(&out, SECURITY);
    for (line, says) in [
        (2, "names another phase than the file name's 01"),
        (4, "is no level 1 to 3"),
        (5, "is no date or RFC 3339 timestamp"),
        (
            14,
            "no plan of the phase holds the boundary `nowhere → here`",
        ),
        (20, "no plan of the phase holds the threat `T-01-01"),
        (21, "the component `src/nowhere`: no status set"),
        (22, "2 plans' entities match the threat `T-01-SC`"),
        (23, "is none of open, closed: no status set"),
        (24, "differs from the plan's"),
        (
            30,
            "no plan of the phase holds the threat `T-09-99`: not imported",
        ),
        (31, "the Date `2026-13-45` is no YYYY-MM-DD date"),
        (32, "the Risk ID `Risk 3` is no `AR-<n>`"),
        (34, "`at` left out"),
        (38, "the Count of `Threats found` is no whole number"),
        (39, "the metric `Elapsed` is no field of an audit run"),
    ] {
        assert!(
            has(&reported, line, "unmapped", says),
            "{line} {says}: {reported:#?}"
        );
    }
}
