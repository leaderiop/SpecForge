//! One edited file re-judges exactly the verdicts that read it, on the taskboard twin, whose phase 01 carries a fresh
//! verifier verdict on each of its seven must-haves, each truth's citing the files it is about (ADR 0377): a truth's
//! verdict reads what it cites, an artifact's its `path`, a key link's its `from` and `to`. After the listed verdicts
//! are recorded again, every other verdict is byte-identical, in the spec and in the ledger.

use serde_json::{Value, json};

use super::harness::{Project, taskboard};

const PHASE: &str = "v1.0/01";
/// The truths of phase 01: each cites `src/tasks.js` and `test/tasks.test.js`, none `src/cli.js`.
const TRUTHS: [&str; 3] = [
    "v1.0/01/01-01/TR1",
    "v1.0/01/01-02/TR1",
    "v1.0/01/01-02/TR2",
];

/// Appends one comment line to a file of the project.
pub(super) fn touch(p: &Project, file: &str) {
    let text = p.read(file);
    assert!(text.ends_with('\n'), "{file} ends with a line break");
    p.write(file, &format!("{text}// edited\n"));
}

/// `(entity, changed)` of each verdict `re-verify --stale-only` lists, each stale, in answer order.
fn stale(p: &Project) -> Vec<(String, Value)> {
    let answer = p.re_verify(PHASE, true);
    answer["items"]
        .as_array()
        .expect("items")
        .iter()
        .map(|item| {
            assert_eq!(item["state"], "stale", "{answer:#}");
            let entity = item["entity"].as_str().expect("an entity").to_string();
            (entity, item["changed"].clone())
        })
        .collect()
}

/// The state of the verifier's verdict on `entity`, as `query --inputs` answers it.
fn state(p: &Project, entity: &str) -> String {
    let inputs = p.inputs(entity, "E501");
    inputs["verdict"]["state"]
        .as_str()
        .unwrap_or_else(|| panic!("a verdict state: {inputs:#}"))
        .to_string()
}

fn changed(input: &str, selector: &str) -> Value {
    json!([{ "input": input, "selector": selector }])
}

#[test]
fn nothing_is_stale_before_an_edit() {
    let p = taskboard();
    assert_eq!(stale(&p), []);
    for truth in TRUTHS {
        assert_eq!(state(&p, truth), "fresh", "{truth}");
    }
}

/// `src/cli.js` is cited by no truth: only 01-02's artifact (its `path`) and key link (its `from`) read it.
#[test]
fn an_edit_to_the_command_line_stales_no_truth() {
    let p = taskboard();
    touch(&p, "src/cli.js");
    assert_eq!(
        stale(&p),
        [
            (
                "v1.0/01/01-02/AR1".to_string(),
                changed("file", "file(self.path)")
            ),
            (
                "v1.0/01/01-02/KL1".to_string(),
                changed("from", "file(self.from)")
            ),
        ]
    );
    for truth in TRUTHS {
        assert_eq!(state(&p, truth), "fresh", "{truth}");
    }
}

/// `src/tasks.js` is cited by every truth: the three are listed with `changed` = `cited`, beside the artifact and the
/// key links that name it. Recording exactly those again leaves every other verdict as it was, byte for byte.
#[test]
fn an_edit_to_the_task_model_re_judges_exactly_the_verdicts_citing_it() {
    let p = taskboard();
    let before = p.verdicts();
    assert_eq!((before.blocks.len(), before.ledger.len()), (7, 7));
    touch(&p, "src/tasks.js");
    let listed = stale(&p);
    let cited = changed("cited", "cited");
    assert_eq!(
        listed,
        [
            (
                "v1.0/01/01-01/AR1".to_string(),
                changed("file", "file(self.path)")
            ),
            (
                "v1.0/01/01-01/KL1".to_string(),
                changed("to", "file(self.to)")
            ),
            ("v1.0/01/01-01/TR1".to_string(), cited.clone()),
            (
                "v1.0/01/01-02/KL1".to_string(),
                changed("to", "file(self.to)")
            ),
            ("v1.0/01/01-02/TR1".to_string(), cited.clone()),
            ("v1.0/01/01-02/TR2".to_string(), cited),
        ]
    );
    for (id, _) in &listed {
        let citing = p.citing(id);
        let extra: Vec<&str> = citing.iter().map(String::as_str).collect();
        let run = p.record(id, "E501", "verified", &extra);
        assert_eq!(run.code, 0, "{id}: {}", run.all());
    }
    assert_eq!(stale(&p), []);
    assert_eq!(p.re_verify(PHASE, false)["fresh"], 7);
    let rejudged: Vec<(String, String)> = listed
        .into_iter()
        .map(|(id, _)| (id, "gsd/E501".to_string()))
        .collect();
    before.assert_only(&p.verdicts(), &rejudged);
}
