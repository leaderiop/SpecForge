//! The judged flow as an agent walks it: query the inputs, judge, record, and judge again once an input has changed.
//! No model is involved: the test plays the judge, as the plan's "What it proves" says.

use super::harness::project;

const PLAN: &str = "v1.0/01/01-02";
const RULES: [&str; 4] = ["W311", "W313", "W315", "W316"];

#[test]
fn judged_flow_end_to_end() {
    let p = project("plan");
    // The plan reads the project's instructions: a CLAUDE.md the verdict will name in its basis.
    p.write("CLAUDE.md", "# Rules\n\nNo task is longer than an hour.\n");
    let status = || p.fields_of(PLAN)["status"].clone();
    assert_eq!(status(), "draft");

    // 1. Query the four rules' inputs: each is missing a verdict, and the instructions are among the inputs.
    let mut bases = Vec::new();
    for code in RULES {
        let inputs = p.inputs(PLAN, code);
        assert_eq!(inputs["verdict"]["state"], "missing", "{code}: {inputs}");
        let text = inputs.to_string();
        assert!(
            text.contains("CLAUDE.md"),
            "{code}: the instructions are an input: {text}"
        );
        bases.push(inputs["basis"].to_string());
    }

    // 2. Record the four: the plan is checked.
    for code in RULES {
        let run = p.record(PLAN, code, "passed", &[]);
        assert_eq!(run.code, 0, "{code}: {}", run.all());
        assert_eq!(p.inputs(PLAN, code)["verdict"]["state"], "fresh", "{code}");
    }
    assert_eq!(status(), "checked");

    // 3. Edit the instructions: every verdict is stale and the plan is a draft again.
    p.write(
        "CLAUDE.md",
        "# Rules\n\nNo task is longer than half an hour.\n",
    );
    let edited = p.read("CLAUDE.md");
    assert!(edited.contains("half an hour"), "the edit landed: {edited}");
    for code in RULES {
        assert_eq!(p.inputs(PLAN, code)["verdict"]["state"], "stale", "{code}");
    }
    assert_eq!(status(), "draft");
    let stale: Vec<_> = p
        .report_all()
        .into_iter()
        .filter(|(c, m)| c == "gsd/W311" && m.contains(PLAN))
        .collect();
    assert_eq!(stale.len(), 1, "{stale:?}");
    assert!(stale[0].1.contains("is stale: "), "{stale:?}");

    // 4. Query again: the basis has changed; recording over the old one is refused.
    for (code, before) in RULES.iter().zip(&bases) {
        assert_ne!(&p.inputs(PLAN, code)["basis"].to_string(), before, "{code}");
    }
    let refused = p.record_over(PLAN, "W311", "passed", &bases[0], &[]);
    assert_ne!(refused.code, 0, "{}", refused.all());
    assert!(
        refused.all().contains("stale_revision"),
        "{}",
        refused.all()
    );

    // 5. Judge again over the new basis: the plan is checked again.
    for code in RULES {
        let run = p.record(PLAN, code, "passed", &[]);
        assert_eq!(run.code, 0, "{code}: {}", run.all());
    }
    assert_eq!(status(), "checked");
}
