//! The GSD-shaped kinds the K4 tests share (ADR 0287): who writes a field, and what history judges. The kinds are
//! written as the wire carries them, so an attribute the host does not read yet is dropped by the descriptor and the
//! test that needs it fails, not the fixture.
//!
//! - `milestone`, `phase` (`milestone R I`), `decision` (`phase R I`, `origin I`), scoped as [`super::scoped_kinds`]
//! - `verification` (`phase R I`, `status C`, `re_verification C`), computed by the pass `verify_status`
//! - `uat_test` (`result H`)
//! - `discussion` (`entries` a block list `{who, text}`, append-only)
//! - `deliverable` (`checks` a block list `{kind R, ref R, status C}`)
//! - `deviation` (an append-only kind with a `note`)
//! - `finding` (an append-only kind scoped by `phase`, with a `note` and a free `component`)

use serde_json::{Value, json};
use specforge_extension_sdk::DeclaredCategory;
use specforge_extension_sdk::prelude::{ContributionsBuilder, ExtensionMeta};
use specforge_protocol_types::{EntityKindDescriptor, ExtensionDeclaration};
use specforge_wasm::testing::InProcessRuntime;

/// The name the extension declaring [`gsd_history_kinds`] is served under.
pub const GSD_EXTENSION: &str = "@test/gsd";

/// The kinds, as the wire carries them.
pub fn gsd_history_wire() -> Value {
    json!([
        {"name": "milestone", "keyword": "milestone"},
        {"name": "phase", "keyword": "phase", "scoped_by": "milestone", "fields": [
            {"name": "milestone", "field_type": "reference", "target_kind": "milestone"},
        ]},
        {"name": "decision", "keyword": "decision", "scoped_by": "phase", "fields": [
            {"name": "phase", "field_type": "reference", "target_kind": "phase", "required": true, "immutable": true},
            {"name": "origin", "field_type": "string", "immutable": true},
        ]},
        {"name": "verification", "keyword": "verification", "scoped_by": "phase", "fields": [
            {"name": "phase", "field_type": "reference", "target_kind": "phase", "required": true, "immutable": true},
            {"name": "status", "field_type": "string", "writer": "computed"},
            {"name": "re_verification", "field_type": "string", "writer": "computed"},
        ]},
        {"name": "finding", "keyword": "finding", "scoped_by": "phase", "append_only": true, "fields": [
            {"name": "phase", "field_type": "reference", "target_kind": "phase"},
            {"name": "note", "field_type": "string"},
            {"name": "component", "field_type": "string"},
        ]},
        {"name": "uat_test", "keyword": "uat_test", "scoped_by": "phase", "fields": [
            {"name": "phase", "field_type": "reference", "target_kind": "phase"},
            {"name": "result", "field_type": "string", "writer": "harness"},
        ]},
        {"name": "discussion", "keyword": "discussion", "fields": [
            {"name": "entries", "field_type": "block_list", "append_only": true, "members": [
                {"name": "who", "field_type": "string"},
                {"name": "text", "field_type": "string"},
            ]},
        ]},
        {"name": "deliverable", "keyword": "deliverable", "fields": [
            {"name": "checks", "field_type": "block_list", "members": [
                {"name": "kind", "field_type": "string", "required": true},
                {"name": "ref", "field_type": "string", "required": true},
                {"name": "status", "field_type": "string", "writer": "computed"},
            ]},
        ]},
        {"name": "deviation", "keyword": "deviation", "append_only": true, "fields": [
            {"name": "note", "field_type": "string"},
        ]},
    ])
}

/// The only history flag sits on a block member: `deliverable.checks[].kind` is immutable (plan U8(a)). With
/// `with_journal`, `journal` is also an append-only kind whose `parts[].kind` is immutable.
pub fn gsd_member_history_wire(with_journal: bool) -> Value {
    let mut kinds = json!([
        {"name": "deliverable", "keyword": "deliverable", "fields": [
            {"name": "checks", "field_type": "block_list", "members": [
                {"name": "kind", "field_type": "string", "immutable": true},
                {"name": "ref", "field_type": "string"},
            ]},
        ]},
    ]);
    if with_journal {
        kinds.as_array_mut().expect("an array").push(json!(
            {"name": "journal", "keyword": "journal", "append_only": true, "fields": [
                {"name": "parts", "field_type": "block_list", "members": [
                    {"name": "kind", "field_type": "string", "immutable": true},
                ]},
            ]}
        ));
    }
    kinds
}

/// `@test/gsd` declaring [`gsd_member_history_wire`] and no pass, served in process.
pub fn gsd_member_history_runtime(with_journal: bool) -> InProcessRuntime {
    InProcessRuntime::new().with(move || {
        let mut c = ContributionsBuilder::new(ExtensionMeta::new(GSD_EXTENSION, "1.0.0"));
        c.raw_category(
            DeclaredCategory::Entities,
            gsd_member_history_wire(with_journal),
        );
        c
    })
}

/// The pass that computes the verification's fields, reading the verification kind's baseline.
pub fn gsd_history_passes() -> Value {
    json!([{
        "name": "verify_status", "phase": "compute", "baseline": ["verification"],
        "computes": ["verification.status", "verification.re_verification", "deliverable.checks.status"],
    }])
}

/// The declaration of `@test/gsd`.
pub fn gsd_history_extension() -> ExtensionDeclaration {
    let c = gsd_history_builder();
    c.declaration()
}

fn gsd_history_builder() -> ContributionsBuilder {
    let mut c = ContributionsBuilder::new(ExtensionMeta::new(GSD_EXTENSION, "1.0.0"));
    c.raw_category(DeclaredCategory::Entities, gsd_history_wire());
    c.raw_category(DeclaredCategory::Passes, gsd_history_passes());
    c
}

/// The kinds of [`gsd_history_wire`] as descriptors (the flags the descriptor reads).
pub fn gsd_history_kinds() -> Vec<EntityKindDescriptor> {
    serde_json::from_value(gsd_history_wire()).expect("the fixture's kinds are descriptors")
}

/// `@test/gsd` served in process. Its pass `verify_status` answers `values` (`[{entity, field, value}]`) with no
/// diagnostics; the runtime records the input it receives.
pub fn gsd_history_runtime(values: Value) -> InProcessRuntime {
    let answer = json!({"diagnostics": [], "summary": {}, "values": values});
    InProcessRuntime::new()
        .with(gsd_history_builder)
        .answer_raw(
            GSD_EXTENSION,
            "__pass_verify_status",
            specforge_wasm::WasmCallResult::Ok(answer.to_string().into_bytes()),
        )
}
