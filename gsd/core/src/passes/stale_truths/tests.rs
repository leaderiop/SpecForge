use serde_json::{Value, json};

use super::*;

const PHASE: &str = "v1.0/01";
const PLAN: &str = "v1.0/01/01-01";
const TR1: &str = "v1.0/01/01-01/TR1";
/// The commit the base names.
const R: &str = "0123456789abcdef0123456789abcdef01234567";

/// A `gsd/E501` judgement in `state` with the inputs `changed` and the `(path, digest)` files it read.
fn verdict(state: &str, changed: &[&str], files: &[(&str, &str)]) -> Value {
    verdict_of("gsd/E501", state, changed, files)
}

fn verdict_of(rule: &str, state: &str, changed: &[&str], files: &[(&str, &str)]) -> Value {
    let files: Vec<Value> = files
        .iter()
        .map(|(path, digest)| json!({ "path": path, "digest": digest }))
        .collect();
    json!({
        "rule": rule, "state": state, "passed": state == "fresh", "outcome": "verified",
        "at": "2026-11-20T10:00:00Z", "changed": changed, "files": files,
    })
}

/// The stale verdict on TR1 after `src/tasks.js` moved from `A` to `C`; `test/tasks.test.js` is still `B`.
fn stale_after_an_edit() -> Value {
    verdict(
        "stale",
        &["cited"],
        &[("src/tasks.js", "C"), ("test/tasks.test.js", "B")],
    )
}

/// TR1's verdict at the base: fresh over `src/tasks.js` at `A` and `test/tasks.test.js` at `B`.
fn fresh_at_the_base() -> Value {
    verdict(
        "fresh",
        &[],
        &[("src/tasks.js", "A"), ("test/tasks.test.js", "B")],
    )
}

/// A phase, its plan and the plan's truth TR1 holding `judgements`, with `extra` keys (`delta`, `previous`) on the
/// input.
fn input(judgements: Vec<Value>, extra: Value) -> PassInput {
    let mut input = json!({
        "entities": [
            { "id": PHASE, "kind": "phase" },
            { "id": PLAN, "kind": "plan" },
            { "id": TR1, "kind": "truth", "judgements": judgements },
        ],
        "edges": [
            { "source": PLAN, "target": PHASE, "label": "phase" },
            { "source": TR1, "target": PLAN, "label": "plan" },
        ],
    });
    for (key, value) in extra.as_object().expect("an object") {
        input[key] = value.clone();
    }
    serde_json::from_value(input).expect("a pass input")
}

fn revision() -> Value {
    json!({ "kind": "revision", "id": R })
}

/// A delta from `base` with `nodes`.
fn delta(base: Value, nodes: Vec<Value>) -> Value {
    json!({ "base": base, "nodes": nodes })
}

/// The baseline read at `revision`, TR1 holding `judgement` there.
fn baseline(revision: &str, judgement: Value) -> Value {
    json!({
        "revision": revision,
        "entities": { TR1: { "kind": "truth", "judgements": [judgement] } },
    })
}

/// A node `modified` with these fields and aspects and two different content digests.
fn modified(kind: &str, id: &str, fields: &[&str], aspects: &[&str]) -> Value {
    json!({
        "kind": kind, "id": id, "change": "modified", "fields": fields, "aspects": aspects,
        "before": "d0", "after": "d1",
    })
}

/// A check since `R` with these delta nodes, TR1 stale after the edit and fresh at the base.
fn since_r(nodes: Vec<Value>, now: Value) -> PassInput {
    input(
        vec![now],
        json!({ "delta": delta(revision(), nodes), "previous": baseline(R, fresh_at_the_base()) }),
    )
}

/// The I527 on TR1, its code and severity checked.
fn on_tr1(input: &PassInput) -> PassDiagnostic {
    let found = check(input);
    let advisory = found
        .iter()
        .find(|d| d.entity.as_deref() == Some(TR1))
        .unwrap_or_else(|| panic!("an advisory on {TR1}: {found:?}"))
        .clone();
    assert_eq!(advisory.code, "I527");
    assert_eq!(advisory.severity, PassSeverity::Info);
    assert_eq!(found.len(), 1, "{found:?}");
    advisory
}

#[test]
fn no_stale_truth_says_nothing() {
    let input = input(
        vec![fresh_at_the_base()],
        json!({ "delta": delta(revision(), vec![]) }),
    );
    assert!(check(&input).is_empty());
}

#[test]
fn a_cold_check_says_nothing_even_over_a_stale_truth() {
    let added = json!({ "kind": "truth", "id": TR1, "change": "added", "after": "d1" });
    let input = input(
        vec![stale_after_an_edit()],
        json!({ "delta": delta(json!({ "kind": "none" }), vec![added]) }),
    );
    assert!(check(&input).is_empty());
}

#[test]
fn a_pass_input_without_a_delta_says_nothing() {
    let input = input(vec![stale_after_an_edit()], json!({}));
    assert!(check(&input).is_empty());
}

#[test]
fn only_a_stale_verifier_verdict_on_a_truth_is_named() {
    let mut input = input(
        vec![verdict("fresh", &[], &[("src/tasks.js", "C")])],
        json!({ "delta": delta(revision(), vec![]) }),
    );
    let stale_plan_check = serde_json::from_value(verdict_of("gsd/W311", "stale", &["plan"], &[]))
        .expect("a judgement");
    let stale_artifact =
        serde_json::from_value(verdict("stale", &["file"], &[("src/tasks.js", "C")]))
            .expect("a judgement");
    input
        .entities
        .iter_mut()
        .find(|e| e.id == PLAN)
        .expect("the plan")
        .judgements
        .push(stale_plan_check);
    input.entities.push(PassEntity {
        id: "v1.0/01/01-01/AR1".into(),
        kind: "artifact".into(),
        judgements: vec![stale_artifact],
        ..Default::default()
    });
    assert!(check(&input).is_empty());
}

#[test]
fn a_cited_file_changed_since_the_revision_is_named_alone() {
    let advisory = on_tr1(&since_r(vec![], stale_after_an_edit()));
    let message = &advisory.message;
    assert!(
        message.contains(&format!("since {}", &R[..12])),
        "{message}"
    );
    assert!(
        message.contains("only the files it cites changed: 'src/tasks.js'"),
        "{message}"
    );
    assert!(!message.contains("test/tasks.test.js"), "{message}");
    assert!(
        message.contains("(cited changed since it was recorded)"),
        "{message}"
    );
}

#[test]
fn nothing_changed_since_the_revision_is_said() {
    let now = verdict(
        "stale",
        &["tests"],
        &[("src/tasks.js", "A"), ("test/tasks.test.js", "B")],
    );
    let advisory = on_tr1(&since_r(vec![], now));
    assert!(
        advisory
            .message
            .contains("neither the spec it reads nor a file it cites changed"),
        "{}",
        advisory.message
    );
}

#[test]
fn a_revision_the_baseline_was_not_read_at_claims_no_file() {
    let other = "fedcba9876543210fedcba9876543210fedcba98";
    let input = input(
        vec![stale_after_an_edit()],
        json!({ "delta": delta(revision(), vec![]), "previous": baseline(other, fresh_at_the_base()) }),
    );
    let message = on_tr1(&input).message;
    assert!(
        message.contains(
            "the spec it reads is unchanged; it cites 'src/tasks.js', 'test/tasks.test.js'"
        ),
        "{message}"
    );
    assert!(!message.contains("only the files"), "{message}");
}

#[test]
fn the_last_check_names_the_cited_files_without_claiming_them() {
    let input = input(
        vec![stale_after_an_edit()],
        json!({
            "delta": delta(json!({ "kind": "previous" }), vec![]),
            "previous": baseline(R, fresh_at_the_base()),
        }),
    );
    let message = on_tr1(&input).message;
    assert!(message.contains("since the last check"), "{message}");
    assert!(message.contains("it cites"), "{message}");
    assert!(!message.contains("only the files"), "{message}");
}

#[test]
fn a_truth_changed_since_the_base_is_the_spec() {
    let now = verdict(
        "stale",
        &["truth"],
        &[("src/tasks.js", "A"), ("test/tasks.test.js", "B")],
    );
    let input = since_r(vec![modified("truth", TR1, &["text"], &[])], now);
    let message = on_tr1(&input).message;
    assert!(
        message.contains(&format!(
            "the spec it reads changed: truth '{TR1}' (text); no file it cites changed"
        )),
        "{message}"
    );
}

#[test]
fn the_suggestion_names_re_verify_on_the_truths_plan() {
    let advisory = on_tr1(&since_r(vec![], stale_after_an_edit()));
    assert_eq!(
        advisory.suggestion.as_deref(),
        Some(
            "`specforge gsd re-verify v1.0/01/01-01 --stale-only` lists the verdicts for the verifier to record \
             again"
        )
    );
}

/// The kind `selector` reaches from a truth, through each reference field it names (`self.plan.phase`: truth's
/// `plan`, then that plan's `phase`), as the kinds declare them; `None` for one that reads no entity (a fact query).
fn kind_from_a_truth(selector: &str) -> Option<&'static str> {
    if selector.contains('(') {
        return None;
    }
    let mut fields = selector.split('.');
    if fields.next() != Some("self") {
        return None;
    }
    fields.try_fold("truth", |kind, name| {
        let kind = crate::kinds::KINDS.iter().find(|k| k.name == kind)?;
        match kind.fields.iter().find(|f| f.name == name)?.ty {
            gsd_kit::dsl::Ty::Ref(target) => Some(target),
            _ => None,
        }
    })
}

#[test]
fn the_delta_kinds_are_the_kinds_a_truths_verdict_reads() {
    let (entities, facts): (Vec<_>, Vec<_>) = crate::judged::TRUTH_INPUTS
        .iter()
        .partition(|(_, selector)| kind_from_a_truth(selector).is_some());
    let kinds: Vec<&str> = entities
        .iter()
        .filter_map(|(_, selector)| kind_from_a_truth(selector))
        .collect();
    assert_eq!(kinds, READS);
    // `read_by` follows the same edges: the truth's `plan`, then the plan's `phase`.
    let selectors: Vec<&str> = entities.iter().map(|(_, selector)| *selector).collect();
    assert_eq!(selectors, ["self", "self.plan", "self.plan.phase"]);
    let left_out: Vec<&str> = facts.iter().map(|(name, _)| *name).collect();
    assert_eq!(left_out, ["tests"]);
}

#[test]
fn a_selector_through_another_reference_reaches_another_kind() {
    assert_eq!(kind_from_a_truth("self.plan.phase"), Some("phase"));
    assert_ne!(kind_from_a_truth("self.plan.milestone"), Some("phase"));
    assert_eq!(kind_from_a_truth("self.text"), None);
    assert_eq!(kind_from_a_truth("test(self<test.names)"), None);
}

#[test]
fn a_verdict_recorded_since_the_base_is_not_a_spec_change() {
    let recorded = json!({
        "kind": "truth", "id": TR1, "change": "modified", "aspects": ["verdicts"],
        "before": "d0", "after": "d0",
    });
    let message = on_tr1(&since_r(vec![recorded], stale_after_an_edit())).message;
    assert!(
        message.contains("only the files it cites changed: 'src/tasks.js'"),
        "{message}"
    );
    assert!(!message.contains("the spec it reads changed"), "{message}");
}

#[test]
fn its_plan_or_its_phase_changed_is_the_spec() {
    let plan = modified("plan", PLAN, &["objective"], &[]);
    let phase = modified("phase", PHASE, &[], &["title"]);
    let now = || {
        verdict(
            "stale",
            &["plan"],
            &[("src/tasks.js", "A"), ("test/tasks.test.js", "B")],
        )
    };
    let message = on_tr1(&since_r(vec![plan.clone()], now())).message;
    assert!(
        message.contains(&format!(
            "the spec it reads changed: plan '{PLAN}' (objective)"
        )),
        "{message}"
    );
    let message = on_tr1(&since_r(vec![phase.clone()], now())).message;
    assert!(
        message.contains(&format!(
            "the spec it reads changed: phase '{PHASE}' (title)"
        )),
        "{message}"
    );
    // The delta lists the phase first; the message names what the verdict reads in its order.
    let message = on_tr1(&since_r(vec![phase, plan], now())).message;
    assert!(
        message.contains(&format!(
            "the spec it reads changed: plan '{PLAN}' (objective), phase '{PHASE}' (title)"
        )),
        "{message}"
    );
}

#[test]
fn a_truth_added_since_the_base_is_named_added() {
    let added = json!({ "kind": "truth", "id": TR1, "change": "added", "after": "d1" });
    let input = input(
        vec![stale_after_an_edit()],
        json!({
            "delta": delta(revision(), vec![added]),
            "previous": { "revision": R, "entities": {} },
        }),
    );
    let message = on_tr1(&input).message;
    assert!(
        message.contains(&format!(
            "the spec it reads changed: truth '{TR1}' (added); it cites 'src/tasks.js', 'test/tasks.test.js'"
        )),
        "{message}"
    );
}

#[test]
fn another_truths_change_is_not_this_ones() {
    let nodes = vec![
        modified("truth", "v1.0/01/01-02/TR1", &["text"], &[]),
        modified("plan", "v1.0/01/01-02", &["objective"], &[]),
    ];
    let message = on_tr1(&since_r(nodes, stale_after_an_edit())).message;
    assert!(!message.contains("the spec it reads changed"), "{message}");
    assert!(
        message.contains("only the files it cites changed: 'src/tasks.js'"),
        "{message}"
    );
}

#[test]
fn the_spec_and_a_cited_file_both_changed() {
    let now = verdict(
        "stale",
        &["truth", "cited"],
        &[("src/tasks.js", "C"), ("test/tasks.test.js", "B")],
    );
    let message = on_tr1(&since_r(vec![modified("truth", TR1, &["text"], &[])], now)).message;
    assert!(
        message.contains(&format!(
            "the spec it reads changed: truth '{TR1}' (text); the files it cites changed too: 'src/tasks.js'"
        )),
        "{message}"
    );
}

#[test]
fn a_path_with_a_space_a_non_ascii_letter_or_a_line_break_stays_on_one_line() {
    let paths = ["src/my file.js", "src/café.js", "src/a\nb.js"];
    let at = |digest: &'static str| -> Vec<(&str, &str)> {
        paths.iter().map(|path| (*path, digest)).collect()
    };
    let input = input(
        vec![verdict("stale", &["cited"], &at("C"))],
        json!({ "delta": delta(revision(), vec![]), "previous": baseline(R, verdict("fresh", &[], &at("A"))) }),
    );
    let message = on_tr1(&input).message;
    assert!(
        message.contains(
            "only the files it cites changed: 'src/my file.js', 'src/café.js', 'src/a\\nb.js'"
        ),
        "{message}"
    );
    assert!(!message.contains('\n'), "{message}");
}

/// A phase with plans `01-2` and `01-10`, each with a truth whose verdict went stale when its text changed since `R`:
/// the entities, the edges and the delta nodes in the order given, or each reversed.
fn two_plans(reversed: bool) -> PassInput {
    let mut entities = vec![json!({ "id": PHASE, "kind": "phase" })];
    let mut edges = Vec::new();
    let mut nodes = Vec::new();
    let mut at_the_base = serde_json::Map::new();
    for plan in ["v1.0/01/01-2", "v1.0/01/01-10"] {
        let truth = format!("{plan}/TR1");
        let now = verdict("stale", &["truth"], &[("src/tasks.js", "A")]);
        entities.push(json!({ "id": plan, "kind": "plan" }));
        entities.push(json!({ "id": truth, "kind": "truth", "judgements": [now] }));
        edges.push(json!({ "source": plan, "target": PHASE, "label": "phase" }));
        edges.push(json!({ "source": truth, "target": plan, "label": "plan" }));
        nodes.push(modified("truth", &truth, &["text"], &[]));
        let then = verdict("fresh", &[], &[("src/tasks.js", "A")]);
        at_the_base.insert(truth, json!({ "kind": "truth", "judgements": [then] }));
    }
    if reversed {
        entities.reverse();
        edges.reverse();
        nodes.reverse();
    }
    serde_json::from_value(json!({
        "entities": entities,
        "edges": edges,
        "delta": delta(revision(), nodes),
        "previous": { "revision": R, "entities": at_the_base },
    }))
    .expect("a pass input")
}

#[test]
fn the_order_of_entities_and_nodes_does_not_change_the_answer() {
    let given = check(&two_plans(false));
    let reversed = check(&two_plans(true));
    assert_eq!(
        serde_json::to_string(&given).expect("JSON"),
        serde_json::to_string(&reversed).expect("JSON")
    );
    let truths: Vec<Option<&str>> = given.iter().map(|d| d.entity.as_deref()).collect();
    assert_eq!(
        truths,
        [Some("v1.0/01/01-2/TR1"), Some("v1.0/01/01-10/TR1")]
    );
}

#[test]
fn a_session_check_names_the_spec_since_the_last_check() {
    let now = verdict(
        "stale",
        &["truth"],
        &[("src/tasks.js", "A"), ("test/tasks.test.js", "B")],
    );
    let input = input(
        vec![now],
        json!({
            "delta": delta(json!({ "kind": "previous" }), vec![modified("truth", TR1, &["text"], &[])]),
            "previous": baseline(R, fresh_at_the_base()),
        }),
    );
    let message = on_tr1(&input).message;
    assert!(
        message.contains(&format!(
            "since the last check, the spec it reads changed: truth '{TR1}' (text); it cites 'src/tasks.js', \
             'test/tasks.test.js'"
        )),
        "{message}"
    );
    assert!(!message.contains("changed too"), "{message}");
    assert!(!message.contains("no file it cites changed"), "{message}");
}

#[test]
fn a_verdict_that_cites_no_file_names_none() {
    let input = input(
        vec![verdict("stale", &["tests"], &[])],
        json!({ "delta": delta(json!({ "kind": "previous" }), vec![]) }),
    );
    let message = on_tr1(&input).message;
    assert!(
        message.ends_with("since the last check, the spec it reads is unchanged"),
        "{message}"
    );
    assert!(!message.contains("it cites"), "{message}");
    assert!(!message.contains("''"), "{message}");
}

/// TR1's stale verdict cites `src/tasks.js` at `C` and `src/added.js`; at the base it cited `then`.
fn with_a_file_added_since(then: &[(&str, &str)]) -> PassInput {
    let now = verdict(
        "stale",
        &["cited"],
        &[("src/tasks.js", "C"), ("src/added.js", "N")],
    );
    input(
        vec![now],
        json!({ "delta": delta(revision(), vec![]), "previous": baseline(R, verdict("fresh", &[], then)) }),
    )
}

#[test]
fn a_file_the_verdict_at_the_base_did_not_read_is_named_apart() {
    let message = on_tr1(&with_a_file_added_since(&[("src/tasks.js", "A")])).message;
    assert!(
        message.ends_with(
            "only the files it cites changed: 'src/tasks.js'; it did not cite 'src/added.js' at the base"
        ),
        "{message}"
    );
}

/// The base's verdict cited no file, another file, or one unchanged file beside the one cited since: no file it read
/// then changed, and one it did not read may have, so the pass claims nothing about the files.
#[test]
fn no_change_among_the_files_cited_at_the_base_claims_nothing_of_the_others() {
    for then in [
        &[][..],
        &[("src/old.js", "A")][..],
        &[("src/tasks.js", "C")][..],
    ] {
        let message = on_tr1(&with_a_file_added_since(then)).message;
        assert!(
            message.ends_with(
                "the spec it reads is unchanged; it cites 'src/tasks.js', 'src/added.js'"
            ),
            "{then:?}: {message}"
        );
        assert!(!message.contains("no file it cites changed"), "{message}");
        assert!(!message.contains("neither"), "{message}");
        assert!(!message.contains("did not cite"), "{message}");
    }
}

#[test]
fn a_spec_change_over_a_base_that_cited_no_file_claims_nothing_of_the_files() {
    let input = input(
        vec![stale_after_an_edit()],
        json!({
            "delta": delta(revision(), vec![modified("truth", TR1, &["text"], &[])]),
            "previous": baseline(R, verdict("fresh", &[], &[])),
        }),
    );
    let message = on_tr1(&input).message;
    assert!(
        message.ends_with(&format!(
            "the spec it reads changed: truth '{TR1}' (text); it cites 'src/tasks.js', 'test/tasks.test.js'"
        )),
        "{message}"
    );
    assert!(!message.contains("no file it cites changed"), "{message}");
}

#[test]
fn an_aspect_is_named_as_the_wire_names_it() {
    for aspect in [
        DeltaAspect::Title,
        DeltaAspect::Methods,
        DeltaAspect::Obligations,
        DeltaAspect::Verdicts,
        DeltaAspect::Edges,
    ] {
        assert_eq!(
            serde_json::to_value(aspect).expect("an aspect serializes"),
            json!(aspect_name(&aspect)),
            "{aspect:?}"
        );
    }
}

/// Ids, field names and input names come from the grammar's identifiers, which hold no line break; the advisory stays
/// one line even if one did, as it does for a path.
#[test]
fn an_id_or_a_name_with_a_line_break_stays_on_one_line() {
    let (phase, plan, truth) = ("v1.0/0\n1", "v1.0/0\n1/01-01", "v1.0/0\n1/01-01/TR1");
    let input: PassInput = serde_json::from_value(json!({
        "entities": [
            { "id": phase, "kind": "phase" },
            { "id": plan, "kind": "plan" },
            { "id": truth, "kind": "truth", "judgements": [verdict("stale", &["ci\nted"], &[])] },
        ],
        "edges": [
            { "source": plan, "target": phase, "label": "phase" },
            { "source": truth, "target": plan, "label": "plan" },
        ],
        "delta": delta(revision(), vec![
            modified("truth", truth, &["te\nxt"], &[]),
            modified("phase", phase, &[], &["title"]),
        ]),
    }))
    .expect("a pass input");
    let found = check(&input);
    assert_eq!(found.len(), 1, "{found:?}");
    let message = &found[0].message;
    assert!(
        message.contains(
            "truth 'v1.0/0\\n1/01-01/TR1' is stale (ci\\nted changed since it was recorded)"
        ),
        "{message}"
    );
    assert!(
        message.contains(
            "the spec it reads changed: truth 'v1.0/0\\n1/01-01/TR1' (te\\nxt), phase 'v1.0/0\\n1' (title)"
        ),
        "{message}"
    );
    assert!(!message.contains('\n'), "{message}");
    let suggestion = found[0].suggestion.as_deref().expect("a suggestion");
    assert!(
        suggestion.contains("re-verify v1.0/0\\n1/01-01 --stale-only"),
        "{suggestion}"
    );
    assert!(!suggestion.contains('\n'), "{suggestion}");
}

/// TR1's plan and phase both changed since `R`, TR1's verdict stale after the edit, and the edges `label` dropped.
fn without_edge(label: &str) -> PassInput {
    let nodes = vec![
        modified("plan", PLAN, &["objective"], &[]),
        modified("phase", PHASE, &[], &["title"]),
    ];
    let mut input = since_r(nodes, stale_after_an_edit());
    input.edges.retain(|e| e.label != label);
    input
}

#[test]
fn a_stale_truth_with_no_plan_names_no_plan_and_suggests_nothing() {
    let advisory = on_tr1(&without_edge("plan"));
    assert_eq!(advisory.suggestion, None);
    assert!(
        advisory
            .message
            .ends_with("only the files it cites changed: 'src/tasks.js'"),
        "{}",
        advisory.message
    );
}

#[test]
fn a_plan_with_no_phase_names_its_plan_and_not_the_phase() {
    let advisory = on_tr1(&without_edge("phase"));
    assert!(
        advisory.message.contains(&format!(
            "the spec it reads changed: plan '{PLAN}' (objective); the files it cites changed too: 'src/tasks.js'"
        )),
        "{}",
        advisory.message
    );
    assert!(
        !advisory.message.contains("phase '"),
        "{}",
        advisory.message
    );
    assert!(advisory.suggestion.is_some());
}

/// At the base TR1 held a missing or unrecorded verdict, whose files the host does not fill, or none: the files it
/// cites are named without a claim of which changed.
#[test]
fn a_truth_with_no_verdict_listing_files_at_the_base_claims_no_file() {
    let at_the_base = [
        vec![verdict("missing", &[], &[])],
        vec![verdict("unrecorded", &[], &[])],
        vec![],
    ];
    for judgements in at_the_base {
        let input = input(
            vec![stale_after_an_edit()],
            json!({
                "delta": delta(revision(), vec![]),
                "previous": { "revision": R, "entities": { TR1: { "kind": "truth", "judgements": judgements } } },
            }),
        );
        let message = on_tr1(&input).message;
        assert!(
            message.ends_with(
                "the spec it reads is unchanged; it cites 'src/tasks.js', 'test/tasks.test.js'"
            ),
            "{judgements:?}: {message}"
        );
    }
}
