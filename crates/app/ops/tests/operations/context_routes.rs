//! `context`, a context route served for one entity (ADR 0356): the GSD-shaped executor and plan-checker routes over
//! the taskboard twin's plan `v1.0/02/02-01`, a budgeted read, the refusals, and the pack digest.

use std::sync::Arc;

use serde_json::{Value, json};
use specforge_ops::read::context::{
    ContextField, ContextRequest, Pack, ROUTE_OF_ANOTHER_KIND, UNKNOWN_ROUTE, context,
};
use specforge_ops::read::export::Format;
use specforge_ops::view::ProjectView;
use specforge_project::testing::{
    CODE_CORE, CONTEXT_INSTRUCTIONS, CONTEXT_PLAN, GSD_CONTEXT_SPEC, GSD_JUDGED, TestProject,
    gsd_context_project, gsd_context_runtime, gsd_routed_project_over,
};
use specforge_project::{CompiledProject, SharedRuntime};
use specforge_test_macros::test as specforge_test;
use specforge_wasm::WasmCallResult;

fn compile(project: &TestProject) -> CompiledProject {
    project.compile_in(Arc::new(gsd_context_runtime()) as SharedRuntime)
}

fn request<'a>(route: &'a str, subject: &'a str, max_tokens: Option<usize>) -> ContextRequest<'a> {
    ContextRequest {
        route,
        subject,
        max_tokens,
        format: Format::Context,
    }
}

fn pack(compiled: &CompiledProject, route: &str, max_tokens: Option<usize>) -> Pack {
    context(
        &ProjectView::of(compiled),
        &request(route, CONTEXT_PLAN, max_tokens),
    )
    .unwrap_or_else(|refusal| panic!("context refused: {:?}", refusal.error))
}

fn document(pack: &Pack) -> Value {
    serde_json::from_str(&pack.text).expect("a pack is JSON")
}

fn strings(value: &Value) -> Vec<String> {
    value
        .as_array()
        .expect("an array")
        .iter()
        .map(|v| v.as_str().expect("a string").to_string())
        .collect()
}

fn section<'a>(document: &'a Value, name: &str) -> Option<&'a Value> {
    document["sections"]
        .as_array()
        .expect("sections")
        .iter()
        .find(|s| s["name"] == name)
}

fn nodes(document: &Value) -> Vec<String> {
    let mut ids: Vec<String> = document["graph"]["nodes"]
        .as_array()
        .expect("nodes")
        .iter()
        .map(|n| n["id"].as_str().expect("an id").to_string())
        .collect();
    ids.sort();
    ids
}

#[test]
fn a_route_serves_its_sections_for_a_subject_of_its_kind() {
    let project = gsd_context_project();
    let compiled = compile(&project);
    let served = pack(&compiled, "gsd/executor", None);
    assert_eq!(served.route, "gsd/executor");
    assert_eq!(served.subject, CONTEXT_PLAN);
    let document = document(&served);
    assert_eq!(document["route"], "gsd/executor");
    assert_eq!(document["agent"], "gsd-executor");
    assert_eq!(document["digest"], served.digest.as_str());
    assert_eq!(document["budget"]["budget_tokens"], 6000);
    assert_eq!(
        document["budget"]["truncated_sections"],
        serde_json::json!([])
    );
    let names: Vec<&str> = document["sections"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["name"].as_str().unwrap())
        .collect();
    assert_eq!(
        names,
        [
            "plan",
            "body",
            "phase",
            "requirements",
            "context",
            "decisions",
            "instructions",
            "research_items",
            "siblings",
            "settings"
        ]
    );
    let decisions = section(&document, "decisions").unwrap();
    assert_eq!(
        strings(&decisions["members"]),
        ["v1.0/02/D-01", "v1.0/02/D-02", "v1.0/02/D-03"]
    );
    let instructions = section(&document, "instructions").unwrap();
    assert_eq!(instructions["kind"], "files");
    assert_eq!(instructions["files"][1]["path"], "CLAUDE.md");
    assert_eq!(
        section(&document, "settings").unwrap()["settings"]["plan_check"],
        true
    );
    // The graph holds the subject and every entity a section served, nothing else.
    let mut served_ids: Vec<String> = document["sections"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|s| s["kind"] == "entities")
        .flat_map(|s| strings(&s["members"]))
        .chain([CONTEXT_PLAN.to_string()])
        .collect();
    served_ids.sort();
    served_ids.dedup();
    assert_eq!(nodes(&document), served_ids);
}

#[specforge_test(
    behavior = "serve_context_pack",
    verify = "a budgeted context read of a plan for its executor keeps every core member and cuts only enrich sections"
)]
fn a_budgeted_pack_keeps_its_core_and_cuts_in_declared_order() {
    let project = gsd_context_project();
    let compiled = compile(&project);
    let document = document(&pack(&compiled, "gsd/executor", Some(1500)));
    let body = strings(&section(&document, "body").expect("the core is served")["members"]);
    for member in ["T1", "T2", "TR1", "TR2", "TR3", "AR1", "KL1"] {
        let id = format!("{CONTEXT_PLAN}/{member}");
        assert!(body.contains(&id), "{id} in {body:?}");
        assert!(nodes(&document).contains(&id), "{id} is a node");
    }
    for core in ["plan", "phase", "requirements", "settings"] {
        assert!(section(&document, core).is_some(), "{core}");
    }
    let budget = &document["budget"];
    assert_eq!(budget["budget_tokens"], 1500);
    assert!(budget["estimated_tokens"].as_u64().unwrap() <= 1500);
    let truncated = strings(&budget["truncated_sections"]);
    assert!(!truncated.is_empty(), "{budget}");
    // Every truncated section is enrich and listed in declared order, after every section served whole.
    let order = [
        "context",
        "decisions",
        "instructions",
        "research_items",
        "siblings",
    ];
    let first = order
        .iter()
        .position(|name| *name == truncated[0])
        .expect("an enrich section");
    assert_eq!(truncated, order[first..]);
    for name in &order[..first] {
        assert!(section(&document, name).is_some(), "{name} served whole");
    }
}

#[specforge_test(
    behavior = "serve_context_pack",
    verify = "a route no loaded extension declares, or a subject of another kind, is invalid input"
)]
fn a_route_no_extension_declares_or_of_another_kind_is_refused() {
    let project = gsd_context_project();
    let compiled = compile(&project);
    let view = ProjectView::of(&compiled);
    let refused = context(&view, &request("gsd/nobody", CONTEXT_PLAN, None)).unwrap_err();
    assert_eq!(refused.error.code, UNKNOWN_ROUTE);
    assert_eq!(refused.subject, Some(ContextField::Route));
    assert!(
        refused.error.message.contains("gsd/executor"),
        "{}",
        refused.error.message
    );

    let refused = context(&view, &request("gsd/executor", "v1.0/02", None)).unwrap_err();
    assert_eq!(refused.error.code, ROUTE_OF_ANOTHER_KIND);
    assert_eq!(refused.subject, Some(ContextField::Subject));

    let refused = context(&view, &request("gsd/executor", "v1.0/09/nothing", None)).unwrap_err();
    assert_eq!(refused.error.code, "E003");
    assert_eq!(refused.subject, Some(ContextField::Subject));

    let refused = context(&view, &request("gsd/executor", CONTEXT_PLAN, Some(10))).unwrap_err();
    assert_eq!(refused.error.code, "E062");
    assert_eq!(refused.subject, Some(ContextField::MaxTokens));
    assert!(
        refused.error.message.contains("`gsd/executor`"),
        "{}",
        refused.error.message
    );
}

#[specforge_test(
    behavior = "serve_context_pack",
    verify = "the pack digest covers the sections served and recomputes from the pack"
)]
fn the_pack_digest_recomputes_and_covers_only_what_is_served() {
    let project = gsd_context_project();
    let compiled = compile(&project);
    let small = pack(&compiled, "gsd/executor", Some(1500));
    assert_eq!(small, pack(&compiled, "gsd/executor", Some(1500)));
    let truncated = strings(&document(&small)["budget"]["truncated_sections"]);
    assert!(
        truncated.contains(&"research_items".to_string()),
        "{truncated:?}"
    );

    // A research finding changes: the small pack, which cut the research, keeps its digest; the whole pack, which
    // served it, moves.
    let whole = pack(&compiled, "gsd/executor", None);
    assert!(section(&document(&whole), "research_items").is_some());
    let edited = GSD_CONTEXT_SPEC.replace(
        "One JSON file, written then renamed",
        "One JSON file per board",
    );
    let after = gsd_routed_project_over(&edited, &[GSD_JUDGED, CODE_CORE])
        .file("CLAUDE.md", CONTEXT_INSTRUCTIONS);
    let recompiled = compile(&after);
    assert_eq!(
        pack(&recompiled, "gsd/executor", Some(1500)).digest,
        small.digest
    );
    assert_ne!(pack(&recompiled, "gsd/executor", None).digest, whole.digest);
}

#[specforge_test(
    behavior = "render_context_section",
    verify = "an installed agent a context route names gets a Context section naming the route, its sections and how to read it"
)]
fn an_agent_a_route_names_gets_a_context_section_and_others_none() {
    use specforge_ops::host::{Host, render};
    let body = json!({"body": "# The agent\n"}).to_string().into_bytes();
    let runtime =
        gsd_context_runtime().answer_raw(GSD_JUDGED, "render__", WasmCallResult::Ok(body));
    let project = gsd_context_project();
    let compiled = project.compile_in(Arc::new(runtime) as SharedRuntime);
    let rendered = render::rendered(&ProjectView::of(&compiled), Host::ClaudeCode)
        .unwrap_or_else(|error| panic!("render refused: {error:?}"));
    let body = |id: &str| {
        rendered
            .agents
            .iter()
            .find(|agent| agent.id == id)
            .unwrap_or_else(|| panic!("the agent {id}"))
            .body
            .clone()
    };
    let executor = body("gsd-executor");
    assert!(executor.contains("\n## Context\n"), "{executor}");
    assert!(
        executor.contains("`specforge context gsd/executor <id>`"),
        "{executor}"
    );
    assert!(
        executor.contains("`specforge://context/<id>?role=gsd/executor`"),
        "{executor}"
    );
    assert!(
        executor.contains(
            "Sections, in order: plan (core), body (core), phase (core), requirements (core), context, decisions, \
             instructions, research_items, siblings (as the budget of 6000 tokens allows)."
        ),
        "{executor}"
    );
    assert!(executor.contains("the `plan` you were given"), "{executor}");
    let checker = body("gsd-plan-checker");
    let context = checker.find("## Context").expect("a Context section");
    let judged = checker
        .find("## Judged rules")
        .expect("a Judged rules section");
    assert!(context < judged, "{checker}");
    assert!(checker.contains("gsd/plan-checker"), "{checker}");
    assert!(!body("gsd-verifier").contains("## Context"));
}
