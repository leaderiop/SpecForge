//! The route `gsd/requirement-thread` (ADR 0331 D8), end to end: `specforge trace` follows a requirement to the
//! commit whose `Gsd-Task:` trailer names its task. The facts half (test, status, verdict) is the taskboard parity
//! golden's.

use super::harness::project;

/// Each step's matches, as `(step, entity id, the text of its annotations)`.
fn matches(trace: &serde_json::Value) -> Vec<(String, String, String)> {
    let steps = trace["steps"]
        .as_array()
        .expect("a route trace lists its steps");
    steps
        .iter()
        .flat_map(|step| {
            step["matches"].as_array().into_iter().flatten().map(|m| {
                let annotations = m["annotations"].as_array().into_iter().flatten();
                let text: Vec<&str> = annotations.filter_map(|a| a["text"].as_str()).collect();
                (
                    step["name"].as_str().unwrap_or_default().to_string(),
                    m["entity_id"].as_str().unwrap_or_default().to_string(),
                    text.join(" "),
                )
            })
        })
        .collect()
}

#[test]
fn trace_follows_a_requirement_to_its_commit() {
    let p = project("execute");
    p.commit();
    let hash = p.git_commit(
        "chore: wire it\n\nGsd-Task: 01-01/T1\n",
        &[("src/w.js", "w\n")],
        "2026-10-29T09:00:00Z",
    );
    let collect = p.collect(&["--runner", "gsd-git"]);
    assert_eq!(collect.code, 0, "{}", collect.all());

    let run = p.run(&["trace", "TASK-01", "--format", "json"]);
    let trace: serde_json::Value = serde_json::from_str(&run.stdout)
        .unwrap_or_else(|e| panic!("trace prints JSON ({e}): {}", run.all()));
    assert_eq!(trace["route"], "gsd/requirement-thread", "{trace:#}");
    let s = |text: &str| text.to_string();
    let found = matches(&trace);
    assert_eq!(
        found[..3],
        [
            (s("plan"), s("v1.0/01/01-01"), s("")),
            (s("task"), s("v1.0/01/01-01/T1"), s("")),
            (s("task"), s("v1.0/01/01-01/T2"), s("")),
        ],
        "{trace:#}"
    );
    assert!(
        found.contains(&(s("commit"), hash[..12].to_string(), s("(Gsd-Task)"))),
        "{trace:#}"
    );
    // Without `@code/core` the step into its `test` kind is inert, named, and never a failure.
    assert_eq!(
        trace["skipped"],
        serde_json::json!(["step 'test'"]),
        "{trace:#}"
    );
    assert!(run.stderr.contains("I210"), "{}", run.all());
}
