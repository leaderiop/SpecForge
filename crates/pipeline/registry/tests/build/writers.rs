//! Who writes a field (ADR 0287): the registry registers a field's writer and history flags, a kind's
//! `append_only` and a compute pass's `computes` and `baseline`, and refuses a contradiction with W021.
//! The declarations are written as the wire carries them.

use serde_json::{Value, json};
use specforge_extension_sdk::DeclaredCategory;
use specforge_protocol_types::ExtensionDeclaration;
use specforge_test_macros::test as spec;

use crate::support::{build, coded, declare, diagnostics};

/// `@test/ext` with these kinds and passes, as the wire carries them.
fn wire(kinds: Value, passes: Value) -> ExtensionDeclaration {
    declare("@test/ext", |c| {
        c.raw_category(DeclaredCategory::Entities, kinds);
        c.raw_category(DeclaredCategory::Passes, passes);
    })
}

/// A kind `thing` with these fields.
fn thing(fields: Value) -> Value {
    json!([{"name": "thing", "keyword": "thing", "fields": fields}])
}

fn compute(name: &str, computes: Value) -> Value {
    json!({"name": name, "phase": "compute", "computes": computes})
}

#[spec(
    behavior = "judge_field_history",
    verify = "a field declares its writer and history, and the registry refuses a contradiction with W021"
)]
fn a_field_declares_its_writer_and_history() {
    let good = build([wire(
        thing(json!([
            {"name": "status", "field_type": "string", "writer": "computed"},
            {"name": "result", "field_type": "string", "writer": "harness"},
            {"name": "origin", "field_type": "string", "immutable": true},
            {"name": "entries", "field_type": "string_list", "append_only": true},
            {"name": "checks", "field_type": "block_list", "members": [
                {"name": "status", "field_type": "string", "writer": "computed"},
            ]},
        ])),
        json!([compute(
            "give",
            json!(["thing.status", "thing.checks.status"])
        )]),
    )]);
    assert!(diagnostics(&good).is_empty(), "{:?}", diagnostics(&good));
    let declared = |field: &str| {
        serde_json::to_value(good.fields().get("thing", field).unwrap().declared()).unwrap()
    };
    assert_eq!(declared("status")["writer"], "computed");
    assert_eq!(declared("result")["writer"], "harness");
    assert_eq!(declared("origin")["immutable"], true);
    assert_eq!(declared("entries")["append_only"], true);

    // Each contradiction is W021 and the flag is dropped.
    for field in [
        json!({"name": "a", "field_type": "string", "writer": "agent"}),
        json!({"name": "a", "field_type": "string", "writer": "computed", "required": true}),
        json!({"name": "a", "field_type": "string", "writer": "computed", "immutable": true}),
        json!({"name": "a", "field_type": "string", "append_only": true}),
        json!({"name": "a", "field_type": "block_list", "members": [
            {"name": "m", "field_type": "string", "writer": "harness"},
        ]}),
    ] {
        let build = build([wire(
            thing(json!([field.clone()])),
            json!([compute(
                "give",
                json!(if field["writer"] == "computed" {
                    vec!["thing.a"]
                } else {
                    vec![]
                })
            )]),
        )]);
        assert_eq!(
            coded(&build, "W021").len(),
            1,
            "{field}: {:?}",
            diagnostics(&build)
        );
    }
}

#[spec(
    behavior = "judge_field_history",
    verify = "a kind is append-only and a pass computes fields and names the baseline it reads, or W021"
)]
fn a_kind_and_a_pass_declare_history() {
    let kinds = json!([
        {"name": "log", "keyword": "log", "append_only": true},
        {"name": "thing", "keyword": "thing", "fields": [
            {"name": "status", "field_type": "string", "writer": "computed"},
        ]},
    ]);
    let passes = json!([{"name": "give", "phase": "compute", "computes": ["thing.status"], "baseline": ["thing"]}]);
    let good = build([wire(kinds.clone(), passes)]);
    assert!(diagnostics(&good).is_empty(), "{:?}", diagnostics(&good));
    assert_eq!(
        serde_json::to_value(&good.kinds().get("log").unwrap().declared).unwrap()["append_only"],
        true
    );

    // Nothing computes the field; a pass computes an authored field; two passes compute one field; `computes`
    // on a pass that is not a compute pass.
    for passes in [
        json!([]),
        json!([{"name": "give", "phase": "compute", "computes": ["thing.status", "thing.missing"]}]),
        json!([
            compute("a", json!(["thing.status"])),
            compute("b", json!(["thing.status"]))
        ]),
        json!([{"name": "give", "phase": "check", "computes": ["thing.status"]}]),
    ] {
        let build = build([wire(kinds.clone(), passes.clone())]);
        assert!(
            !coded(&build, "W021").is_empty(),
            "{passes}: {:?}",
            diagnostics(&build)
        );
    }
}

#[spec(
    behavior = "judge_field_history",
    verify = "lifecycle_field is read and ignored: a kind that sends it loads without W138"
)]
fn lifecycle_field_is_read_and_ignored() {
    let kinds = json!([{"name": "thing", "keyword": "thing", "lifecycle_field": "nosuch",
        "fields": [{"name": "status", "field_type": "string"}]}]);
    let build = build([wire(kinds, json!([]))]);
    assert!(diagnostics(&build).is_empty(), "{:?}", diagnostics(&build));
    assert!(build.fields().contains("thing", "status"));
}

#[spec(
    behavior = "judge_field_history",
    verify = "a kind and a field may be human-only and a pass a guard, or W021"
)]
fn a_kind_and_a_field_may_be_human_and_a_pass_a_guard() {
    let kinds = json!([
        {"name": "override", "keyword": "override", "writer": "human", "fields": [
            {"name": "reason", "field_type": "string", "writer": "human", "required": true},
        ]},
        {"name": "thing", "keyword": "thing", "fields": [
            {"name": "note", "field_type": "string", "writer": "human"},
        ]},
    ]);
    let passes = json!([{"name": "gate", "phase": "guard"}]);
    let good = build([wire(kinds, passes.clone())]);
    assert!(diagnostics(&good).is_empty(), "{:?}", diagnostics(&good));
    assert!(good.kinds().get("override").unwrap().is_human_only());
    assert!(!good.kinds().get("thing").unwrap().is_human_only());
    // A guard runs when a write is judged: not with a compile, not under analyze.
    assert_eq!(good.guard_passes().count(), 1);
    assert_eq!(good.check_passes().count(), 0);
    assert_eq!(good.analyze_passes().count(), 0);

    // A kind writer other than human; a required human field on a kind agents write; computes on a guard.
    for (kinds, passes) in [
        (
            json!([{"name": "thing", "keyword": "thing", "writer": "agent"}]),
            passes.clone(),
        ),
        (
            json!([{"name": "thing", "keyword": "thing", "fields": [
                {"name": "note", "field_type": "string", "writer": "human", "required": true},
            ]}]),
            passes.clone(),
        ),
        (
            json!([{"name": "thing", "keyword": "thing", "fields": [
                {"name": "status", "field_type": "string", "writer": "computed"},
            ]}]),
            json!([{"name": "gate", "phase": "guard", "computes": ["thing.status"]}]),
        ),
    ] {
        let build = build([wire(kinds.clone(), passes)]);
        assert!(
            !coded(&build, "W021").is_empty(),
            "{kinds}: {:?}",
            diagnostics(&build)
        );
    }
}
