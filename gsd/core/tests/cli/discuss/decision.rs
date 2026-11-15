//! A decision's own relations: what it supersedes, what it affects, the alternatives it weighed and its record
//! fields (ADR 0359).

use super::{checks_clean, edges, fixture, has_edge};
use crate::harness::{Project, judged, project};

/// The report's errors and warnings: kernel and `gsd/` codes alike, the judged rules' among them unless each is one
/// [`Project::expected_judged`] names. An info (an invariant's pending `gsd/I536`) is no finding.
fn findings(p: &Project) -> Vec<(String, String)> {
    let report = p.report_all();
    assert_eq!(judged(&report), p.expected_judged(&[]), "{report:#?}");
    p.report()
        .into_iter()
        .filter(|(code, _)| {
            let local = code.strip_prefix("gsd/").unwrap_or(code);
            local.starts_with('E') || local.starts_with('W')
        })
        .collect()
}

/// An invariant, with the one field it must write, and decision `D-03` of phase 02 that affects it and TASK-01.
const AFFECTS: &str = "invariant tasks_never_lost \"Tasks are never lost\" {
  statement \"A task added is still there after a restart\"
}

decision D-03 \"Save after every change\" {
  phase   v1.0/02
  area    \"Storage\"
  text    \"The file is saved after every change\"
  origin  context
  affects [TASK-01, tasks_never_lost]
}
";

/// The later decision names the one it reverses, in its phase or from the project: D-01 reverses D-02 (written on
/// D-01 as ADR 0359's conversion row writes a `superseded_by D-01` held on D-02), and `supersedes [D-02]` resolves in
/// the phase's scope, `[/D-01]` from the project root.
#[test]
fn a_superseded_decision_resolves_in_its_phase_and_from_the_project() {
    let p = project("discuss");
    let own = edges(&p, "v1.0/02/D-01");
    assert!(
        has_edge(&own, "v1.0/02/D-01", "v1.0/02/D-02", "supersedes"),
        "{own:?}"
    );
    checks_clean(&p);
    let spec = fixture("discuss.spec");
    p.write(
        "spec/discuss.spec",
        &spec.replace("supersedes [D-02]", "supersedes [/D-01]"),
    );
    checks_clean(&p);
    let rooted = edges(&p, "v1.0/02/D-01");
    assert!(
        has_edge(&rooted, "v1.0/02/D-01", "D-01", "supersedes"),
        "{rooted:?}"
    );
}

/// A decision superseded with force no longer holds: its computed `in_force` is false and its successor's true, and
/// the planner's pack serves both with it, as gsd/E310 counts them (no selector filters on a computed field).
#[test]
fn a_superseded_decision_is_not_in_force_in_the_planners_pack() {
    let p = project("discuss");
    assert_eq!(p.fields_of("v1.0/02/D-02")["in_force"], false);
    assert_eq!(p.fields_of("v1.0/02/D-01")["in_force"], true);
    let run = p.run(&["context", "gsd/planner", "v1.0/02"]);
    assert_eq!(run.code, 0, "{}", run.all());
    let pack: serde_json::Value = serde_json::from_str(&run.stdout).expect("a pack");
    let in_force = |id: &str| {
        pack["graph"]["nodes"]
            .as_array()
            .and_then(|nodes| nodes.iter().find(|n| n["id"] == id))
            .map(|n| n["fields"]["in_force"].clone())
    };
    assert_eq!(in_force("v1.0/02/D-02"), Some(false.into()), "{pack}");
    assert_eq!(in_force("v1.0/02/D-01"), Some(true.into()), "{pack}");
}

/// `superseded_by` is read back, never written: the fixture's relation written the old way, on D-02, is E079 naming
/// `decision.supersedes`, with the edit.
#[test]
fn an_authored_superseded_by_is_e079_naming_supersedes() {
    let p = project("discuss");
    let written = "  text   \"The file is written to a temp name, then renamed\"\n";
    let spec = fixture("discuss.spec");
    assert!(spec.contains(written), "{spec}");
    p.write(
        "spec/discuss.spec",
        &spec
            .replace("  supersedes [D-02]\n", "")
            .replace(written, &format!("{written}  superseded_by D-01\n")),
    );
    let check = p.run(&["check"]);
    assert_eq!(check.code, 1, "{}", check.all());
    let report = p.report();
    assert!(
        report
            .iter()
            .any(|(code, message)| code == "E079" && message.contains("decision.supersedes")),
        "{report:#?}"
    );
    assert!(report.iter().all(|(code, _)| code != "W020"), "{report:#?}");
    assert!(check.all().contains("supersedes ["), "{}", check.all());
}

/// `new decision` writes a rooted id of a reference list as `link` writes it, spelled from the new decision's own
/// scope chain: `/D-01` from a phase decision keeps its slash (the phase's own D-01 would answer `D-01`), and
/// `/TASK-01` is `TASK-01`, which names the project's requirement from there too.
#[test]
fn new_writes_a_rooted_reference_as_link_does() {
    let p = project("discuss");
    let new = p.run(&[
        "new",
        "decision",
        "D-09",
        "Nine",
        "--in",
        "spec/discuss.spec",
        "--phase",
        "v1.0/02",
        "--origin",
        "context",
        "--text",
        "Nine",
        "--supersedes",
        "/D-01",
        "--affects",
        "/TASK-01",
    ]);
    assert_eq!(new.code, 0, "{}", new.all());
    for (field, target) in [("supersedes", "/D-01"), ("affects", "/TASK-01")] {
        let linked = p.run(&["link", "v1.0/02/D-02", field, target]);
        assert_eq!(linked.code, 0, "{}", linked.all());
    }
    let spec = p.read("spec/discuss.spec");
    let block = |id: &str| -> Vec<String> {
        let start = spec.find(&format!("decision {id} ")).expect("the block");
        spec[start..]
            .lines()
            .take_while(|line| *line != "}")
            .filter(|line| line.contains("supersedes") || line.contains("affects"))
            .map(|line| line.split_whitespace().collect::<Vec<_>>().join(" "))
            .collect()
    };
    assert_eq!(block("D-09"), ["supersedes [/D-01]", "affects [TASK-01]"]);
    assert_eq!(block("D-09"), block("D-02"));
    let own = edges(&p, "v1.0/02/D-09");
    assert!(
        has_edge(&own, "v1.0/02/D-09", "D-01", "supersedes"),
        "{own:?}"
    );
    assert!(
        has_edge(&own, "v1.0/02/D-09", "TASK-01", "affects"),
        "{own:?}"
    );
}

/// A typed edit of `superseded_by` is refused `other_side`, naming `supersedes`; the owning side writes.
#[test]
fn a_typed_edit_of_superseded_by_names_supersedes() {
    let p = project("discuss");
    for verb in ["set", "link"] {
        let refused = p.run(&[verb, "v1.0/02/D-02", "superseded_by", "v1.0/02/D-01"]);
        assert_ne!(refused.code, 0, "{verb}: {}", refused.all());
        assert!(
            refused.all().contains("other_side") && refused.all().contains("supersedes"),
            "{verb}: {}",
            refused.all()
        );
    }
    let linked = p.run(&["link", "v1.0/02/D-01", "supersedes", "/D-01"]);
    assert_eq!(linked.code, 0, "{}", linked.all());
    let own = edges(&p, "v1.0/02/D-01");
    for target in ["v1.0/02/D-02", "D-01"] {
        assert!(
            has_edge(&own, "v1.0/02/D-01", target, "supersedes"),
            "{target}: {own:?}"
        );
    }
    checks_clean(&p);
}

/// A decision affects a requirement and an invariant: both resolve, each reads it back, and nothing is found.
#[test]
fn a_decision_affects_a_requirement_and_an_invariant() {
    let p = project("discuss");
    p.write("spec/affects.spec", AFFECTS);
    assert_eq!(findings(&p), Vec::new());
    let own = edges(&p, "v1.0/02/D-03");
    for target in ["TASK-01", "tasks_never_lost"] {
        assert!(
            has_edge(&own, "v1.0/02/D-03", target, "affects"),
            "{target}: {own:?}"
        );
        let back = edges(&p, target);
        assert!(
            has_edge(&back, "v1.0/02/D-03", target, "affects"),
            "{target} reads it back: {back:?}"
        );
    }
}

/// `affected_by` is read back, never written: on a requirement it is E079 naming `decision.affects`, and a typed edit
/// of it is refused `other_side`.
#[test]
fn an_authored_affected_by_is_e079_naming_affects() {
    let p = project("discuss");
    p.write("spec/affects.spec", AFFECTS);
    p.edit(
        "spec/project.spec",
        "  text \"A user can add a task with a title\"\n",
        "  text \"A user can add a task with a title\"\n  affected_by [v1.0/02/D-03]\n",
    );
    let report = p.report();
    assert!(
        report
            .iter()
            .any(|(code, message)| code == "E079" && message.contains("decision.affects")),
        "{report:#?}"
    );
    assert!(report.iter().all(|(code, _)| code != "W020"), "{report:#?}");

    let p = project("discuss");
    p.write("spec/affects.spec", AFFECTS);
    let refused = p.run(&["link", "TASK-01", "affected_by", "v1.0/02/D-03"]);
    assert_ne!(refused.code, 0, "{}", refused.all());
    assert!(
        refused.all().contains("other_side") && refused.all().contains("affects"),
        "{}",
        refused.all()
    );
}

/// `specforge new decision --alternatives` takes the records as one JSON value; each keeps why it lost, and an
/// alternative with no name is refused, leaving the file as it was.
#[test]
fn an_alternative_keeps_why_it_lost() {
    let p = project("discuss");
    let new = |alternatives: &str| {
        p.run(&[
            "new",
            "decision",
            "D-04",
            "One JSON file",
            "--in",
            "spec/discuss.spec",
            "--phase",
            "v1.0/02",
            "--origin",
            "context",
            "--text",
            "One JSON file",
            "--alternatives",
            alternatives,
        ])
    };
    let before = p.read("spec/discuss.spec");
    let nameless = new(r#"[{"pros": ["queries"], "why_not": "No dependency in v1"}]"#);
    assert_ne!(nameless.code, 0, "{}", nameless.all());
    assert_eq!(
        p.read("spec/discuss.spec"),
        before,
        "a refused edit writes nothing"
    );

    let written = new(
        r#"[{"name": "SQLite", "pros": ["queries"], "cons": ["a dependency"], "why_not": "No dependency in v1"}]"#,
    );
    assert_eq!(written.code, 0, "{}", written.all());
    let alternatives = &p.fields_of("v1.0/02/D-04")["alternatives"];
    assert_eq!(alternatives[0]["name"], "SQLite", "{alternatives}");
    assert_eq!(
        alternatives[0]["why_not"], "No dependency in v1",
        "{alternatives}"
    );
    assert_eq!(alternatives[0]["cons"][0], "a dependency", "{alternatives}");
    checks_clean(&p);
}

/// `per_call_site_sandbox`, one of SpecForge's own governance decisions, in its converted form: a project decision whose
/// old `decision` is its `text`, `date` its `decided`, `invariants` its `affects`; nothing it did not hold (no tags,
/// no deciders).
const CONVERTED: &str = r#"invariant wasm_sandbox_integrity "Wasm Sandbox Integrity" {
  statement "Wasm extensions do not escape the Wasm sandbox"
}

decision per_call_site_sandbox "Per-Call-Site Sandbox" {
  text         """
    Host function permissions are enforced per export call site, not per
    extension. Each contribution type has a fixed permission set: validators
    get query_graph + emit_diagnostic, renderers additionally get emit_file,
    providers additionally get http_get, parsers get read_file +
    emit_diagnostic + add_graph_node + add_graph_edge (but NOT query_graph,
    since they run during graph construction). The runtime checks the current
    export context before dispatching host function calls.
  """
  context      """
    The contribution-based model means one extension can contribute entities,
    rendering, and provider validation. A per-extension sandbox policy is
    too coarse — an extension's validator should not have emit_file access
    just because the same extension also contributes a renderer. The
    principle of least privilege requires finer-grained permissions.
  """
  consequences [
    "Least-privilege per operation — validator cannot write files even if same extension has a renderer",
    "Defense in depth — compromised validator export cannot escalate to filesystem access",
    "Runtime must track which export is currently executing per call",
    "Permission model is static and predictable — no runtime configuration needed",
  ]
  settled      true
  origin       project
  decided      2026-03-03
  affects      [wasm_sandbox_integrity]
}
"#;

/// The converted decision checks with no error and no warning, and reads back its record fields.
#[test]
fn a_converted_governance_decision_checks_clean() {
    let p = project("discuss");
    p.write("spec/governance.spec", CONVERTED);
    assert_eq!(findings(&p), Vec::new());
    let fields = p.fields_of("per_call_site_sandbox");
    assert_eq!(fields["decided"], "2026-03-03", "{fields}");
    assert_eq!(
        fields["consequences"].as_array().map(Vec::len),
        Some(4),
        "{fields}"
    );
    assert!(
        fields["context"]
            .as_str()
            .is_some_and(|c| c.contains("A per-extension sandbox policy")),
        "{fields}"
    );
    assert!(fields.get("tags").is_none_or(|v| v.is_null()), "{fields}");
    let own = edges(&p, "per_call_site_sandbox");
    assert!(
        has_edge(
            &own,
            "per_call_site_sandbox",
            "wasm_sandbox_integrity",
            "affects"
        ),
        "{own:?}"
    );
}
