//! A field's type may be a list of words, a block, a union or a range, and a file field declares its base (ADR
//! 0285, plan K3): what a project with the typed fields of `Assembled::typed()` reads, links, checks and exports.

use specforge_common::Diagnostic;
use specforge_graph::{Graph, written_tokens};
use specforge_parser::{FieldValue, parse};
use specforge_project::testing::{Assembled, TYPED_EXTENSION, TestProject, typed_runtime};
use specforge_test_macros::test as specforge_test;
use std::sync::Arc;

/// Milestone `v1.0` with phases `02` and `03`, plan `03-02` of phase 03 and its summary.
const HIERARCHY: &str = "\
milestone v1.0 \"One\" {
}

phase 02 \"Two\" {
  milestone v1.0
}

phase 03 \"Three\" {
  milestone v1.0
}

plan 03-02 \"Plan\" {
  phase v1.0/03
  files_modified [\"src/a.ts\"]
}
";

/// A project over the typed extension with `source` as `a.spec`, compiled as every surface compiles it.
fn compile(source: &str) -> (Graph, Vec<Diagnostic>) {
    let dir = TestProject::new()
        .config(|c| c["name"] = "p".into())
        .enabling(&[TYPED_EXTENSION])
        .installing_enabled()
        .file("a.spec", source);
    let compiled = dir.compile_in(Arc::new(typed_runtime()));
    (compiled.graph().clone(), compiled.diagnostics())
}

fn coded<'d>(diagnostics: &'d [Diagnostic], code: &str) -> Vec<&'d Diagnostic> {
    diagnostics.iter().filter(|d| d.code == code).collect()
}

#[specforge_test(
    behavior = "check_field_value_types",
    verify = "an enum list takes the declared words, each once, and a scalar is a one-item list"
)]
fn an_enum_list_takes_declared_words_once() {
    let (graph, diagnostics) = compile(
        "refactor_proposal r1 \"R\" {\n  reasons [threshold, jump]\n}\n\
         refactor_proposal r2 \"R\" {\n  reasons jump\n}\n",
    );
    assert!(coded(&diagnostics, "E061").is_empty(), "{diagnostics:#?}");
    let words = |id: &str| match graph.node(id).unwrap().fields.get("reasons") {
        Some(FieldValue::List(items)) => items.iter().map(|i| i.text.clone()).collect::<Vec<_>>(),
        other => panic!("{other:?}"),
    };
    assert_eq!(words("r1"), ["threshold", "jump"]);
    assert_eq!(words("r2"), ["jump"]);

    let (_, diagnostics) = compile(
        "refactor_proposal r1 \"R\" {\n  reasons [thresold]\n}\n\
         refactor_proposal r2 \"R\" {\n  reasons [jump, jump]\n}\n",
    );
    let e061 = coded(&diagnostics, "E061");
    assert_eq!(e061.len(), 2, "{diagnostics:#?}");
    assert_eq!(
        e061[0].suggestion.as_deref(),
        Some("did you mean 'threshold'?")
    );
    assert!(
        e061[1].message.contains("'jump' is written twice")
            || e061[1]
                .suggestion
                .as_deref()
                .is_some_and(|s| s.contains("written twice"))
    );
}

#[specforge_test(
    behavior = "check_field_value_types",
    verify = "a typed block reads and admits each member by its own declared type"
)]
fn a_typed_block_coerces_and_admits_each_member() {
    let (graph, diagnostics) =
        compile("discussion d1 \"D\" {\n  scores { goal 03-02 ambiguity 7 }\n}\n");
    assert!(coded(&diagnostics, "E061").is_empty(), "{diagnostics:#?}");
    let FieldValue::Block(scores) = graph.node("d1").unwrap().fields.get("scores").unwrap() else {
        panic!("a block");
    };
    assert!(
        matches!(scores.get("goal"), Some(FieldValue::Integer(1))),
        "{scores:?}"
    );
}

#[specforge_test(
    behavior = "check_field_value_types",
    verify = "a typed block reads and admits each member by its own declared type"
)]
fn a_member_of_a_typed_block_is_admitted_at_its_own_span() {
    let (_, diagnostics) = compile("discussion d1 \"D\" {\n  scores { goal \"x\" }\n}\n");
    let e061 = coded(&diagnostics, "E061");
    assert_eq!(e061.len(), 1, "{diagnostics:#?}");
    assert!(
        e061[0].message.contains("scores.goal"),
        "{}",
        e061[0].message
    );
    let span = e061[0].span.as_ref().unwrap();
    assert_eq!(
        (span.start_line, span.start_col),
        (2, 17),
        "the member's own span"
    );
}

#[specforge_test(
    behavior = "check_field_value_types",
    verify = "an integer outside its declared minimum or maximum is E061 naming the bounds"
)]
fn an_integer_outside_its_bounds_is_e061() {
    let (_, diagnostics) = compile("truth t1 \"T\" {\n  criterion 9\n}\n");
    let e061 = coded(&diagnostics, "E061");
    assert_eq!(e061.len(), 1, "{diagnostics:#?}");
    assert_eq!(
        e061[0].suggestion.as_deref(),
        Some("write a whole number from 1 to 5")
    );
}

#[specforge_test(
    behavior = "check_field_value_types",
    verify = "an integer outside its declared minimum or maximum is E061 naming the bounds"
)]
fn a_bounded_member_is_e061_at_its_own_span() {
    let (_, diagnostics) = compile("discussion d1 \"D\" {\n  scores { goal 120 }\n}\n");
    let e061 = coded(&diagnostics, "E061");
    assert_eq!(e061.len(), 1, "{diagnostics:#?}");
    assert_eq!(
        e061[0].suggestion.as_deref(),
        Some("write a whole number from 0 to 100")
    );
    let span = e061[0].span.as_ref().unwrap();
    assert_eq!(
        (span.start_line, span.start_col),
        (2, 17),
        "the member's own span"
    );
}

#[specforge_test(
    behavior = "link_derived_references",
    verify = "an enum list's words and a typed block's members are no type expression: no derived edge and no E004"
)]
fn a_typed_block_on_a_type_kind_is_no_type_expression() {
    let (graph, diagnostics) = compile(&format!(
        "{HIERARCHY}\nshape s1 \"S\" {{\n  phase v1.0/03\n  tags [cli, api]\n  checks [{{ kind cli note \"x\" }}]\n  g Missing\n}}\n"
    ));
    assert!(coded(&diagnostics, "E001").is_empty(), "{diagnostics:#?}");
    let e004 = coded(&diagnostics, "E004");
    assert_eq!(e004.len(), 1, "{diagnostics:#?}");
    assert!(e004[0].message.contains("Missing"), "{}", e004[0].message);
    assert!(
        graph.edges().iter().all(|e| e.label.as_str() != "parts"),
        "no derived edge"
    );
}

/// The summary of plan `03-02` requiring phase `02`, which the summary's chain reaches in milestone `v1.0`.
const REQUIRING: &str = "\
summary 03-02-S \"S\" {
  plan v1.0/03/03-02
  requires [
    { phase 02 provides \"x\" }
    { phase 03 provides \"y\" }
  ]
}
";

#[specforge_test(
    behavior = "link_entity_references",
    verify = "a reference inside a block is held by its entity, resolves outward from it and is labelled by its member path"
)]
fn a_reference_in_a_block_links_from_its_entity() {
    let (graph, diagnostics) = compile(&format!("{HIERARCHY}\n{REQUIRING}"));
    assert!(coded(&diagnostics, "E003").is_empty(), "{diagnostics:#?}");
    let edges: Vec<_> = graph
        .edges()
        .iter()
        .filter(|e| e.label.as_str() == "requires.phase")
        .map(|e| (e.source.to_string(), e.target.to_string()))
        .collect();
    assert_eq!(
        edges,
        [
            ("v1.0/03/03-02/03-02-S".to_string(), "v1.0/02".to_string()),
            ("v1.0/03/03-02/03-02-S".to_string(), "v1.0/03".to_string()),
        ]
    );

    let (_, diagnostics) = compile(&format!(
        "{HIERARCHY}\nsummary 03-02-S \"S\" {{\n  plan v1.0/03/03-02\n  requires [{{ phase 99 provides \"x\" }}]\n}}\n"
    ));
    assert_eq!(coded(&diagnostics, "E003").len(), 1, "{diagnostics:#?}");
}

#[specforge_test(
    behavior = "link_entity_references",
    verify = "a reference field declaring several target kinds takes an entity of any of them and refuses another, naming each"
)]
fn a_union_reference_takes_any_listed_kind() {
    let (graph, diagnostics) = compile(&format!(
        "{HIERARCHY}\nverify_gap VG1 \"G\" {{\n}}\nuat_gap G-03-1 \"G\" {{\n}}\n\
         plan 03-03 \"Other\" {{\n  phase v1.0/03\n  files_modified [\"src/a.ts\"]\n  gap_closure [VG1, G-03-1]\n}}\n"
    ));
    assert!(coded(&diagnostics, "E022").is_empty(), "{diagnostics:#?}");
    assert_eq!(
        graph
            .edges()
            .iter()
            .filter(|e| e.label.as_str() == "gap_closure")
            .count(),
        2
    );

    let (_, diagnostics) = compile(&format!(
        "{HIERARCHY}\nplan 03-03 \"Other\" {{\n  phase v1.0/03\n  files_modified [\"src/a.ts\"]\n  gap_closure [03-02]\n}}\n"
    ));
    let e022 = coded(&diagnostics, "E022");
    assert_eq!(e022.len(), 1, "{diagnostics:#?}");
    assert!(
        e022[0].message.contains("verify_gap") && e022[0].message.contains("uat_gap"),
        "{}",
        e022[0].message
    );
}

#[specforge_test(
    behavior = "build_in_memory_graph",
    verify = "the finder reports a reference inside a block under its member path, held by the entity"
)]
fn the_finder_holds_member_tokens_by_their_entity() {
    let project = Assembled::typed();
    let file = parse(&format!("{HIERARCHY}\n{REQUIRING}"), "a.spec");
    let build = project.compiled().environment().graph_of([file.clone()]);
    let tokens = written_tokens(&file, build.inputs(), build.graph().scopes());
    let token = tokens
        .iter()
        .find(|t| t.target.as_str() == "v1.0/02")
        .expect("the token of `02`");
    assert_eq!(token.field.as_str(), "requires.phase");
    assert_eq!(token.held_by, ["v1.0/03/03-02/03-02-S"]);
}

#[specforge_test(
    behavior = "check_field_value_types",
    verify = "E061, E006, W020 and the declarative rules reach every member of every item of a typed block"
)]
fn checks_reach_block_members() {
    let (_, diagnostics) = compile(
        "project p1 \"P\" {\n  constraints [\n    { kind timelin what \"a\" }\n    { kind timeline what \"b\" whyy \"c\" }\n  ]\n}\n\
         milestone v1.0 \"One\" {\n}\nphase 03 \"Three\" {\n  milestone v1.0\n}\n\
         plan 03-02 \"Plan\" {\n  phase v1.0/03\n  files_modified [\"src/a.ts\"]\n  coupling_justified [{ peer 03-02 }]\n}\n",
    );
    let e061 = coded(&diagnostics, "E061");
    assert_eq!(e061.len(), 1, "{diagnostics:#?}");
    assert!(
        e061[0].message.contains("constraints[0].kind"),
        "{}",
        e061[0].message
    );
    let span = e061[0].span.as_ref().unwrap();
    assert_eq!(
        (span.start_line, span.start_col),
        (3, 12),
        "the member's own span"
    );
    let w020 = coded(&diagnostics, "W020");
    assert_eq!(w020.len(), 1, "{diagnostics:#?}");
    assert!(w020[0].message.contains("'whyy'"), "{}", w020[0].message);
    assert_eq!(w020[0].suggestion.as_deref(), Some("did you mean 'why'?"));
    let e006 = coded(&diagnostics, "E006");
    assert_eq!(e006.len(), 1, "{diagnostics:#?}");
    assert!(
        e006[0].message.contains("'reason'") && e006[0].message.contains("coupling_justified[0]"),
        "{}",
        e006[0].message
    );
}

#[specforge_test(
    behavior = "validate_file_reference_paths",
    verify = "a file field declaring the project base reads its relative paths from the directory of specforge.json"
)]
fn a_project_file_field_reads_from_the_project_root() {
    let dir = TestProject::new()
        .config(|c| {
            c["name"] = "p".into();
            c["spec_root"] = "spec".into();
        })
        .enabling(&[TYPED_EXTENSION])
        .installing_enabled()
        .file("spec/a.spec", format!(
            "{HIERARCHY}\nplan 03-03 \"Other\" {{\n  phase v1.0/03\n  files_modified [\"src/b.ts\", \"src/c.ts\"]\n}}\n\
             context c1 \"C\" {{\n  canonical_refs [{{ topic \"t\" paths [\"src/mising.ts\"] }}]\n}}\n"))
        .file("src/a.ts", "")
        .file("src/b.ts", "")
        .file("src/missing.ts", "");
    let compiled = dir.compile_in(Arc::new(typed_runtime()));
    let diagnostics = compiled.diagnostics();
    let mut e016 = coded(&diagnostics, "E016");
    e016.sort_by(|a, b| a.message.cmp(&b.message));
    let missing: Vec<_> = e016.iter().map(|d| d.message.as_str()).collect();
    assert_eq!(missing.len(), 2, "{diagnostics:#?}");
    assert!(missing[0].contains("'src/c.ts'"), "{missing:?}");
    assert!(missing[1].contains("'src/mising.ts'"), "{missing:?}");
    assert_eq!(
        e016[1].suggestion.as_deref(),
        Some("did you mean 'src/missing.ts'?")
    );
}

#[specforge_test(
    behavior = "run_check_phase_passes",
    verify = "a pass receives a typed block and a block list structured beside their text"
)]
fn a_pass_receives_a_block_structured() {
    let dir = TestProject::new()
        .config(|c| c["name"] = "p".into())
        .enabling(&[TYPED_EXTENSION])
        .installing_enabled()
        .file(
            "a.spec",
            format!(
                "{HIERARCHY}\nplan 03-03 \"Other\" {{\n  phase v1.0/03\n  files_modified [\"src/a.ts\"]\n  \
                 coupling_justified [{{ peer 03-02 reason \"shared\" }}]\n}}\n"
            ),
        );
    let runtime = Arc::new(typed_runtime());
    dir.compile_in(runtime.clone());
    let input = runtime
        .calls()
        .into_iter()
        .rev()
        .find(|c| c.export == "__pass_observe")
        .map(|c| c.input)
        .expect("the pass ran");
    let plan = input["entities"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["id"] == "v1.0/03/03-03")
        .expect("the plan");
    assert_eq!(
        plan["values"]["coupling_justified"],
        serde_json::json!([{"peer": "03-02", "reason": "shared"}])
    );
    assert!(
        plan["fields"]["coupling_justified"].is_string(),
        "the text is unchanged"
    );
    // A list field is its items too (ADR 0294); a scalar still has no entry.
    assert_eq!(
        plan["values"]["files_modified"],
        serde_json::json!(["src/a.ts"])
    );
    assert!(
        plan["values"].get("phase").is_none(),
        "a scalar is its text"
    );
}

#[specforge_test(
    behavior = "serialize_json_graph",
    verify = "a block list is exported as an array of objects and the Graph Protocol schema describes its members"
)]
fn an_export_writes_blocks_and_their_schema() {
    let (graph, _) = compile(&format!(
        "{HIERARCHY}\nplan 03-03 \"Other\" {{\n  phase v1.0/03\n  files_modified [\"src/a.ts\"]\n  \
         coupling_justified [{{ peer 03-02 reason \"shared\" }} {{ peer 03-02 reason \"again\" }}]\n}}\n\
         refactor_proposal r1 \"R\" {{\n  reasons [threshold, jump]\n}}\n"
    ));
    let json: serde_json::Value = serde_json::from_str(
        &specforge_emitter::emit(&graph, &specforge_emitter::EmitOptions::default()).unwrap(),
    )
    .unwrap();
    let field = |id: &str, name: &str| {
        json["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .find(|n| n["id"] == id)
            .unwrap()["fields"][name]
            .clone()
    };
    assert_eq!(
        field("v1.0/03/03-03", "coupling_justified"),
        serde_json::json!([
            {"peer": "03-02", "reason": "shared"},
            {"peer": "03-02", "reason": "again"},
        ])
    );
    assert_eq!(
        field("r1", "reasons"),
        serde_json::json!(["threshold", "jump"])
    );

    let registries =
        specforge_registry::build_registries(vec![specforge_project::testing::typed_extension()]);
    let schema = specforge_emitter::generate_schema(&registries);
    let schema_json = serde_json::to_value(&schema).unwrap();
    let summary = schema_json["entity_kinds"]
        .as_array()
        .unwrap()
        .iter()
        .find(|k| k["name"] == "summary")
        .unwrap();
    let requires = summary["fields"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["name"] == "requires")
        .unwrap();
    assert_eq!(requires["field_type"], "block_list");
    assert_eq!(requires["members"].as_array().unwrap().len(), 2);
    let published = specforge_emitter::publish(&schema, specforge_emitter::EmitFormat::Json)
        .expect("a published schema")
        .to_string();
    assert!(
        published.contains("\"required\":[\"phase\",\"provides\"]"),
        "{published}"
    );
}

#[specforge_test(
    behavior = "check_field_value_types",
    verify = "a member written in another form than its type gives is W159 at the member, named by its path"
)]
fn an_off_form_member_is_w159_at_the_member() {
    let (_, diagnostics) = compile("discussion d1 \"D\" {\n  scores { goal \"3\" }\n}\n");
    let w159 = coded(&diagnostics, "W159");
    assert_eq!(w159.len(), 1, "{diagnostics:#?}");
    assert!(
        w159[0].message.contains("member 'scores.goal'"),
        "{}",
        w159[0].message
    );
    assert!(coded(&diagnostics, "E061").is_empty(), "{diagnostics:#?}");
}
