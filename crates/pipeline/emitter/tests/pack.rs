//! A context pack fitted to its budget in declared order (ADR 0356): the subject and core sections are the floor, enrich
//! sections are served whole in order, and inside the first that does not fit its most connected entities are kept.

use serde_json::Value;
use specforge_common::Sym;
use specforge_diagnostics::codes;
use specforge_emitter::pack::{
    Cut, PackPlan, PackSection, Sealed, SectionBody, Served, fit, render,
};
use specforge_emitter::{EmitFormat, GraphProtocolSchema};
use specforge_graph::Graph;
use specforge_protocol_types::ContextTier;
use specforge_protocol_types::tokens::estimate_tokens;
use specforge_registry::RegistryBuild;

const SRC: &str = r#"
invariant hub "Hub invariant" { guarantee "g" }
behavior s "The subject" { contract "c" invariants [hub] }
behavior near "Near" { contract "c" invariants [hub] }
invariant i1 "Lone invariant with a long title to cost tokens" { guarantee "g" }
invariant i2 "Central invariant with a long title to cost tokens" { guarantee "g" }
invariant i3 "Edge invariant with a long title to cost tokens" { guarantee "g" }
behavior x1 "X1" { contract "c" invariants [i2, i3] }
behavior x2 "X2" { contract "c" invariants [i2] }
behavior late "Late" { contract "c" }
"#;

fn graph() -> Graph {
    specforge_graph::build_graph(&[specforge_parser::parse(SRC, "main.spec")]).0
}

fn digest(n: u8) -> String {
    format!("sha256:{}", format!("{n:02x}").repeat(32))
}

fn entities(name: &str, tier: ContextTier, ids: &[&str], n: u8) -> PackSection {
    PackSection {
        name: name.to_string(),
        tier,
        selector: format!("self<{name}"),
        body: SectionBody::Entities(ids.iter().map(|id| Sym::new(id)).collect()),
        digest: digest(n),
    }
}

/// A route over `s`: core `hub`, then enrich `near`, `rest` (i1, i2, i3) and `late`.
fn plan<'a>(
    budget: usize,
    schema: &'a GraphProtocolSchema,
    registries: &'a RegistryBuild,
) -> PackPlan<'a> {
    PackPlan {
        route: "acme/reader".to_string(),
        subject: Sym::new("s"),
        agent: None,
        format: EmitFormat::Brief,
        budget,
        sections: vec![
            entities("core", ContextTier::Core, &["hub"], 1),
            entities("near", ContextTier::Enrich, &["near"], 2),
            entities("rest", ContextTier::Enrich, &["i1", "i2", "i3"], 3),
            entities("late", ContextTier::Enrich, &["late"], 4),
        ],
        skipped: Vec::new(),
        schema,
        registries,
        computed: None,
    }
}

/// The cut's digests as a caller seals them: each served section's, a part's re-digested.
fn sealed(plan: &PackPlan<'_>, cut: &Cut) -> Sealed {
    Sealed {
        sections: plan
            .sections
            .iter()
            .zip(&cut.served)
            .map(|(section, served)| match served {
                Served::Cut => String::new(),
                Served::Part(_) => digest(9),
                Served::Whole => section.digest.clone(),
            })
            .collect(),
        pack: digest(10),
    }
}

/// The smallest budget `fit` accepts, searched upward from 1 (a counted search, at most 5,000 steps).
fn floor(graph: &Graph) -> usize {
    (1..5_000)
        .find(|&budget| fit(graph, &plan_at(budget)).is_ok())
        .expect("a budget holds the core")
}

fn plan_at(budget: usize) -> PackPlan<'static> {
    static SCHEMA: std::sync::LazyLock<GraphProtocolSchema> =
        std::sync::LazyLock::new(GraphProtocolSchema::empty);
    static REGISTRIES: std::sync::LazyLock<RegistryBuild> =
        std::sync::LazyLock::new(RegistryBuild::default);
    plan(budget, &SCHEMA, &REGISTRIES)
}

fn node_ids(pack: &str) -> Vec<String> {
    let document: Value = serde_json::from_str(pack).unwrap();
    document["graph"]["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|node| node["id"].as_str().unwrap().to_string())
        .collect()
}

#[specforge_test_macros::test(
    behavior = "serve_context_pack",
    verify = "core sections are never cut, and a budget below them is E062 naming the route"
)]
fn core_sections_are_never_cut_and_a_budget_below_them_is_e062_naming_the_route() {
    let graph = graph();
    let at = floor(&graph);
    let error = fit(&graph, &plan_at(at - 1)).unwrap_err();
    assert_eq!(error.code(), Some(codes::E062));
    assert!(error.to_string().contains("`acme/reader`"), "{error}");
    assert!(error.to_string().contains("`s`"), "{error}");

    let cut = fit(&graph, &plan_at(at)).unwrap();
    assert_eq!(cut.served[0], Served::Whole);
    assert_eq!(cut.truncated_sections, ["near", "rest", "late"]);
    assert!(cut.estimated_tokens <= at);
}

#[specforge_test_macros::test(
    behavior = "serve_context_pack",
    verify = "enrich sections fill the budget whole in declared order, and inside the first that does not fit its most central entities are kept"
)]
fn enrich_sections_fill_the_budget_whole_in_declared_order() {
    let graph = graph();
    let cut = fit(&graph, &plan_at(100_000)).unwrap();
    assert!(cut.served.iter().all(|s| *s == Served::Whole), "{cut:?}");
    assert!(cut.truncated_sections.is_empty());
    assert!(cut.truncated_entities.is_empty());

    // Raising the budget one token at a time, `near` is served whole before anything of `rest`, and `late` only once
    // `rest` is whole.
    let at = floor(&graph);
    let mut near_whole = None;
    for budget in at..at + 5_000 {
        let cut = fit(&graph, &plan_at(budget)).unwrap();
        if cut.served[1] == Served::Whole && near_whole.is_none() {
            near_whole = Some(budget);
        }
        if cut.served[2] != Served::Cut {
            assert!(near_whole.is_some_and(|whole| whole <= budget), "{cut:?}");
        }
        if cut.served[3] != Served::Cut {
            assert_eq!(cut.served[2], Served::Whole, "{cut:?}");
            return;
        }
    }
    panic!("no budget served `late`");
}

#[specforge_test_macros::test(
    behavior = "serve_context_pack",
    verify = "enrich sections fill the budget whole in declared order, and inside the first that does not fit its most central entities are kept"
)]
fn the_first_enrich_section_that_does_not_fit_keeps_its_most_central_entities_and_later_ones_are_dropped()
 {
    let graph = graph();
    let at = floor(&graph);
    let part = (at..at + 5_000)
        .map(|budget| fit(&graph, &plan_at(budget)).unwrap())
        .find(|cut| matches!(&cut.served[2], Served::Part(kept) if kept.len() == 1))
        .expect("a budget keeps one entity of `rest`");
    assert_eq!(part.served[2], Served::Part(vec![Sym::new("i2")]));
    assert_eq!(part.served[1], Served::Whole);
    assert_eq!(part.served[3], Served::Cut);
    assert_eq!(part.truncated_sections, ["rest", "late"]);
    assert_eq!(part.truncated_entities, ["i1", "i3"]);
}

#[test]
fn the_subject_is_never_cut() {
    let graph = graph();
    let at = floor(&graph);
    let plan = plan_at(at);
    let cut = fit(&graph, &plan).unwrap();
    let pack = render(&graph, &plan, &cut, &sealed(&plan, &cut)).unwrap();
    assert_eq!(node_ids(&pack), ["hub", "s"]);
}

#[test]
fn a_pack_renders_its_budget_sections_and_served_graph() {
    let graph = graph();
    let plan = plan_at(100_000);
    let cut = fit(&graph, &plan).unwrap();
    let pack = render(&graph, &plan, &cut, &sealed(&plan, &cut)).unwrap();
    assert_eq!(estimate_tokens(&pack), cut.estimated_tokens);
    assert_eq!(
        pack,
        render(&graph, &plan, &cut, &sealed(&plan, &cut)).unwrap()
    );
    let document: Value = serde_json::from_str(&pack).unwrap();
    assert_eq!(document["route"], "acme/reader");
    assert_eq!(document["subject"], "s");
    assert_eq!(document["digest"], digest(10));
    let budget = &document["budget"];
    assert_eq!(budget["strategy"], "declared");
    assert_eq!(budget["budget_tokens"], 100_000);
    assert_eq!(budget["truncated_sections"], serde_json::json!([]));
    assert_eq!(budget["truncated_entities"], serde_json::json!([]));
    let names: Vec<&str> = document["sections"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, ["core", "near", "rest", "late"]);
    assert_eq!(
        document["sections"][2]["members"],
        serde_json::json!(["i1", "i2", "i3"])
    );
    assert_eq!(document["sections"][2]["tier"], "enrich");
    assert_eq!(document["sections"][2]["digest"], digest(3));
    let mut nodes = node_ids(&pack);
    nodes.sort();
    assert_eq!(nodes, ["hub", "i1", "i2", "i3", "late", "near", "s"]);
}

/// The plan at `budget` over core `hub` and the enrich sections `enrich`, each `(name, members)`.
fn plan_over(budget: usize, enrich: &[(&str, &[&str])]) -> PackPlan<'static> {
    let mut plan = plan_at(budget);
    plan.sections.truncate(1);
    for (n, (name, ids)) in enrich.iter().enumerate() {
        plan.sections
            .push(entities(name, ContextTier::Enrich, ids, n as u8 + 2));
    }
    plan
}

/// Every cut from the floor of `enrich` up to the budget that serves it all.
fn cuts(graph: &Graph, enrich: &[(&str, &[&str])]) -> Vec<Cut> {
    let at = (1..5_000)
        .find(|&budget| fit(graph, &plan_over(budget, enrich)).is_ok())
        .expect("a budget holds the core");
    let mut cuts = Vec::new();
    for budget in at..at + 5_000 {
        let cut = fit(graph, &plan_over(budget, enrich)).unwrap();
        let whole = cut.served.iter().all(|s| *s == Served::Whole);
        cuts.push(cut);
        if whole {
            return cuts;
        }
    }
    panic!("no budget serves {enrich:?}");
}

#[test]
fn a_subject_an_enrich_section_also_selects_is_never_new_nor_truncated() {
    let graph = graph();
    let cuts = cuts(&graph, &[("rest", &["s", "i1", "i2", "i3"])]);
    for cut in &cuts {
        assert!(
            !cut.truncated_entities.contains(&"s".to_string()),
            "{cut:?}"
        );
        if let Served::Part(kept) = &cut.served[1] {
            assert!(kept.contains(&Sym::new("s")), "{cut:?}");
        }
    }
    // Kept entities only grow with the budget.
    let kept = |cut: &Cut| match &cut.served[1] {
        Served::Cut => 0,
        Served::Part(kept) => kept.len(),
        Served::Whole => 4,
    };
    assert!(cuts.windows(2).all(|w| kept(&w[0]) <= kept(&w[1])));
}

#[test]
fn an_empty_enrich_section_is_whole_or_cut_never_an_empty_part() {
    let graph = graph();
    for cut in cuts(&graph, &[("none", &[])]) {
        assert_ne!(cut.served[1], Served::Part(Vec::new()), "{cut:?}");
        assert!(cut.truncated_entities.is_empty(), "{cut:?}");
    }
}

#[test]
fn an_enrich_section_bigger_than_the_budget_is_cut_whole_not_served_empty() {
    let graph = graph();
    let cuts = cuts(&graph, &[("rest", &["i1", "i2", "i3"])]);
    assert_eq!(cuts[0].served[1], Served::Cut);
    assert_eq!(cuts[0].truncated_sections, ["rest"]);
    assert!(cuts[0].truncated_entities.is_empty());
    for cut in &cuts {
        assert_ne!(cut.served[1], Served::Part(Vec::new()), "{cut:?}");
    }
}
