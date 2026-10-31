//! ADR 0304: a judged input may read a file named by a string field, and a referrer selector goes on past its
//! reference. The fixture is a milestone audit over `@t/audit`'s own kinds.

use std::sync::Arc;

use serde_json::{Value, json};
use specforge_extension_sdk::DeclaredCategory;
use specforge_extension_sdk::prelude::{ContributionsBuilder, ExtensionMeta};
use specforge_project::judged::{Cited, Inputs};
use specforge_project::testing::{TestProject, declare_rules};
use specforge_project::{CompiledProject, SharedRuntime};
use specforge_registry::PlainDisk;
use specforge_test::prelude::*;
use specforge_wasm::testing::InProcessRuntime;

const EXT: &str = "@t/audit";

const SPEC: &str = r#"milestone m1 "One" {}

milestone m2 "Two" {}

requirement R1 "First" {}

requirement R2 "Second" {}

phase p1 "P1" {
  milestone m1
  requirements [R1]
}

phase p2 "P2" {
  milestone m2
  requirements [R2]
}

audit AU "Audit" {
  milestone m1
  path "src/a.ts"
}
"#;

fn reference(name: &str, target: &str) -> Value {
    json!({"name": name, "field_type": "reference", "target_kind": target})
}

fn kind(name: &str, scoped_by: Option<&str>, fields: Vec<Value>) -> Value {
    let mut kind = json!({"name": name, "keyword": name, "fields": fields});
    if let Some(scope) = scoped_by {
        kind["scoped_by"] = json!(scope);
    }
    kind
}

fn extension() -> ContributionsBuilder {
    let mut c = ContributionsBuilder::new(ExtensionMeta::new(EXT, "0.1.0"));
    c.raw_category(
        DeclaredCategory::Entities,
        json!([
            kind("milestone", None, vec![]),
            kind("requirement", None, vec![]),
            kind(
                "phase",
                Some("milestone"),
                vec![
                    reference("milestone", "milestone"),
                    json!({"name": "requirements", "field_type": "reference_list", "target_kind": "requirement"}),
                ]
            ),
            kind(
                "audit",
                None,
                vec![
                    reference("milestone", "milestone"),
                    json!({"name": "path", "field_type": "string"}),
                ]
            ),
        ]),
    );
    declare_rules(
        &mut c,
        json!([{
            "code": "W900", "severity": "warning", "check": "judged",
            "message_template": "the audit {id} does not hold",
            "judged": {"judge": "auditor", "outcomes": ["passed", "issues"], "passing": ["passed"], "targets": [
                {"kind": "audit", "inputs": [
                    {"name": "audit", "selector": "self"},
                    {"name": "requirements", "selector": "self.milestone<phase.milestone.requirements"},
                    {"name": "code", "selector": "file(self.path)"},
                ]},
            ]},
        }]),
    );
    c.agent("auditor", |a| {
        a.description("Judges");
    });
    c
}

fn project() -> TestProject {
    TestProject::new()
        .config(|c| c["name"] = "p".into())
        .enabling(&[EXT])
        .installing_enabled()
        .file("a.spec", SPEC)
        .file("src/a.ts", "export const a = 1;\n")
}

fn compile(p: &TestProject) -> CompiledProject {
    p.compile_in(Arc::new(InProcessRuntime::new().with(extension)) as SharedRuntime)
}

fn inputs(compiled: &CompiledProject) -> Inputs {
    let rule = compiled
        .environment()
        .registries()
        .judged_rule("t.audit/W900")
        .expect("the rule is registered")
        .clone();
    specforge_project::judged::inputs(compiled, &PlainDisk, "AU", &rule, &Cited::default())
        .expect("the rule judges the audit")
}

fn digest(inputs: &Inputs, name: &str) -> String {
    inputs
        .digests
        .iter()
        .find(|(n, _)| n == name)
        .unwrap_or_else(|| panic!("input {name}: {:?}", inputs.digests))
        .1
        .clone()
}

#[specforge_test(
    behavior = "digest_judged_inputs",
    verify = "a file input may name a path held in a string field"
)]
fn a_file_input_on_a_string_field_digests_the_file() {
    let p = project();
    let first = inputs(&compile(&p));
    assert_eq!(first.files.len(), 1, "{:?}", first.files);
    assert_eq!(first.files[0].path, "src/a.ts");
    p.write("src/a.ts", "export const a = 2;\n");
    let second = inputs(&compile(&p));
    assert_ne!(digest(&first, "code"), digest(&second, "code"));
    std::fs::remove_file(p.root().join("src/a.ts")).unwrap();
    let missing = inputs(&compile(&p));
    assert_eq!(missing.files[0].digest, "absent");
}

#[specforge_test(
    behavior = "digest_judged_inputs",
    verify = "a referrer selector goes on past its reference"
)]
fn a_referrer_path_continues_forward() {
    let p = project();
    let first = inputs(&compile(&p));
    let members = &first
        .reports
        .iter()
        .find(|r| r.name == "requirements")
        .expect("the input is read")
        .members;
    assert_eq!(members, &["R1".to_string()]);
    // A requirement of another milestone's phase changes nothing; one of this milestone's phases does.
    p.write("a.spec", SPEC.replace("\"Second\"", "\"Second, changed\""));
    let other = inputs(&compile(&p));
    assert_eq!(
        digest(&first, "requirements"),
        digest(&other, "requirements")
    );
    p.write("a.spec", SPEC.replace("\"First\"", "\"First, changed\""));
    let own = inputs(&compile(&p));
    assert_ne!(digest(&first, "requirements"), digest(&own, "requirements"));
}
