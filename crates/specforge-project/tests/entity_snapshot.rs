//! How every reader after the graph build sees an entity (ADR 0019): the
//! field text a declarative rule matches, a custom validator receives and a
//! compiler pass reads; who owes obligations under an untargeted obligation
//! rule; and where `file_exists` resolves a path. The probe extension
//! `@pin/snapshot` declares, in process, the rules that echo what each
//! reader saw (plan 01 §3, Appendix A).

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use serde_json::{Value, json};
use specforge_common::Diagnostic;
use specforge_extension_sdk::prelude::*;
use specforge_project::CompiledProject;
use specforge_wasm::testing::InProcessRuntime;
use tempfile::TempDir;

const EXT: &str = "@pin/snapshot";

/// The fields the `P2xx` rules read, in code order (`P200` is `values`).
const FIELDS: &[&str] = &[
    "values",
    "mix",
    "ensures",
    "metric",
    "empty_block",
    "tags",
    "shape",
];

/// `item` (testable, accepts verify, `abstract` exempts) and `note` (not
/// testable, accepts verify); `P100` a custom rule answering `Pass` (its
/// inputs are read from the runtime's calls); `P200`–`P206` rules that
/// never match, on the fields of [`FIELDS`]; `P300` an obligation rule
/// with no target kind; `P400` `file_exists` on `doc`; a check pass `echo`
/// that reports nothing.
fn extension() -> ContributionsBuilder {
    let mut c = ContributionsBuilder::new(ExtensionMeta::new(EXT, "0.1.0"));
    c.kind("item", |k| {
        k.description("probe")
            .testable(true)
            .supports_verify(true)
            .open_fields(true);
        k.field("abstract", |f| {
            f.field_type(FieldType::Bool).exempts_obligations();
        });
    });
    c.kind("note", |k| {
        k.description("probe, not testable")
            .testable(false)
            .supports_verify(true)
            .open_fields(true);
    });
    c.rule("P100", |r| {
        r.check(CheckKind::Custom)
            .severity(ValidationSeverity::Info)
            .target_kind("item")
            .wasm_function("validate__echo")
            .message_template("custom validator sees {id}: {value}")
            .validate(|_| ValidatorVerdict::Pass);
    });
    for (i, field) in FIELDS.iter().enumerate() {
        c.rule(&format!("P2{i:02}"), |r| {
            r.check(CheckKind::FieldValueConstraint)
                .severity(ValidationSeverity::Info)
                .target_kind("item")
                .field(field)
                .message_template("declarative rule sees {id}.{field} = '{value}'")
                .constraint(|k| {
                    k.kind(ConstraintKind::Matches).pattern("^__never__$");
                });
        });
    }
    c.rule("P300", |r| {
        r.check(CheckKind::NoVerifyStatements)
            .severity(ValidationSeverity::Warning)
            .field("verify")
            .message_template("{kind} '{id}' declares no verify obligations");
    });
    c.rule("P400", |r| {
        r.check(CheckKind::FileExists)
            .severity(ValidationSeverity::Warning)
            .target_kind("item")
            .field("doc")
            .message_template("{id}: file '{value}' does not exist");
    });
    c.pass("echo", |p| {
        p.phase("check")
            .run(|_: &PassInput| Vec::<PassDiagnostic>::new());
    });
    c
}

const SPEC: &str = r#"item alpha "Alpha" {
  values [low, high]
  mix [1, true]
  ensures {
    done "it is done"
  }
  empty_block {
  }
  tags []
  doc "doc.md"
  metric expr { latency < 10ms, load > 5 }
  shape string | string[]
  verify unit "alpha works"
}

item beta "Beta" {
  values []
}

note gamma "Gamma" {
}

item delta "Delta" {
  abstract true
}
"#;

/// The §3 project: `spec_root: "spec"`, `spec/main.spec` and `spec/doc.md`.
fn project() -> TempDir {
    let dir = TempDir::new().unwrap();
    let config =
        json!({ "name": "repro", "version": "0.1.0", "spec_root": "spec", "extensions": [EXT] });
    fs::write(dir.path().join("specforge.json"), config.to_string()).unwrap();
    fs::create_dir(dir.path().join("spec")).unwrap();
    fs::write(dir.path().join("spec/main.spec"), SPEC).unwrap();
    fs::write(dir.path().join("spec/doc.md"), "# doc\n").unwrap();
    dir
}

fn runtime() -> InProcessRuntime {
    InProcessRuntime::new().with(extension)
}

/// A compile of `root` through `runtime`, and its diagnostics.
fn compile(root: &Path, runtime: &InProcessRuntime) -> (CompiledProject, Vec<Diagnostic>) {
    let compiled = CompiledProject::compile(root, Some(runtime));
    let diagnostics = compiled.diagnostics();
    (compiled, diagnostics)
}

/// `code message` of every diagnostic whose code starts with `prefix`.
fn reported(diagnostics: &[Diagnostic], prefix: &str) -> Vec<String> {
    diagnostics
        .iter()
        .filter(|d| d.code.starts_with(prefix))
        .map(|d| format!("{} {}", d.code, d.message))
        .collect()
}

/// The fields the custom validator received for `id`, by key.
fn validator_fields(runtime: &InProcessRuntime, id: &str) -> serde_json::Map<String, Value> {
    let call = runtime
        .calls()
        .into_iter()
        .find(|c| c.export == "validate__echo" && c.input["entity"]["id"] == id)
        .unwrap_or_else(|| panic!("validate__echo was never called for {id}"));
    call.input["entity"]["fields"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| (f["key"].as_str().unwrap().to_string(), f["value"].clone()))
        .collect()
}

/// The pass input's entity `id`.
fn pass_entity(runtime: &InProcessRuntime, id: &str) -> Value {
    let input = runtime
        .calls()
        .into_iter()
        .rev()
        .find(|c| c.export == "__pass_echo")
        .expect("the echo pass ran")
        .input;
    input["entities"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["id"] == id)
        .unwrap_or_else(|| panic!("no pass entity {id}"))
        .clone()
}

fn keys(entity: &Value) -> BTreeSet<String> {
    entity["fields"]
        .as_object()
        .unwrap()
        .keys()
        .cloned()
        .collect()
}

#[test]
fn pin_validators_see_null_where_rules_see_text() {
    // pin (01-T0): today's behaviour; flipped by 01-T2
    let dir = project();
    let runtime = runtime();
    let (_, diagnostics) = compile(dir.path(), &runtime);

    let alpha = validator_fields(&runtime, "alpha");
    for key in ["values", "mix", "metric", "shape"] {
        assert_eq!(alpha[key], Value::Null, "{key}");
    }
    assert_eq!(alpha["empty_block"], "");
    assert_eq!(alpha["ensures"], "done");
    assert_eq!(validator_fields(&runtime, "beta")["values"], Value::Null);

    assert_eq!(
        reported(&diagnostics, "P2"),
        [
            "P200 declarative rule sees alpha.values = 'low | high'",
            "P202 declarative rule sees alpha.ensures = 'done'",
            "P205 declarative rule sees alpha.tags = ''",
        ]
    );

    assert_eq!(
        keys(&pass_entity(&runtime, "alpha")),
        ["doc", "ensures", "tags", "values", "verify"]
            .map(String::from)
            .into()
    );
    assert!(keys(&pass_entity(&runtime, "beta")).is_empty());
}

#[test]
fn pin_an_untargeted_obligation_rule_fires_everywhere_and_exempts_everyone() {
    // pin (01-T0): today's behaviour; flipped by 01-T3
    let dir = project();
    let runtime = runtime();
    let (compiled, diagnostics) = compile(dir.path(), &runtime);

    assert_eq!(
        reported(&diagnostics, "P3"),
        [
            "P300 item 'beta' declares no verify obligations",
            "P300 note 'gamma' declares no verify obligations",
        ]
    );
    for id in ["alpha", "beta", "gamma", "delta"] {
        assert_eq!(pass_entity(&runtime, id)["exempt"], true, "{id}");
    }
    let recorded = compiled
        .recorded()
        .at(
            Some(dir.path()),
            &compiled.graph,
            specforge_project::coverage::CoverageRegistries::of(&compiled.env.registries),
        )
        .unwrap();
    assert_eq!(recorded.coverage.summary.testable_total, 1);
}

#[test]
fn pin_file_exists_reads_the_working_directory() {
    // pin (01-T0): today's behaviour; flipped by 01-T5
    let dir = project();
    let (_, diagnostics) = compile(dir.path(), &runtime());
    // The test runs in the crate directory, never the temp spec root.
    assert_eq!(
        reported(&diagnostics, "P4"),
        ["P400 alpha: file 'doc.md' does not exist"]
    );
}
