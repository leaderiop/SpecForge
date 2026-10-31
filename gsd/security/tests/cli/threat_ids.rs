//! `gsd.security/E101` and `specforge gsd threat-ids`: two plans of one phase claiming one threat ID.

use super::harness::Project;
use super::model::model;

fn threat(plan: &str, name: &str) -> String {
    format!(
        "\nthreat {name} \"Dup\" {{\n  plan v1.0/01/{plan}\n  category tampering\n  component \"x\"\n  severity low\n  \
         disposition mitigate\n  mitigation \"m\"\n}}\n"
    )
}

fn add(p: &Project, text: &str) {
    let now = p.read("spec/security.spec");
    p.write("spec/security.spec", &format!("{now}{text}"));
}

fn ids(p: &Project, phase: &str) -> serde_json::Value {
    let run = p.run(&["gsd", "threat-ids", phase, "--format", "json"]);
    assert_eq!(run.code, 0, "{}", run.all());
    serde_json::from_str(&run.stdout).unwrap_or_else(|e| panic!("JSON ({e}): {}", run.all()))
}

/// Phase 01's plans 01-01 and 01-02 both claim T-01-02 and both keep T-01-SC.
fn duplicated() -> Project {
    let p = model();
    add(&p, &threat("01-01", "T-01-02"));
    add(&p, &threat("01-02", "T-01-02"));
    add(&p, &threat("01-02", "T-01-SC"));
    p
}

#[test]
fn a_duplicate_is_one_e101_per_claiming_threat_and_the_supply_chain_row_never_counts() {
    let p = duplicated();
    let e101: Vec<_> = p
        .report()
        .into_iter()
        .filter(|(code, _)| code == "gsd.security/E101")
        .collect();
    assert_eq!(e101.len(), 2, "{e101:#?}");
    assert!(
        e101.iter()
            .all(|(_, m)| m.contains("T-01-02") && m.contains("01-01, 01-02")),
        "{e101:#?}"
    );
    assert!(e101.iter().all(|(_, m)| !m.contains("SC")), "{e101:#?}");
}

#[test]
fn threat_ids_answers_init_s_fields() {
    let p = duplicated();
    assert_eq!(
        ids(&p, "01"),
        serde_json::json!({
            "threat_id_duplicates": [{ "id": "T-01-02", "plans": ["01-01", "01-02"] }],
            "threat_id_duplicate_count": 1
        })
    );
}

#[test]
fn no_duplicate_is_an_empty_list_and_a_renumber_silences_it() {
    let p = model();
    assert_eq!(
        ids(&p, "01"),
        serde_json::json!({ "threat_id_duplicates": [], "threat_id_duplicate_count": 0 })
    );
    let p = duplicated();
    p.edit(
        "spec/security.spec",
        "threat T-01-02 \"Dup\" {\n  plan v1.0/01/01-02",
        "threat T-01-03 \"Dup\" {\n  plan v1.0/01/01-02",
    );
    assert_eq!(ids(&p, "01")["threat_id_duplicate_count"], 0);
    assert!(
        !p.report()
            .iter()
            .any(|(code, _)| code == "gsd.security/E101")
    );
}

#[test]
fn an_unknown_phase_is_refused() {
    let run = model().run(&["gsd", "threat-ids", "99", "--format", "json"]);
    assert_ne!(run.code, 0, "{}", run.all());
}
