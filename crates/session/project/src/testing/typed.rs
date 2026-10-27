//! The GSD-shaped typed fields the K3 tests share (ADR 0285), on top of [`super::scoped_kinds`]. The fields are
//! written as the wire carries them, so an attribute the host does not read yet is dropped by the descriptor and
//! the test that needs it fails, not the fixture.

use serde_json::{Value, json};
use specforge_extension_sdk::DeclaredCategory;
use specforge_extension_sdk::prelude::{
    ContributionsBuilder, ExtensionMeta, PassDiagnostic, PassInput,
};
use specforge_protocol_types::{EntityKindDescriptor, ExtensionDeclaration, FieldDescriptor};
use specforge_wasm::testing::InProcessRuntime;

/// The name the extension declaring [`typed_extension`] is served under.
pub const TYPED_EXTENSION: &str = "@test/typed";

fn fields(wire: Value) -> Vec<FieldDescriptor> {
    serde_json::from_value(wire).expect("the fixture's fields are descriptors")
}

fn kind(name: &str, keyword: &str, scoped_by: Option<&str>, wire: Value) -> EntityKindDescriptor {
    EntityKindDescriptor {
        name: name.into(),
        keyword: Some(keyword.into()),
        scoped_by: scoped_by.map(str::to_string),
        fields: fields(wire),
        ..EntityKindDescriptor::default()
    }
}

/// The scoped kinds with these fields:
///
/// - `project.constraints` a block list of `{kind enum{tech_stack, timeline} R, what string R, why string}`
/// - `plan.gap_closure` a reference list of `verify_gap | uat_gap`
/// - `plan.coupling_justified` a block list of `{peer plan R, reason string R}`
/// - `plan.files_modified` a required string list of files from the project root
/// - `summary.requires` a block list of `{phase phase R, provides string R}`
/// - `milestone.completion` a block `{at date, audit audit}`
/// - `discussion.scores` a block `{goal integer 0..100, ambiguity integer 0..100}`
/// - `refactor_proposal.reasons` an enum list `{threshold, jump}`, `.score` an integer of at least 0
/// - `context.canonical_refs` a block list of `{topic string, paths string list of files from the project root}`
/// - `shape.tags` an enum list `{cli, api}`, `shape.checks` a block list of `{kind enum{cli, api}, note string}`
/// - `truth.criterion` an integer from 1 to 5
pub fn typed_extension() -> ExtensionDeclaration {
    let mut declaration = super::scoped_extension().declaration();
    declaration.handshake.name = TYPED_EXTENSION.into();
    let kinds = &mut declaration.entities;
    let add = |kinds: &mut Vec<EntityKindDescriptor>, name: &str, wire: Value| {
        kinds
            .iter_mut()
            .find(|k| k.name == name)
            .expect("a scoped kind")
            .fields
            .extend(fields(wire));
    };
    add(
        kinds,
        "Plan",
        json!([
            {"name": "gap_closure", "field_type": "reference_list", "target_kinds": ["verify_gap", "uat_gap"]},
            {"name": "coupling_justified", "field_type": "block_list", "members": [
                {"name": "peer", "field_type": "reference", "target_kind": "plan", "required": true},
                {"name": "reason", "field_type": "string", "required": true},
            ]},
            {"name": "files_modified", "field_type": "string_list", "file_reference": true,
             "file_base": "project", "required": true},
        ]),
    );
    add(
        kinds,
        "Milestone",
        json!([{"name": "completion", "field_type": "block", "members": [
            {"name": "at", "field_type": "date"},
            {"name": "audit", "field_type": "reference", "target_kind": "audit"},
        ]}]),
    );
    add(
        kinds,
        "Shape",
        json!([
            {"name": "tags", "field_type": "enum_list", "enum_values": ["cli", "api"]},
            {"name": "checks", "field_type": "block_list", "members": [
                {"name": "kind", "field_type": "enum", "enum_values": ["cli", "api"]},
                {"name": "note", "field_type": "string"},
            ]},
        ]),
    );
    kinds.extend([
        kind(
            "Project",
            "project",
            None,
            json!([{"name": "constraints", "field_type": "block_list", "members": [
                {"name": "kind", "field_type": "enum", "enum_values": ["tech_stack", "timeline"], "required": true},
                {"name": "what", "field_type": "string", "required": true},
                {"name": "why", "field_type": "string"},
            ]}]),
        ),
        kind(
            "Summary",
            "summary",
            Some("plan"),
            json!([
                {"name": "plan", "field_type": "reference", "target_kind": "plan"},
                {"name": "requires", "field_type": "block_list", "members": [
                    {"name": "phase", "field_type": "reference", "target_kind": "phase", "required": true},
                    {"name": "provides", "field_type": "string", "required": true},
                ]},
            ]),
        ),
        kind(
            "Discussion",
            "discussion",
            None,
            json!([{"name": "scores", "field_type": "block", "members": [
                {"name": "goal", "field_type": "integer", "minimum": 0, "maximum": 100},
                {"name": "ambiguity", "field_type": "integer", "minimum": 0, "maximum": 100},
            ]}]),
        ),
        kind(
            "RefactorProposal",
            "refactor_proposal",
            None,
            json!([
                {"name": "reasons", "field_type": "enum_list", "enum_values": ["threshold", "jump"]},
                {"name": "score", "field_type": "integer", "minimum": 0},
            ]),
        ),
        kind(
            "Context",
            "context",
            None,
            json!([{"name": "canonical_refs", "field_type": "block_list", "members": [
                {"name": "topic", "field_type": "string"},
                {"name": "paths", "field_type": "string_list", "file_reference": true, "file_base": "project"},
            ]}]),
        ),
        kind(
            "Truth",
            "truth",
            None,
            json!([{"name": "criterion", "field_type": "integer", "minimum": 1, "maximum": 5}]),
        ),
        kind("VerifyGap", "verify_gap", None, json!([])),
        kind("UatGap", "uat_gap", None, json!([])),
    ]);
    declaration
}

/// [`typed_extension`], served in process as `@test/typed`: a project that enables it compiles with these kinds.
/// Its check pass `observe` reports nothing; the runtime records the input it receives.
pub fn typed_runtime() -> InProcessRuntime {
    InProcessRuntime::new().with(|| {
        let mut c = ContributionsBuilder::new(ExtensionMeta::new(TYPED_EXTENSION, "1.0.0"));
        typed_kinds(&mut c);
        c.pass("observe", |p| {
            p.phase("check")
                .run(|_: &PassInput| Vec::<PassDiagnostic>::new());
        });
        c
    })
}

/// The kinds of [`typed_extension`], as an extension `c` contributes them.
pub fn typed_kinds(c: &mut ContributionsBuilder) {
    c.raw_category(
        DeclaredCategory::Entities,
        serde_json::to_value(typed_extension().entities).expect("kinds serialize"),
    );
}
