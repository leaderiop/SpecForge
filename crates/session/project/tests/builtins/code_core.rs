//! `@code/core`'s check passes over a project (ADR 0329 D5, D6): the builtin's component loads beside the in-process
//! `Collected` fixture peering it (`@t/node` collects, `@t/spec`'s `truth` cites `test`), and `@t/owner` scopes a
//! citing `step` by a `plan` complete on `status ∈ [complete]` and declares `api`, whose `items` cites `item` facts
//! (ADR 0347 D9), and `change`, whose `items_changed` the harness writes once (a history reference, ADR 0352 D2).

use std::sync::Arc;

use serde_json::{Value, json};
use specforge_extension_sdk::prelude::{
    ContributionsBuilder, ExtensionMeta, FieldType, Vocabulary,
};
use specforge_project::CompiledProject;
use specforge_project::testing::{Collected, NODE, SPEC, TestProject, component_runtime, stored};
use specforge_wasm::testing::InProcessRuntime;

const CODE_CORE: &str = "@code/core";
const OWNER: &str = "@t/owner";
const KEY: &str = "0123456789abcdef0123456789abcdef01234567";
const ADDS: &str = "js:app/test/a.test.js#board > adds";

/// `node`'s store: the test `ADDS`, its last run giving `status`, naming `names`, and its run.
fn adds(status: &str, names: &[&str]) -> Value {
    json!([
        {"kind": "test", "id": ADDS, "title": "board > adds",
         "fields": {"package": "app", "file": "test/a.test.js", "suite": ["board"], "name": "adds", "status": status,
                    "names": names}},
        {"kind": "test_run", "id": format!("run:{ADDS}"), "fields": {"test": ADDS, "status": status}},
    ])
}

/// `@t/owner`: a `plan` complete on `status ∈ [complete]`, a `step` it scopes that cites tests in `proof`, and an
/// `api` that cites items in `items`.
fn owner() -> ContributionsBuilder {
    let mut c = ContributionsBuilder::new(ExtensionMeta::new(OWNER, "1.0.0"));
    c.peer(CODE_CORE, "^1.0");
    c.kind("plan", |k| {
        k.complete_when("status", &["complete"])
            .field("status", |f| {
                f.vocabulary(&Vocabulary::new(&["open", "complete"]));
            });
    });
    c.kind("step", |k| {
        k.scoped_by("plan")
            .field("plan", |f| {
                f.field_type(FieldType::Reference).target_kind("plan");
            })
            .field("proof", |f| {
                f.field_type(FieldType::ReferenceList).target_kind("test");
            });
    });
    c.kind("api", |k| {
        k.field("items", |f| {
            f.field_type(FieldType::ReferenceList).target_kind("item");
        });
    });
    c.kind("change", |k| {
        k.field("items_changed", |f| {
            f.field_type(FieldType::ReferenceList)
                .target_kind("item")
                .harness()
                .immutable();
        });
    });
    c
}

/// A source pack's store: the Rust items `ids`, each a function of `src/lib.rs`.
fn items(ids: &[&str]) -> Value {
    ids.iter()
        .map(|id| {
            json!({"kind": "item", "id": id,
                   "fields": {"file": "src/lib.rs", "kind": ["function"], "visibility": "public"}})
        })
        .collect()
}

/// An api citing `cited` in `items`, its reference on line 2.
fn api(cited: &str) -> String {
    format!("api A1 \"One\" {{\n  items [\"{cited}\"]\n}}\n")
}

/// A project enabling `@code/core`, the fixture peering it and `@t/owner`, with `files`.
fn project(files: &[(&str, &str)]) -> TestProject {
    TestProject::new()
        .config(|c| c["name"] = "p".into())
        .enabling(&[CODE_CORE, NODE, SPEC, OWNER])
        .installing_enabled()
        .files(files)
}

/// `p` compiled with `@code/core`'s component beside the fixture.
fn compiled(p: &TestProject) -> CompiledProject {
    let beside = InProcessRuntime::new()
        .with(owner)
        .beside(component_runtime());
    p.compile_in(Arc::new(Collected::new().peering(CODE_CORE).over(beside)))
}

/// `@code/core`'s findings: `(code, entity, message)`.
fn about(p: &CompiledProject) -> Vec<(String, String, String)> {
    p.diagnostics()
        .iter()
        .filter(|d| d.code.as_str().starts_with("code/"))
        .map(|d| {
            let entity = d.subject().map(str::to_string).unwrap_or_default();
            (d.code.to_string(), entity, d.message.clone())
        })
        .collect()
}

/// `@code/core`'s findings: `(code, line, suggestion)`.
fn found(p: &CompiledProject) -> Vec<(String, usize, Option<String>)> {
    p.diagnostics()
        .iter()
        .filter(|d| d.code.as_str().starts_with("code/"))
        .map(|d| {
            (
                d.code.to_string(),
                d.span.as_ref().map_or(0, |s| s.start_line),
                d.suggestion.clone(),
            )
        })
        .collect()
}

/// A truth citing `cited` in `proof`, its reference on line 2.
fn citing(cited: &str) -> String {
    format!("truth T1 \"One\" {{\n  proof [\"{cited}\"]\n}}\n")
}

#[specforge_test_macros::test(requirement = "FEATURE-003")]
#[specforge_test_macros::test(requirement = "FEATURE-004")]
fn a_fact_ref_to_a_missing_test_is_code_e101() {
    let p = project(&[("truths.spec", &citing("js:app/test/a.test.js#board > add"))]);
    stored(p.root(), "node", KEY, adds("passed", &[]));

    let compiled = compiled(&p);

    assert_eq!(
        found(&compiled),
        [(
            "code/E101".to_string(),
            2,
            Some(format!("did you mean `{ADDS}`?"))
        )],
        "{:?}",
        compiled.diagnostics()
    );
}

#[specforge_test_macros::test(requirement = "FEATURE-003")]
#[specforge_test_macros::test(requirement = "FEATURE-004")]
fn a_fact_ref_with_no_facts_collected_is_code_w102() {
    let p = project(&[("truths.spec", &citing(ADDS))]);

    let compiled = compiled(&p);

    assert_eq!(
        found(&compiled),
        [("code/W102".to_string(), 2, None)],
        "{:?}",
        compiled.diagnostics()
    );
}

#[specforge_test_macros::test(requirement = "FEATURE-003")]
#[specforge_test_macros::test(requirement = "FEATURE-004")]
fn a_fact_ref_of_a_pending_owner_is_unchecked() {
    let plans = format!(
        "plan open \"Open\" {{\n  status open\n}}\n\nplan done \"Done\" {{\n  status complete\n}}\n\n\
         step later \"Later\" {{\n  plan open\n  proof [\"{ADDS}\"]\n}}\n\n\
         step now \"Now\" {{\n  plan done\n  proof [\"{ADDS}\"]\n}}\n"
    );
    let p = project(&[("plans.spec", &plans)]);

    let compiled = compiled(&p);

    // Only the complete plan's step is checked: its reference is on line 16.
    assert_eq!(
        found(&compiled),
        [("code/W102".to_string(), 16, None)],
        "{:?}",
        compiled.diagnostics()
    );
}

#[specforge_test_macros::test(requirement = "FEATURE-003")]
#[specforge_test_macros::test(requirement = "FEATURE-004")]
fn a_held_fact_ref_links_and_reports_nothing() {
    let p = project(&[("truths.spec", &citing(ADDS))]);
    stored(p.root(), "node", KEY, adds("passed", &[]));

    let compiled = compiled(&p);

    assert_eq!(found(&compiled), [], "{:?}", compiled.diagnostics());
    assert!(
        compiled
            .graph()
            .edges_from("T1")
            .iter()
            .any(|e| e.target.as_str() == ADDS),
        "T1 links the test"
    );
}

#[specforge_test_macros::test(requirement = "FEATURE-004")]
fn a_failing_proof_is_code_w401() {
    let plans = format!(
        "{}\nplan open \"Open\" {{\n  status open\n}}\n\nstep later \"Later\" {{\n  plan open\n  proof [\"{ADDS}\"]\n}}\n",
        citing(ADDS)
    );
    let p = project(&[("truths.spec", &plans)]);
    stored(p.root(), "node", KEY, adds("failed", &[]));

    let compiled = compiled(&p);

    // The pending step links the failed test too, and is not reported.
    assert_eq!(
        about(&compiled),
        [(
            "code/W401".to_string(),
            "T1".to_string(),
            format!("`T1` links the test `{ADDS}`, whose last collected run failed")
        )],
        "{:?}",
        compiled.diagnostics()
    );
}

#[specforge_test_macros::test(requirement = "FEATURE-004")]
fn a_test_naming_a_missing_entity_is_code_i402() {
    let p = project(&[("truths.spec", &citing(ADDS))]);
    stored(p.root(), "node", KEY, adds("passed", &["T1", "T9"]));

    let compiled = compiled(&p);

    assert_eq!(
        about(&compiled),
        [(
            "code/I402".to_string(),
            ADDS.to_string(),
            format!("the test `{ADDS}` names `T9`, which is no entity of the project")
        )],
        "{:?}",
        compiled.diagnostics()
    );
}

#[specforge_test_macros::test(requirement = "FEATURE-003")]
#[specforge_test_macros::test(requirement = "FEATURE-004")]
fn a_test_runs_dangling_test_reference_is_code_e101_at_the_fact() {
    let p = project(&[("truths.spec", &citing(ADDS))]);
    let mut facts = adds("passed", &[]);
    facts.as_array_mut().unwrap().push(json!({
        "kind": "test_run", "id": "run:zz", "fields": {"test": "zz", "status": "passed"},
        "at": [{"file": "test/a.test.js", "start_line": 7, "start_col": 1, "end_line": 7, "end_col": 20}],
    }));
    stored(p.root(), "node", KEY, facts);

    let compiled = compiled(&p);

    let codes: Vec<String> = compiled
        .diagnostics()
        .iter()
        .map(|d| d.code.to_string())
        .collect();
    assert!(
        !codes.iter().any(|c| c == "E022" || c == "E003"),
        "{:?}",
        compiled.diagnostics()
    );
    assert_eq!(
        about(&compiled),
        [(
            "code/E101".to_string(),
            "run:zz".to_string(),
            "the fact 'run:zz' names 'zz', which no collected fact holds (its collector's facts disagree)"
                .to_string()
        )],
        "{:?}",
        compiled.diagnostics()
    );
    let finding = compiled
        .diagnostics()
        .into_iter()
        .find(|d| d.code == "code/E101")
        .unwrap();
    let span = finding.span.as_ref().expect("placed at the fact");
    assert_eq!((span.file.as_str(), span.start_line), ("test/a.test.js", 7));
}

#[specforge_test_macros::test(requirement = "FEATURE-003")]
#[specforge_test_macros::test(requirement = "FEATURE-004")]
fn a_rust_test_ref_beside_only_item_facts_is_code_w102() {
    let p = project(&[("truths.spec", &citing("rust:toy#tests::adds"))]);
    stored(
        p.root(),
        "rust-source",
        KEY,
        items(&["rust:toy::tests::adds"]),
    );

    let compiled = compiled(&p);

    assert_eq!(
        about(&compiled),
        [(
            "code/W102".to_string(),
            "T1".to_string(),
            "`proof` names `rust:toy#tests::adds`, but no `rust` `test` facts were collected: run \
             `specforge collect` with `@rust/test`"
                .to_string()
        )],
        "{:?}",
        compiled.diagnostics()
    );
}

#[specforge_test_macros::test(requirement = "FEATURE-003")]
#[specforge_test_macros::test(requirement = "FEATURE-004")]
fn a_missing_item_of_a_collected_crate_is_code_e101() {
    let p = project(&[("apis.spec", &api("rust:toy::Board::ad"))]);
    stored(
        p.root(),
        "rust-source",
        KEY,
        items(&["rust:toy::Board::add"]),
    );

    let compiled = compiled(&p);

    assert_eq!(
        found(&compiled),
        [("code/E101".to_string(), 2, None)],
        "{:?}",
        compiled.diagnostics()
    );
    assert_eq!(
        about(&compiled)[0].2,
        "`items` names `rust:toy::Board::ad`, which no collected `rust` `item` fact holds"
    );
}

#[specforge_test_macros::test(requirement = "FEATURE-003")]
#[specforge_test_macros::test(requirement = "FEATURE-004")]
fn an_item_ref_links_to_its_fact() {
    let p = project(&[("apis.spec", &api("rust:toy::Board::add"))]);
    stored(
        p.root(),
        "rust-source",
        KEY,
        items(&["rust:toy::Board::add"]),
    );

    let compiled = compiled(&p);

    assert_eq!(found(&compiled), [], "{:?}", compiled.diagnostics());
    assert!(
        compiled
            .graph()
            .edges_from("A1")
            .iter()
            .any(|e| e.target.as_str() == "rust:toy::Board::add"),
        "A1 links the item"
    );
}

/// An item the harness recorded once and the code has since renamed is history: what held when it was written, never
/// a finding, where an authored reference to it is `code/E101`.
#[specforge_test_macros::test(requirement = "FEATURE-003")]
#[specforge_test_macros::test(requirement = "FEATURE-004")]
fn a_harness_immutable_reference_is_no_finding() {
    let change = "change C1 \"One\" {\n  items_changed [\"rust:toy::Board::ad\"]\n}\n";
    let p = project(&[("changes.spec", change)]);
    stored(
        p.root(),
        "rust-source",
        KEY,
        items(&["rust:toy::Board::add"]),
    );

    let compiled = compiled(&p);

    assert_eq!(found(&compiled), [], "{:?}", compiled.diagnostics());
}

/// The project package the report facts below cite.
const TOY: &str = "pkg:cargo/toy";

/// The item `rust:toy::add` and the package [`TOY`], then the report facts `reports`: one store, as collected.
fn with_reports(reports: Value) -> Value {
    let mut facts = items(&["rust:toy::add"]);
    let all = facts.as_array_mut().unwrap();
    all.push(json!({"kind": "package", "id": TOY, "fields": {"name": "toy", "path": ""}}));
    all.extend(reports.as_array().unwrap().iter().cloned());
    facts
}

/// One report fact of each kind: unmappable when `mapped` is false (no item, no package, no purl, unknown rule and
/// severity), else naming its item and purl, with a known rule and severity.
fn reports(mapped: bool) -> Value {
    let item = |mut fields: Value| {
        if mapped {
            fields["item"] = "rust:toy::add".into();
        }
        fields
    };
    let (rule, severity) = if mapped {
        ("needless_return", "high")
    } else {
        ("unknown", "unknown")
    };
    let mut dependency =
        json!({"package": TOY, "name": "serde", "version": "1.0.0", "scope": "required"});
    if mapped {
        dependency["purl"] = "pkg:cargo/serde@1.0.0".into();
    }
    json!([
        {"kind": "coverage", "id": "cov:@src/gen.rs",
         "fields": item(json!({"file": "src/gen.rs", "lines_found": 3, "lines_hit": 1}))},
        {"kind": "api_item", "id": "api:rust:toy::add",
         "fields": item(json!({"path": "toy::add", "kind": "function", "visibility": "public",
                              "signature": "pub fn toy::add()"}))},
        {"kind": "static_finding", "id": "sarif:clippy/rule",
         "fields": item(json!({"tool": "clippy", "rule": rule, "level": "warning", "message": "m"}))},
        {"kind": "dependency", "id": "dep:pkg:cargo/toy/serde", "fields": dependency},
        {"kind": "vulnerability", "id": "vuln:pkg:cargo/toy/RUSTSEC-2099-0001",
         "fields": {"package": TOY, "osv_id": "RUSTSEC-2099-0001", "aliases": [],
                    "purl": ["pkg:cargo/serde@1.0.0"], "version": ["1.0.0"], "severity": severity}},
    ])
}

/// The `reports` pass (ADR 0388) raises each code on its class only, once per fact, naming the fact: W104 on the
/// coverage, api_item and finding naming no item, W105 on the finding's unknown rule, W106 on the purl-less
/// dependency, W107 on the vulnerability of unknown severity.
#[specforge_test_macros::test(requirement = "FEATURE-003")]
fn the_reports_pass_warns_once_per_unmappable_report_fact() {
    let p = project(&[]);
    stored(p.root(), "rust-source", KEY, with_reports(reports(false)));

    let compiled = compiled(&p);

    let mut found: Vec<(String, String)> = about(&compiled)
        .into_iter()
        .map(|(code, entity, message)| {
            assert!(message.contains(&entity), "{message} names {entity}");
            (code, entity)
        })
        .collect();
    found.sort();
    let expected = [
        ("code/W104", "api:rust:toy::add"),
        ("code/W104", "cov:@src/gen.rs"),
        ("code/W104", "sarif:clippy/rule"),
        ("code/W105", "sarif:clippy/rule"),
        ("code/W106", "dep:pkg:cargo/toy/serde"),
        ("code/W107", "vuln:pkg:cargo/toy/RUSTSEC-2099-0001"),
    ]
    .map(|(code, entity)| (code.to_string(), entity.to_string()));
    assert_eq!(found, expected, "{:?}", compiled.diagnostics());
}

/// Report facts naming their item and purl, with a known rule and severity, raise no report code (ADR 0388).
#[specforge_test_macros::test(requirement = "FEATURE-003")]
fn mapped_report_facts_get_no_report_warning() {
    let p = project(&[]);
    stored(p.root(), "rust-source", KEY, with_reports(reports(true)));

    let compiled = compiled(&p);

    assert_eq!(about(&compiled), [], "{:?}", compiled.diagnostics());
}

/// A project storing no report fact gets no report code (ADR 0388).
#[specforge_test_macros::test(requirement = "FEATURE-003")]
fn a_project_with_no_report_facts_gets_no_report_warning() {
    let p = project(&[]);
    stored(p.root(), "rust-source", KEY, with_reports(json!([])));

    let compiled = compiled(&p);

    assert_eq!(about(&compiled), [], "{:?}", compiled.diagnostics());
}
