use serde_json::{Value, json};
use specforge_extension_sdk::CommandGraph;

use super::*;

const NONCE: &str = "0123456789abcdef0123456789abcdef";
const SEED: Seed = Seed(0xFEDC_BA98_7654_3210);

/// The command's answer over a graph holding `taken`, at a pinned clock and nonce.
fn called(args: Value, nonce: &str, taken: &[&str]) -> CommandOutput {
    let nodes = taken
        .iter()
        .map(|id| GraphNode {
            id: (*id).to_string(),
            kind: "plan".to_string(),
            ..Default::default()
        })
        .collect();
    let input = CommandInput {
        args: args.as_object().expect("args are an object").clone(),
        graph: CommandGraph::new(nodes, Vec::new()),
        format: CommandFormat::Json,
        today: "2026-11-05".to_string(),
        now: "2026-11-05T10:00:00Z".to_string(),
        nonce: nonce.to_string(),
        ..Default::default()
    };
    crate::build()
        .call_command("cmd__quick_init", &input)
        .expect("the command is declared")
}

fn answer_of(args: Value) -> Value {
    let out = called(args, NONCE, &[]);
    assert_eq!(out.exit_code, 0, "{}", out.stderr);
    serde_json::from_str(&out.stdout).expect("the answer is JSON")
}

fn refusal_of(args: Value) -> String {
    let out = called(args, NONCE, &[]);
    assert_eq!(out.exit_code, 1, "{}", out.stdout);
    out.stderr
}

#[test]
fn a_quick_id_is_the_day_and_three_base_36_digits() {
    let minted = ids(SEED, "261105", std::iter::empty(), 200).unwrap();
    assert_eq!(minted.len(), 200);
    for id in &minted {
        assert!(
            QuickId::parse(id).is_some_and(|id| id.is_of("261105")),
            "{id}"
        );
    }
}

#[test]
fn ids_in_the_graph_and_in_the_call_are_skipped() {
    let first = ids(SEED, "261105", std::iter::empty(), 1)
        .unwrap()
        .remove(0);
    let next = ids(SEED, "261105", [first.as_str()].into_iter(), 1)
        .unwrap()
        .remove(0);
    assert_ne!(next, first);
    // A call never mints one id twice, even where every draw lands in one taken run.
    let many = ids(SEED, "261105", std::iter::empty(), 5000).unwrap();
    let distinct: BTreeSet<&String> = many.iter().collect();
    assert_eq!(distinct.len(), many.len());
}

#[test]
fn the_same_nonce_mints_the_same_ids() {
    let of = |nonce: &str| {
        let out = called(json!({"description": "a"}), nonce, &[]);
        let answer: Value = serde_json::from_str(&out.stdout).expect("the answer is JSON");
        answer["items"][0]["id"].clone()
    };
    assert_eq!(of(NONCE), of(NONCE));
    assert_ne!(of(NONCE), of("00000000000000000000000000000001"));
}

#[test]
fn a_full_day_is_refused() {
    let all: Vec<String> = (0..QuickId::PER_DAY)
        .map(|n| QuickId::nth("261105", n).into_string())
        .collect();
    let taken = || all.iter().map(String::as_str);
    let why = ids(SEED, "261105", taken(), 1).unwrap_err();
    assert!(why.contains("0 of 261105's 46656 are free"), "{why}");
    // The last free id is found wherever the draw lands; another day's ids take none of this day's.
    let last = ids(SEED, "261105", taken().skip(1), 1).unwrap();
    assert_eq!(last, ["261105-000"]);
    assert_eq!(ids(SEED, "261106", taken(), 1).unwrap().len(), 1);
}

#[test]
fn a_task_list_loses_its_markers_and_blank_lines() {
    let list = "- Fix login\n\n* Trim titles\n1. Add a cap\n2) Sort rows\n   \n  - Keep -- this\nno marker\n-tight\n1.5 apples\n-\n";
    assert_eq!(
        tasks(list),
        [
            "Fix login",
            "Trim titles",
            "Add a cap",
            "Sort rows",
            "Keep -- this",
            "no marker",
            "-tight",
            "1.5 apples"
        ]
    );
    // A list read from a file with CRLF ends: the terminator is no part of the task.
    assert_eq!(tasks("- a\r\n- b\r\n"), ["a", "b"]);
}

#[test]
fn a_batch_needs_two_tasks() {
    for (list, holds) in [("- only one\n", 1), ("\n \n", 0)] {
        let why = refusal_of(json!({"batch": true, "tasks": list}));
        assert!(
            why.contains(&format!(
                "a batch needs at least 2 tasks; the list holds {holds}"
            )),
            "{why}"
        );
    }
    let why = refusal_of(json!({"batch": true}));
    assert!(why.contains("--batch needs --tasks"), "{why}");
    let answer = answer_of(json!({"batch": true, "tasks": "- a\n- b\n"}));
    assert_eq!(answer["items"][1]["batch"], "261105-batch");
    assert_eq!(answer["items"][1]["count"], 2);
    assert_eq!(answer["items"][1]["description"], "b");
    assert_eq!(answer["items"][1]["slug"], "b");
}

#[test]
fn the_template_takes_num_and_slug() {
    assert_eq!(
        branch("quick/{num}-{slug}", "261105-a1b", "trim")
            .expect("git takes it")
            .as_deref(),
        Some("quick/261105-a1b-trim")
    );
    assert_eq!(branch("", "261105-a1b", "trim"), Ok(None));
    for template in ["--help", "-{num}", "a..{slug}", "{slug}.lock"] {
        let why = branch(template, "261105-a1b", "trim").unwrap_err();
        assert!(why.contains("git refuses"), "{template}: {why}");
    }
    let answer = answer_of(json!({"description": "Trim titles!", "template": "q/{num}/{slug}"}));
    let item = &answer["items"][0];
    let id = item["id"].as_str().unwrap();
    assert_eq!(item["slug"], "trim-titles");
    assert_eq!(item["description"], "Trim titles!");
    assert_eq!(item["branch_name"], format!("q/{id}/trim-titles"));
    assert_eq!(item["date"], "2026-11-05");
    assert_eq!(item["count"], 1);
    assert!(item.get("batch").is_none());
    // one row of one call reads everything: the item holds it, and nothing but the items is answered
    assert_eq!(answer.as_object().unwrap().len(), 1, "{answer}");
    assert!(item.get("timestamp").is_none());
}

#[test]
fn a_slug_is_cut_to_40_and_never_empty() {
    assert_eq!(quick_slug(&"a".repeat(50)).len(), 40);
    // the cut lands on a dash: it goes
    assert_eq!(quick_slug(&format!("{} b", "a".repeat(39))), "a".repeat(39));
    let answer = answer_of(json!({"description": "!!!"}));
    assert_eq!(answer["items"][0]["slug"], answer["items"][0]["id"]);
}

#[test]
fn an_id_is_for_a_task() {
    let why = refusal_of(json!({}));
    assert!(why.contains("--description or --tasks"), "{why}");
    let why = refusal_of(json!({"template": "q/{num}"}));
    assert!(why.contains("--description or --tasks"), "{why}");
}

#[test]
fn the_sources_of_tasks_exclude_each_other_and_a_control_character_is_refused() {
    assert!(refusal_of(json!({"description": "a", "tasks": "b"})).contains("invalid_input"));
    assert!(refusal_of(json!({"tasks": "a\u{7}\nb"})).contains("U+0007"));
    assert!(refusal_of(json!({"description": "a\u{7}"})).contains("U+0007"));
    assert!(refusal_of(json!({"tasks": "\n\n"})).contains("--tasks holds no task"));
}

#[test]
fn no_nonce_is_refused() {
    for bad in [
        "",
        "short",
        "0123456789ABCDEF0123456789ABCDEF",
        &"g".repeat(32),
    ] {
        let out = called(json!({"description": "a"}), bad, &[]);
        assert_eq!(out.exit_code, 1, "{bad}: {}", out.stdout);
        assert!(
            out.stderr.contains("the host sent no nonce"),
            "{}",
            out.stderr
        );
    }
}

#[test]
fn an_id_of_the_graph_is_not_minted() {
    let first = answer_of(json!({"description": "a"}))["items"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let out = called(json!({"description": "a"}), NONCE, &[first.as_str()]);
    assert_eq!(out.exit_code, 0, "{}", out.stderr);
    let answer: Value = serde_json::from_str(&out.stdout).unwrap();
    assert_ne!(answer["items"][0]["id"], first.as_str());
}

#[test]
fn the_batch_id_skips_a_taken_one() {
    assert_eq!(batch_id("261112", |_| false), "261112-batch");
    let taken = ["261112-batch", "261112-batch-2"];
    assert_eq!(
        batch_id("261112", |id| taken.contains(&id)),
        "261112-batch-3"
    );
}
