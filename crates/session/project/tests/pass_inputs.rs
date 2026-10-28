//! Pins what a check, a compute and a guard pass receive when they declare nothing beyond 1.10: the entities, the
//! edges, the scalar, block and block-list fields, and no file facts, no recorded test report, no list items and no
//! clock (r35, K11-K14). The snapshots are the encoded inputs of the project at the round's base; the tickets that
//! add a field to the wire must leave them as they are, for a pass that does not ask.

use std::sync::Arc;

use serde_json::Value;
use specforge_extension_sdk::prelude::*;
use specforge_project::testing::TestProject;
use specforge_test::prelude::*;
use specforge_wasm::testing::InProcessRuntime;

const EXT: &str = "@test/inputs";

const SPEC: &str = "gadget one \"One\" {\n  title \"First\"\n  meta { owner ann }\n  parts [ { name a  kind x } ]\n}\n\ngadget two \"Two\" {\n  title \"Second\"\n}\n";

fn nothing(_: &PassInput) -> Vec<PassDiagnostic> {
    Vec::new()
}

/// `@test/inputs`: a `gadget` of scalar, block and block-list fields (no list field) and one pass of each phase.
fn extension() -> InProcessRuntime {
    InProcessRuntime::new().with(|| {
        let mut c = ContributionsBuilder::new(ExtensionMeta::new(EXT, "1.0.0"));
        c.kind("gadget", |k| {
            k.keyword("gadget");
            k.field("title", |f| {
                f.field_type(FieldType::String);
            });
            k.field("meta", |f| {
                f.field_type(FieldType::Block).member("owner", |m| {
                    m.field_type(FieldType::String);
                });
            });
            k.field("parts", |f| {
                f.field_type(FieldType::BlockList)
                    .member("name", |m| {
                        m.field_type(FieldType::String);
                    })
                    .member("kind", |m| {
                        m.field_type(FieldType::String);
                    });
            });
            k.field("score", |f| {
                f.field_type(FieldType::Integer).computed();
            });
        });
        c.pass("audit", |p| {
            p.phase("check").run(nothing);
        });
        c.pass("score", |p| {
            p.compute().computes(&["gadget.score"]).run(nothing);
        });
        c.pass("gate", |p| {
            p.guard().baseline(&["gadget"]).run(nothing);
        });
        c
    })
}

fn input_of(runtime: &InProcessRuntime, export: &str) -> String {
    let call = runtime
        .calls()
        .into_iter()
        .rev()
        .find(|c| c.export == export)
        .unwrap_or_else(|| panic!("{export} was never called"));
    serde_json::to_string_pretty(&call.input).unwrap()
}

#[specforge_test(
    behavior = "run_check_phase_passes",
    verify = "a pass declared for the check phase runs on every compile"
)]
fn pass_input_is_byte_equal_without_k11_k14() {
    let project = TestProject::new()
        .config(|c| c["name"] = "p".into())
        .enabling(&[EXT])
        .installing_enabled()
        .file("a.spec", SPEC);
    let runtime = Arc::new(extension());
    let compiled = project.compile_in(runtime.clone());

    let check = input_of(&runtime, "__pass_audit");
    let compute = input_of(&runtime, "__pass_score");
    insta::assert_snapshot!("check", check);
    insta::assert_snapshot!("compute", compute);

    // A write is judged: the guard pass runs against the project before it.
    let edited = compiled
        .edited(&[("a.spec".to_string(), SPEC.replace("Second", "Zweite"))].into())
        .judged_against(&compiled);
    drop(edited);
    let guard = input_of(&runtime, "__pass_gate");
    insta::assert_snapshot!("guard", guard);

    // None of the later wire fields is on a pass input that does not ask for it.
    for text in [&check, &compute, &guard] {
        let input: Value = serde_json::from_str(text).unwrap();
        for absent in ["files", "test_results", "now", "today"] {
            assert!(input.get(absent).is_none(), "{absent} in {text}");
        }
    }
}
