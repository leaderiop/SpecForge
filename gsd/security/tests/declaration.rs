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

/// SECURITY.md's columns at v1.18.0 (`gsd-core/templates/SECURITY.md`; T19 vendors the file, and this copy names the
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
