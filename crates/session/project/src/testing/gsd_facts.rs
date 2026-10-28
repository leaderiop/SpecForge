//! The GSD-shaped extension the r35 kernel tests share (K11-K13), served in process and written as the wire carries
//! it, so an attribute the host does not read yet is dropped by the descriptor and the test that needs it fails, not
//! the fixture:
//!
//! - [`FACTS_EXTENSION`] (`@t/facts`): the kinds `phase` (`criteria`, a `string_list` whose items hold `, `) and
//!   `plan` (`phase`, `files_modified` a `string_list` of project paths)
//! - the check passes `files` (reads the field `plan.files_modified` with its text, as `modified`, and the glob
//!   `src/**/*.rs` as `sources`), `tests` (declares `reads_tests`) and `lists` (declares nothing), each answering
//!   no finding; the runtime records every input
//!
//! [`gsd_facts_project`] is a project of one phase, one plan naming `src/lib.rs` (which exists) and `docs/gone.md`
//! (which does not), and no test report; [`REPORT`] is a report to write at `specforge-report.json`.

use serde_json::{Value, json};
use specforge_extension_sdk::DeclaredCategory;
use specforge_extension_sdk::prelude::{ContributionsBuilder, ExtensionMeta};
use specforge_wasm::WasmCallResult;
use specforge_wasm::testing::InProcessRuntime;

use super::TestProject;

/// The extension declaring the kinds and the passes.
pub const FACTS_EXTENSION: &str = "@t/facts";

/// The text of `src/lib.rs` in [`gsd_facts_project`].
pub const LIB_RS: &str = "pub fn one() -> u8 {\n    1\n}\n";

/// A recorded test report with no result.
pub const REPORT: &str = r#"{"runner":"fixture","results":{}}"#;

/// The spec of [`gsd_facts_project`].
pub const GSD_FACTS_SPEC: &str = "phase p1 \"One\" {\n  criteria [\"ships, fast\", \"works\"]\n}\n\nplan pl1 \"Plan\" {\n  phase p1\n  files_modified [\"src/lib.rs\", \"docs/gone.md\"]\n}\n";

/// The kinds of [`FACTS_EXTENSION`], as the wire carries them.
pub fn gsd_facts_wire() -> Value {
    json!([
        {"name": "phase", "keyword": "phase", "fields": [
            {"name": "criteria", "field_type": "string_list"},
        ]},
        {"name": "plan", "keyword": "plan", "fields": [
            {"name": "phase", "field_type": "reference", "target_kind": "phase"},
            {"name": "files_modified", "field_type": "string_list"},
        ]},
    ])
}

/// The passes of [`FACTS_EXTENSION`], as the wire carries them.
pub fn gsd_facts_passes() -> Value {
    json!([
        {"name": "files", "phase": "check", "reads": [
            {"name": "modified", "field": "plan.files_modified", "text": true},
            {"name": "sources", "glob": "src/**/*.rs"},
        ]},
        {"name": "tests", "phase": "check", "reads_tests": true},
        {"name": "lists", "phase": "check"},
    ])
}

/// The extension, served in process: each pass answers no finding.
pub fn gsd_facts_runtime() -> InProcessRuntime {
    let none = json!({"diagnostics": [], "summary": {}, "values": []})
        .to_string()
        .into_bytes();
    let mut runtime = InProcessRuntime::new().with(|| {
        let mut c = ContributionsBuilder::new(ExtensionMeta::new(FACTS_EXTENSION, "1.0.0"));
        c.raw_category(DeclaredCategory::Entities, gsd_facts_wire());
        c.raw_category(DeclaredCategory::Passes, gsd_facts_passes());
        c
    });
    for pass in ["files", "tests", "lists"] {
        runtime = runtime.answer_raw(
            FACTS_EXTENSION,
            &format!("__pass_{pass}"),
            WasmCallResult::Ok(none.clone()),
        );
    }
    runtime
}

/// A project enabling [`FACTS_EXTENSION`] with [`GSD_FACTS_SPEC`] and `src/lib.rs`; no report.
pub fn gsd_facts_project() -> TestProject {
    TestProject::new()
        .config(|c| c["name"] = "p".into())
        .enabling(&[FACTS_EXTENSION])
        .installing_enabled()
        .file("a.spec", GSD_FACTS_SPEC)
        .file("src/lib.rs", LIB_RS)
}

/// The input the last call of `pass` was sent, as JSON.
pub fn pass_input(runtime: &InProcessRuntime, pass: &str) -> Value {
    let export = format!("__pass_{pass}");
    runtime
        .calls()
        .into_iter()
        .rev()
        .find(|call| call.export == export)
        .unwrap_or_else(|| panic!("{export} was never called"))
        .input
}

/// [`FACTS_EXTENSION`]'s kinds and the `passes` (as the wire carries them), each answering no finding.
pub fn gsd_facts_serving(passes: Value) -> InProcessRuntime {
    let none = json!({"diagnostics": [], "summary": {}, "values": []})
        .to_string()
        .into_bytes();
    let names: Vec<String> = passes
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|pass| pass["name"].as_str().map(str::to_string))
        .collect();
    let mut runtime = InProcessRuntime::new().with(move || {
        let mut c = ContributionsBuilder::new(ExtensionMeta::new(FACTS_EXTENSION, "1.0.0"));
        c.raw_category(DeclaredCategory::Entities, gsd_facts_wire());
        c.raw_category(DeclaredCategory::Passes, passes.clone());
        c
    });
    for name in names {
        runtime = runtime.answer_raw(
            FACTS_EXTENSION,
            &format!("__pass_{name}"),
            WasmCallResult::Ok(none.clone()),
        );
    }
    runtime
}

/// [`gsd_facts_serving`] with one check pass `probe` declaring `reads` (as the wire carries them).
pub fn gsd_facts_probe(reads: Value) -> InProcessRuntime {
    gsd_facts_serving(json!([{"name": "probe", "phase": "check", "reads": reads}]))
}
