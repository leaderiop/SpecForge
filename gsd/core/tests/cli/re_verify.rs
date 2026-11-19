//! `specforge gsd re-verify`: the verifier's verdicts of a phase or a plan and which no longer hold, on the taskboard
//! twin, whose phase 01 carries a fresh verdict on each of its seven must-haves.

use serde_json::Value;

use super::harness::taskboard;

/// `(entity, state)` of each listed item, in answer order.
fn listed(answer: &Value) -> Vec<(String, String)> {
    answer["items"]
        .as_array()
        .expect("items")
        .iter()
        .map(|i| {
            let text = |k: &str| i[k].as_str().expect("a text").to_string();
            (text("entity"), text("state"))
        })
        .collect()
}

#[test]
fn every_verdict_of_the_verified_phase_holds() {
    let p = taskboard();
    let all = p.re_verify("v1.0/01", false);
    let items = listed(&all);
    assert_eq!(items.len(), 7, "{all:#}");
    assert!(items.iter().all(|(_, state)| state == "fresh"), "{all:#}");
    assert_eq!(
        (all["holding"].as_u64(), all["not_holding"].as_u64()),
        (Some(7), Some(0))
    );
    let stale = p.re_verify("v1.0/01", true);
    assert_eq!(listed(&stale), []);
    let text = p.run(&["gsd", "re-verify", "v1.0/01", "--stale-only"]);
    assert_eq!(text.code, 0, "{}", text.all());
    assert!(
        text.stdout.contains("every verdict holds (7)"),
        "{}",
        text.all()
    );
}

#[test]
fn an_edit_lists_exactly_the_verdicts_that_read_the_file() {
    let p = taskboard();
    p.write(
        "src/tasks.js",
        &format!("{}\n// edited\n", p.read("src/tasks.js")),
    );
    let answer = p.re_verify("v1.0/01", true);
    let items = answer["items"].as_array().expect("items");
    let by_entity = |id: &str| {
        items
            .iter()
            .find(|i| i["entity"] == id)
            .unwrap_or_else(|| panic!("{id} is listed: {answer:#}"))
    };
    // The three truths cite src/tasks.js; 01-01's artifact is it, and both key links point at it.
    for truth in [
        "v1.0/01/01-01/TR1",
        "v1.0/01/01-02/TR1",
        "v1.0/01/01-02/TR2",
    ] {
        let item = by_entity(truth);
        assert_eq!(item["state"], "stale", "{truth}");
        assert_eq!(
            item["changed"],
            serde_json::json!([{ "input": "cited", "selector": "cited" }]),
            "{truth}"
        );
    }
    assert_eq!(
        by_entity("v1.0/01/01-01/AR1")["changed"],
        serde_json::json!([{ "input": "file", "selector": "file(self.path)" }])
    );
    for link in ["v1.0/01/01-01/KL1", "v1.0/01/01-02/KL1"] {
        assert_eq!(
            by_entity(link)["changed"],
            serde_json::json!([{ "input": "to", "selector": "file(self.to)" }]),
            "{link}"
        );
    }
    // 01-02's artifact reads src/cli.js only: its verdict still holds and is not listed.
    assert_eq!(items.len(), 6, "{answer:#}");
    assert_eq!(
        (answer["holding"].as_u64(), answer["not_holding"].as_u64()),
        (Some(1), Some(6))
    );
    let text = p.run(&["gsd", "re-verify", "v1.0/01", "--stale-only"]);
    assert!(
        text.stdout
            .contains("gsd/E501 v1.0/01/01-01/AR1 stale: file (file(self.path))"),
        "{}",
        text.all()
    );
    assert!(
        text.stdout.contains("6 of 7 verdict(s) no longer hold"),
        "{}",
        text.all()
    );
}

#[test]
fn a_phase_whose_plans_are_unexecuted_owes_no_verdict() {
    let p = taskboard();
    let answer = p.re_verify("v1.0/02", true);
    assert_eq!(listed(&answer), []);
    assert_eq!(p.re_verify("v1.0/02", false)["holding"], 0);
    let text = p.run(&["gsd", "re-verify", "v1.0/02"]);
    assert!(
        text.stdout.contains("no verdict is owed in v1.0/02"),
        "{}",
        text.all()
    );
}

#[test]
fn a_plan_subject_lists_its_own_verdicts_and_an_unknown_one_is_refused() {
    let p = taskboard();
    let plan = p.re_verify("v1.0/01/01-01", false);
    assert_eq!(
        listed(&plan)
            .into_iter()
            .map(|(e, _)| e)
            .collect::<Vec<_>>(),
        [
            "v1.0/01/01-01/AR1",
            "v1.0/01/01-01/KL1",
            "v1.0/01/01-01/TR1"
        ]
    );
    let run = p.run(&["gsd", "re-verify", "v9.0/07"]);
    assert_eq!(run.code, 1, "{}", run.all());
    assert!(
        run.stderr.contains("no phase or plan 'v9.0/07'"),
        "{}",
        run.all()
    );
}
