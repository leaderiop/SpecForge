//! What `@gsd/security` declares, pinned through the SDK's runtime-free `MockHost`: the five kinds as the model lists
//! them, the enhancement on core's `validation_row`, and the map from SECURITY.md's columns to the fields.

use gsd_security::build;
use serde_json::Value;
use specforge_extension_sdk::testing::MockHost;

fn describe(category: &str) -> Value {
    let host = MockHost::new(build());
    let text = host
        .describe_json(category)
        .unwrap_or_else(|| panic!("no category {category}"));
    serde_json::from_str(&text).expect("describe answers JSON")
}

/// The golden `tests/golden/<name>`; `GSD_BLESS=1` writes it.
fn golden(name: &str, actual: &Value) {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/golden")
        .join(name);
    let text = serde_json::to_string_pretty(actual).expect("a JSON value prints") + "\n";
    if std::env::var_os("GSD_BLESS").is_some() {
        std::fs::create_dir_all(path.parent().expect("a folder")).expect("the golden folder");
        std::fs::write(&path, &text).expect("write the golden");
        return;
    }
    let expected = std::fs::read_to_string(&path)
        .unwrap_or_else(|_| panic!("{} is missing: GSD_BLESS=1 writes it", path.display()));
    assert!(
        expected == text,
        "{name} differs: review the change, then GSD_BLESS=1"
    );
}

fn fields_of(kind: &str) -> Vec<String> {
    let entities = describe("entities");
    let entity = entities["items"]
        .as_array()
        .and_then(|kinds| kinds.iter().find(|k| k["name"] == kind))
        .unwrap_or_else(|| panic!("no kind {kind}"));
    entity["fields"]
        .as_array()
        .expect("fields")
        .iter()
        .map(|f| f["name"].as_str().expect("a name").to_string())
        .collect()
}

#[test]
fn the_kinds_are_the_models() {
    let names: Vec<String> = describe("entities")["items"]
        .as_array()
        .expect("items")
        .iter()
        .map(|k| k["name"].as_str().expect("name").to_string())
        .collect();
    assert_eq!(
        names,
        [
            "trust_boundary",
            "threat",
            "security_audit",
            "accepted_risk",
            "audit_run"
        ]
    );
    assert_eq!(
        fields_of("trust_boundary"),
        ["plan", "boundary", "description", "data_crossing"]
    );
    assert_eq!(
        fields_of("threat"),
        [
            "plan",
            "category",
            "component",
            "severity",
            "disposition",
            "mitigation",
            "files",
            "status",
            "evidence",
            "retroactive",
            "blocking"
        ]
    );
    assert_eq!(
        fields_of("security_audit"),
        [
            "phase",
            "asvs_level",
            "created",
            "threats_open",
            "status",
            "register_planned"
        ]
    );
    assert_eq!(
        fields_of("accepted_risk"),
        ["audit", "threat", "rationale", "accepted_by", "at"]
    );
    assert_eq!(
        fields_of("audit_run"),
        ["audit", "at", "total", "closed", "open", "run_by"]
    );
}

#[test]
fn the_declaration_is_pinned() {
    golden("declaration.json", &describe("entities"));
    golden("enhancements.json", &describe("enhancements"));
}

#[test]
fn validation_row_gains_threat_and_secure_behavior() {
    let enhancements = describe("enhancements");
    let row = enhancements["items"]
        .as_array()
        .expect("items")
        .iter()
        .find(|e| e["target_kind"] == "validation_row")
        .unwrap_or_else(|| panic!("no enhancement of validation_row: {enhancements}"));
    let names: Vec<&str> = row["fields"]
        .as_array()
        .expect("fields")
        .iter()
        .map(|f| f["name"].as_str().expect("name"))
        .collect();
    assert_eq!(names, ["threat", "secure_behavior"]);
}

/// SECURITY.md's columns at v1.19.0 (`gsd-core/templates/SECURITY.md`; T19 vendors the file, and this copy names the
/// upstream path until the test reads it), each with the field that holds it. `None`: SpecForge's own form.
const TEMPLATE_COLUMNS: &[(&str, &str, Option<&str>)] = &[
    ("frontmatter", "phase", Some("security_audit.phase")),
    ("frontmatter", "slug", None), // the phase's own slug
    ("frontmatter", "status", Some("security_audit.status")),
    (
        "frontmatter",
        "threats_open",
        Some("security_audit.threats_open"),
    ),
    (
        "frontmatter",
        "asvs_level",
        Some("security_audit.asvs_level"),
    ),
    ("frontmatter", "created", Some("security_audit.created")),
    (
        "Trust Boundaries",
        "Boundary",
        Some("trust_boundary.boundary"),
    ),
    (
        "Trust Boundaries",
        "Description",
        Some("trust_boundary.description"),
    ),
    (
        "Trust Boundaries",
        "Data Crossing",
        Some("trust_boundary.data_crossing"),
    ),
    ("Threat Register", "Threat ID", None), // the entity's name
    ("Threat Register", "Category", Some("threat.category")),
    ("Threat Register", "Component", Some("threat.component")),
    ("Threat Register", "Severity", Some("threat.severity")),
    ("Threat Register", "Disposition", Some("threat.disposition")),
    ("Threat Register", "Mitigation", Some("threat.mitigation")),
    ("Threat Register", "Status", Some("threat.status")),
    ("Accepted Risks Log", "Risk ID", None), // the entity's name
    (
        "Accepted Risks Log",
        "Threat Ref",
        Some("accepted_risk.threat"),
    ),
    (
        "Accepted Risks Log",
        "Rationale",
        Some("accepted_risk.rationale"),
    ),
    (
        "Accepted Risks Log",
        "Accepted By",
        Some("accepted_risk.accepted_by"),
    ),
    ("Accepted Risks Log", "Date", Some("accepted_risk.at")),
    ("Security Audit Trail", "Audit Date", Some("audit_run.at")),
    (
        "Security Audit Trail",
        "Threats Total",
        Some("audit_run.total"),
    ),
    ("Security Audit Trail", "Closed", Some("audit_run.closed")),
    ("Security Audit Trail", "Open", Some("audit_run.open")),
    ("Security Audit Trail", "Run By", Some("audit_run.run_by")),
    ("Sign-Off", "Approval", Some("security_audit.status")),
];

/// Fields no template column names: SpecForge's own (the kind or the field says why).
const OWN_FIELDS: &[&str] = &[
    "trust_boundary.plan",
    "threat.plan",
    "threat.files",
    "threat.evidence",
    "threat.retroactive",
    "threat.blocking",
    "security_audit.register_planned",
    "accepted_risk.audit",
    "audit_run.audit",
    "security_audit.phase", // also `phase:`
];

#[test]
fn the_template_columns_map_to_fields() {
    let mapped: Vec<&str> = TEMPLATE_COLUMNS.iter().filter_map(|c| c.2).collect();
    let mut declared = Vec::new();
    for kind in [
        "trust_boundary",
        "threat",
        "security_audit",
        "accepted_risk",
        "audit_run",
    ] {
        for field in fields_of(kind) {
            declared.push(format!("{kind}.{field}"));
        }
    }
    for field in &mapped {
        assert!(
            declared.iter().any(|d| d == field),
            "{field} is not a field"
        );
    }
    for field in &declared {
        assert!(
            mapped.contains(&field.as_str()) || OWN_FIELDS.contains(&field.as_str()),
            "{field} is in no column and not listed as SpecForge's own"
        );
    }
}

/// The twin of core's: `Cargo.toml`'s `tracks` is the crate's `TRACKS`, and both extensions track one release.
#[test]
fn the_tracked_release_is_the_manifests() {
    let manifest = include_str!("../Cargo.toml");
    assert!(
        manifest.contains(&format!("tracks = \"{}\"", gsd_security::TRACKS)),
        "Cargo.toml tracks a different release"
    );
    assert_eq!(gsd_security::TRACKS, gsd_core::TRACKS);
}

/// A threat's files are the mitigation its plan writes: checked once the threat is closed, its own owner state
/// (ADR 0350, ADR 0362).
#[test]
fn threat_files_is_a_planned_path() {
    let entities = describe("entities");
    let files = entities["items"]
        .as_array()
        .and_then(|kinds| kinds.iter().find(|k| k["name"] == "threat"))
        .and_then(|threat| threat["fields"].as_array())
        .and_then(|fields| fields.iter().find(|f| f["name"] == "files"))
        .cloned()
        .expect("threat.files");
    assert_eq!(files["field_type"], "string_list");
    assert_eq!(
        files["checks"][0]["checker"], "@specforge/checks:file",
        "{files}"
    );
    assert_eq!(files["checks_while"], "complete", "{files}");
}

/// A threat is complete once it is closed, under its plan's own `complete_when`: the owner state its files and its
/// mitigation verdict wait for.
#[test]
fn a_threat_is_complete_once_closed() {
    let entities = describe("entities");
    let threat = entities["items"]
        .as_array()
        .and_then(|kinds| kinds.iter().find(|k| k["name"] == "threat"))
        .cloned()
        .expect("threat");
    assert_eq!(
        threat["complete_when"],
        serde_json::json!({"field": "status", "values": ["closed"]})
    );
}

/// One row of the security spec's "Judged rules' inputs" table: `(code, target, inputs, judged_while)`.
type JudgedRow = (String, String, Vec<(String, String)>, Option<String>);

fn judged_rows() -> Vec<JudgedRow> {
    let table = gsd_kit::markdown::tables(include_str!("../spec/README.md"))
        .into_iter()
        .find(|t| t.heading == "Judged rules' inputs")
        .expect("the judged inputs table");
    table
        .rows
        .iter()
        .filter_map(|cells| {
            let [rule, target, inputs, _binds] = &cells[..] else {
                panic!("a judged row has four cells: {cells:?}");
            };
            let code = rule.strip_prefix("gsd.security/")?.to_string();
            let (list, note) = inputs.split_once(';').unwrap_or((inputs, ""));
            let inputs = list
                .split(" · ")
                .map(|piece| {
                    let rest = piece.trim().strip_prefix('`').expect("a quoted input name");
                    let (name, selector) = rest.split_once('`').expect("a closing quote");
                    (name.to_string(), selector.trim().to_string())
                })
                .collect();
            let judged_while = note
                .split("`judged_while ")
                .nth(1)
                .and_then(|r| r.split('`').next())
                .map(String::from);
            Some((code, target.clone(), inputs, judged_while))
        })
        .collect()
}

/// Each judged rule's descriptor reads exactly what the spec's table lists for it: its target, its inputs in order and
/// its owner state; and the table lists every judged rule of the code book's table.
#[test]
fn the_judged_inputs_are_the_spec_table() {
    let rows = judged_rows();
    let codes: Vec<&str> = rows.iter().map(|(code, ..)| code.as_str()).collect();
    let table: Vec<&str> = gsd_security::judged::JUDGED
        .iter()
        .map(|j| j.code)
        .collect();
    assert_eq!(codes, table);
    let rules = describe("validation_rules");
    for (code, target, inputs, judged_while) in rows {
        let descriptor = rules["items"]
            .as_array()
            .expect("rules")
            .iter()
            .find(|r| r["code"] == code.as_str() && r["check"] == "judged")
            .unwrap_or_else(|| panic!("{code} is not judged"));
        let judged = &descriptor["judged"];
        assert_eq!(judged["judge"], "prompt:secure_phase", "{code}");
        let targets = judged["targets"].as_array().expect("targets");
        assert_eq!(targets.len(), 1, "{code}");
        assert_eq!(targets[0]["kind"], target.as_str(), "{code}");
        let declared: Vec<(String, String)> = targets[0]["inputs"]
            .as_array()
            .expect("inputs")
            .iter()
            .map(|i| {
                (
                    i["name"].as_str().expect("name").to_string(),
                    i["selector"].as_str().expect("selector").to_string(),
                )
            })
            .collect();
        assert_eq!(declared, inputs, "{code}");
        assert_eq!(
            targets[0]["judged_while"].as_str(),
            judged_while.as_deref(),
            "{code}"
        );
    }
}
