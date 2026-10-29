//! What `@gsd/core` declares for the verification kinds (`spec/verify.md`), pinned through the SDK's runtime-free
//! `MockHost`. The total number of kinds lives here.

#[allow(
    dead_code,
    reason = "each test crate that includes this module uses some of it"
)]
mod common;

use common::{describe, field_of, fields_of};

fn kinds() -> Vec<serde_json::Value> {
    describe("entities")["items"]
        .as_array()
        .expect("kinds")
        .clone()
}

#[test]
fn the_model_has_its_kinds() {
    assert_eq!(kinds().len(), 54);
}

#[test]
fn an_override_is_a_human_only_kind() {
    let kind = |name: &str| {
        kinds()
            .into_iter()
            .find(|k| k["name"] == name)
            .unwrap_or_else(|| panic!("no kind {name}"))
    };
    assert_eq!(kind("override")["writer"], "human", "{}", kind("override"));
    assert_ne!(kind("verify_gap")["writer"], "human");
}

#[test]
fn the_verification_kinds_declare_the_fields_of_the_model() {
    assert_eq!(
        fields_of("verification"),
        [
            "phase",
            "deferred",
            "status",
            "score",
            "verified",
            "covered_files",
            "re_verification",
            "anti_patterns"
        ]
    );
    assert_eq!(
        fields_of("verify_gap"),
        [
            "verification",
            "truth",
            "reason",
            "artifacts",
            "missing",
            "status",
            "closed_by"
        ]
    );
    assert_eq!(
        fields_of("override"),
        ["verification", "truth", "reason", "by", "at"]
    );
    assert_eq!(field_of("override", "by")["required"], false);
}

#[test]
fn the_guards_are_declared_with_their_reports() {
    let passes = describe("passes");
    let items = passes["items"].as_array().expect("passes");
    let guards: Vec<&serde_json::Value> = items.iter().filter(|p| p["phase"] == "guard").collect();
    let reported: Vec<String> = guards
        .iter()
        .flat_map(|p| p["reports"].as_array().cloned().unwrap_or_default())
        .filter_map(|c| c.as_str().map(String::from))
        .collect();
    let mut reported = reported;
    reported.sort();
    assert_eq!(
        reported,
        ["E106", "E511", "E512", "E516", "W318", "W404"],
        "{passes}"
    );
    // A guard answers no values.
    assert!(
        guards
            .iter()
            .all(|p| p["computes"].as_array().is_none_or(Vec::is_empty))
    );
}
