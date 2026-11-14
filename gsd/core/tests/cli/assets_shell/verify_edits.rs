//! The verify family's steps (the verifier, its spawn, UAT, validation), run as the assets hold them on a taskboard
//! copy with only `specforge` on the `PATH`, and the entities and answers they produce.

use super::{Shell, asset, typed};
use crate::harness::taskboard;

/// The code of the first bash fence of `text` holding `mark`.
pub(super) fn fence_holding(text: &str, mark: &str) -> String {
    let fence = gsd_kit::markdown::fences(text)
        .into_iter()
        .find(|f| f.language == "bash" && f.text.contains(mark))
        .unwrap_or_else(|| panic!("no bash fence holds {mark}"));
    super::code(&fence.text).to_string()
}

/// The verifier's Step 6 traces the phase's requirements along their threads: each requirement of `v1.0/01` with its
/// plan and its verified truth.
#[test]
fn the_verifier_traces_its_phases_requirements() {
    let Some(sh) = Shell::only(&[]) else {
        return;
    };
    let p = taskboard();
    let code = fence_holding(
        &asset("agents/gsd-verifier.md"),
        "trace --route gsd/requirement-thread",
    );
    let (out, err, status) = sh.run_in(p.root(), &format!("PHASE_ID=v1.0/01\n{code}"));
    assert_eq!(status, 0, "{err}");
    for want in ["TASK-01", "TASK-02", "plan 01-01", "TR1 verified"] {
        assert!(out.contains(want), "{want}: {out}");
    }
}

/// A verdict cites a test fact by the id the thread's test step prints: the verification template's record fence,
/// filled with that id, the basis the truth's inputs give and no finding, is accepted, and the truth's inputs then
/// list the cited test.
#[test]
fn the_verifier_cites_a_test_by_fact_id() {
    let Some(sh) = Shell::only(&[]) else {
        return;
    };
    let p = taskboard();
    let run = p.collect_node_test_golden();
    assert_eq!(run.code, 0, "{}", run.all());
    let trace = p.run(&[
        "trace",
        "--route",
        "gsd/requirement-thread",
        "--in",
        "v1.0/01",
        "--format",
        "json",
    ]);
    let trace: serde_json::Value = serde_json::from_str(&trace.stdout).expect("the threads");
    let test = trace["traces"]
        .as_array()
        .expect("traces")
        .iter()
        .find(|t| t["entity_id"] == "TASK-01")
        .and_then(|t| t["steps"].as_array()?.iter().find(|s| s["name"] == "test"))
        .and_then(|s| s["matches"][0]["entity_id"].as_str())
        .expect("TASK-01's test")
        .to_string();
    let truth = "v1.0/01/01-01/TR1";
    let inputs = p.run(&[
        "query",
        truth,
        "--inputs",
        "gsd/E501",
        "--cite-file",
        "src/tasks.js",
        "--cite-test",
        &test,
        "--format",
        "json",
    ]);
    let inputs: serde_json::Value = serde_json::from_str(&inputs.stdout).expect("the inputs");
    let cited = serde_json::json!([{"file": "src/tasks.js"}, {"test": test}]).to_string();
    let record = typed::filled(
        &fence_holding(
            &asset("gsd-core/templates/verification-report.md"),
            "record-verdict",
        ),
        &[
            ("TRUTH", truth),
            ("OUTCOME", "verified"),
            ("BASIS", &inputs["basis"].to_string()),
            ("CITED", &cited),
            ("FINDINGS", "[]"),
        ],
    );
    let done = typed::apply(&sh, &p, &record);
    assert_eq!(done.code, 0, "{}{}", done.out, done.err);
    assert!(done.added.is_empty(), "{:?}", done.added);
    let inputs = p.run(&["query", truth, "--inputs", "gsd/E501", "--format", "json"]);
    assert!(inputs.stdout.contains(&test), "{}", inputs.stdout);
}

/// The verifier spawn's reading list opens with the `gsd/verifier` pack line and reads no context window; run with
/// `PHASE_ID=v1.0/01`, the line serves the phase's truths.
#[test]
fn the_verifier_spawn_reads_one_context_line() {
    let Some(sh) = Shell::only(&[]) else {
        return;
    };
    let text = asset("gsd-core/workflows/execute-phase/steps/verify-phase-goal.md");
    assert!(!text.contains("CONTEXT_WINDOW"));
    let reading = super::block(&text, "<required_reading>", "</required_reading>");
    let first = reading.lines().nth(1).expect("a first line");
    assert!(
        first.starts_with("specforge context gsd/verifier"),
        "{reading}"
    );
    let p = taskboard();
    let line = first.replace("{PHASE_ID}", "$PHASE_ID");
    let (out, err, code) = sh.run_in(p.root(), &format!("PHASE_ID=v1.0/01\n{line}"));
    assert_eq!(code, 0, "{err}");
    assert!(out.contains("v1.0/01/01-01/TR1"), "{out}");
}

/// The UAT template's fences, filled, create a session on `v1.0/01` (its own taken out) with one test, then a gap on
/// that test.
#[test]
fn the_uat_template_records_a_session_a_test_and_a_gap() {
    let Some(sh) = Shell::only(&[]) else {
        return;
    };
    let p = without_a_session();
    let template = asset("gsd-core/templates/UAT.md");
    let session = typed::filled(
        &fence_holding(&template, "new uat_session"),
        &[("PHASE_ID", "v1.0/01")],
    );
    let done = typed::apply(&sh, &p, &session);
    assert_eq!(done.code, 0, "{}{}", done.out, done.err);
    assert_eq!(p.fields_of("v1.0/01/01-UAT")["phase"], "v1.0/01");
    let test = typed::filled(
        &fence_holding(&template, "new uat_test"),
        &[
            ("UAT_SESSION", "v1.0/01/01-UAT"),
            ("TEST_ID", "U1"),
            ("NAME", "Complete a task"),
            ("EXPECTED", "`node src/cli.js done 1` marks task 1 done"),
        ],
    );
    let done = typed::apply(&sh, &p, &test);
    assert_eq!(done.code, 0, "{}{}", done.out, done.err);
    let gap = typed::filled(
        &fence_holding(&template, "new uat_gap"),
        &[
            ("TEST", "v1.0/01/01-UAT/U1"),
            ("GAP_ID", "G1"),
            ("SEVERITY", "major"),
            ("TRUTH", "Completing a task marks it done"),
            ("REASON", "User reported: nothing happens"),
        ],
    );
    let done = typed::apply(&sh, &p, &gap);
    assert_eq!(done.code, 0, "{}{}", done.out, done.err);
    assert!(done.added.is_empty(), "{:?}", done.added);
    assert_eq!(
        p.fields_of("v1.0/01/01-UAT/U1/G1")["reason"],
        "User reported: nothing happens"
    );
}

/// diagnose-issues' fence, filled, records the diagnosis on `01-UAT`'s gap `G1`: its root cause, its artifacts and
/// what is missing, with no new finding.
#[test]
fn a_uat_gap_diagnosis_is_a_typed_edit() {
    let Some(sh) = Shell::only(&[]) else {
        return;
    };
    let p = taskboard();
    let gap = "v1.0/01/01-UAT/U2/G1";
    let diagnosis = typed::filled(
        &fence_holding(
            &asset("gsd-core/workflows/diagnose-issues.md"),
            "root_cause",
        ),
        &[
            ("GAP", gap),
            (
                "ROOT_CAUSE",
                "done() rethrows the lookup error before printing",
            ),
            (
                "ARTIFACTS",
                r#"[{"path": "src/cli.js", "issue": "the catch rethrows"}]"#,
            ),
            (
                "MISSING",
                "Print the message without the stack\nTest the one-line output",
            ),
        ],
    );
    let done = typed::apply(&sh, &p, &diagnosis);
    assert_eq!(done.code, 0, "{}{}", done.out, done.err);
    assert!(done.added.is_empty(), "{:?}", done.added);
    let fields = p.fields_of(gap);
    assert_eq!(
        fields["root_cause"],
        "done() rethrows the lookup error before printing"
    );
    assert_eq!(fields["artifacts"][0]["path"], "src/cli.js");
    assert_eq!(
        fields["missing"].as_array().map(Vec::len),
        Some(2),
        "{fields}"
    );
}

/// The line verify-work shows the person for a deferred follow-up, run as the person runs it on `01-UAT`'s `U3`, adds
/// one `follow_ups` entry to the session, and the step's count reads it.
#[test]
fn a_deferred_follow_up_lands_on_the_session() {
    let Some(sh) = Shell::only(&[]) else {
        return;
    };
    let text = asset("gsd-core/workflows/verify-work.md");
    assert!(text.contains(
        "`specforge gsd set-uat-result <uat_test> skipped --by <who> --follow-up \"<their words>\"`"
    ));
    let p = taskboard();
    let run = p.run_as_person(&[
        "gsd",
        "set-uat-result",
        "v1.0/01/01-UAT/U3",
        "skipped",
        "--by",
        "Sam",
        "--follow-up",
        "print the version beside the usage line",
    ]);
    assert_eq!(run.code, 0, "{}", run.all());
    let follow_ups = p.fields_of("v1.0/01/01-UAT")["follow_ups"].clone();
    assert_eq!(follow_ups.as_array().map(Vec::len), Some(1), "{follow_ups}");
    let count = fence_holding(&text, "totals.follow_ups");
    let (out, err, code) = sh.run_in(p.root(), &format!("PHASE_ID=v1.0/01\n{count}"));
    assert_eq!(code, 0, "{err}");
    assert!(out.contains("deferred follow-ups: 1"), "{out}");
}

/// verify-work's [P] fence for one follow-up adds the backlog phase `v1.0/999.1` and no finding: a backlog phase is
/// exempt from gsd/W103.
#[test]
fn uat_follow_ups_promote_to_backlog_phases() {
    let Some(sh) = Shell::only(&[]) else {
        return;
    };
    let p = taskboard();
    let promote = typed::filled(
        &fence_holding(&asset("gsd-core/workflows/verify-work.md"), "--backlog"),
        &[
            ("TEST", "3"),
            ("IDEA", "print the version beside the usage line"),
        ],
    );
    let done = typed::apply(&sh, &p, &format!("PHASE_ID=v1.0/01\n{promote}"));
    assert_eq!(done.code, 0, "{}{}", done.out, done.err);
    assert!(done.added.is_empty(), "{:?}", done.added);
    let phase = p.fields_of("v1.0/999.1");
    assert_eq!(
        phase["goal"], "print the version beside the usage line",
        "{phase}"
    );
}

/// validate-phase §6's fence, filled and run once per row on `v1.0/02` (no validation yet; its tasks given an
/// automated verify, as `gsd/E314` asks of a validated phase), creates one validation and two rows with no new finding;
/// the auditor spawn's context line, run with that phase, names the validation.
#[test]
fn the_validation_audit_writes_rows() {
    let Some(sh) = Shell::only(&[]) else {
        return;
    };
    let p = taskboard();
    for task in ["v1.0/02/02-01/T1", "v1.0/02/02-01/T2"] {
        let run = p.run(&[
            "set",
            task,
            "verify",
            r#"[{"automated": "npm test", "fails_when": "a task test fails"}]"#,
        ]);
        assert_eq!(run.code, 0, "{}", run.all());
    }
    let text = asset("gsd-core/workflows/validate-phase.md");
    let fence = fence_holding(&text, "new validation_row");
    for (row, task) in [("R1", "v1.0/02/02-01/T1"), ("R2", "v1.0/02/02-01/T2")] {
        let filled = typed::filled(
            &fence,
            &[
                ("PHASE_ID", "v1.0/02"),
                ("FRAMEWORK", "node:test"),
                ("QUICK_CMD", "node --test"),
                ("FULL_CMD", "npm test"),
                ("ROW_ID", row),
                ("TASK", task),
                ("TEST_TYPE", "unit"),
                ("REQUIREMENT", "TASK-03"),
                ("TEST_FILE", "test/tasks.test.js"),
                ("COMMAND", "node --test test/tasks.test.js"),
            ],
        );
        let done = typed::apply(&sh, &p, &filled);
        assert_eq!(done.code, 0, "{}{}", done.out, done.err);
        assert!(done.added.is_empty(), "{:?}", done.added);
    }
    assert_eq!(p.nodes_of("validation").len(), 1);
    assert_eq!(p.nodes_of("validation_row").len(), 2);
    let line = text
        .lines()
        .find(|l| l.contains("specforge context gsd/nyquist-auditor"))
        .expect("the spawn's context line");
    let start = line.find("specforge context").expect("the command");
    // the command ends at the prompt's first `\n` or at the tag
    let rest = &line[start..];
    let end = rest
        .find("\\n")
        .or_else(|| rest.find("</required_reading>"))
        .expect("its end")
        + start;
    let command = line[start..end]
        .replace("\\\"", "\"")
        .replace("{PHASE_ID}", "$PHASE_ID");
    let (out, err, code) = sh.run_in(p.root(), &format!("PHASE_ID=v1.0/02\n{command}"));
    assert_eq!(code, 0, "{err}");
    assert!(out.contains("02-VALIDATION"), "{out}");
}

/// The taskboard with phase `v1.0/01`'s UAT entities taken out: its summaries stay, so a session can be created on it.
fn without_a_session() -> crate::harness::Project {
    let p = taskboard();
    p.write(
        "spec/verify.spec",
        "verification 01-VERIFICATION \"Phase 01 verification\" {\n  phase v1.0/01\n}\n",
    );
    p
}

/// The words of the line a step shows the person, the code span of `text` that starts with `start`, after its
/// `specforge`: a double-quoted word is one word, and each placeholder of `values` is filled.
fn person_line(text: &str, start: &str, values: &[(&str, &str)]) -> Vec<String> {
    let line = super::span(text, start);
    let (mut words, mut word, mut quoted) = (Vec::new(), String::new(), false);
    for c in line.chars() {
        match c {
            '"' => quoted = !quoted,
            ' ' if !quoted => words.extend((!word.is_empty()).then(|| std::mem::take(&mut word))),
            _ => word.push(c),
        }
    }
    words.extend((!word.is_empty()).then_some(word));
    let words: Vec<String> = words
        .into_iter()
        .map(|w| values.iter().fold(w, |w, (k, v)| w.replace(k, v)))
        .collect();
    assert!(
        words.iter().all(|w| !w.contains('<')),
        "unfilled: {words:?}"
    );
    assert_eq!(words[0], "specforge", "{line}");
    words[1..].to_vec()
}

/// The UAT loop as the assets hold it, on `v1.0/01` with no session: the template's fences create the session and two
/// tests, and the present step's lines print them. The person's pass line records `U1`; on `U2` the person's issue
/// line records the result and the answer step's gap fence records `G1`. The counts step lists `U2` alone as open;
/// diagnosis reads `G1` back, the debug spawn's line prints it with its test, and the recorded diagnosis leaves the
/// session `diagnosed`.
#[test]
fn the_uat_loop_records_a_pass_and_a_gap_the_diagnosis_reads() {
    let Some(sh) = Shell::only(&[]) else {
        return;
    };
    let p = without_a_session();
    let template = asset("gsd-core/templates/UAT.md");
    let session = typed::filled(
        &fence_holding(&template, "new uat_session"),
        &[("PHASE_ID", "v1.0/01")],
    );
    let done = typed::apply(&sh, &p, &session);
    assert_eq!(done.code, 0, "{}{}", done.out, done.err);
    assert!(done.added.is_empty(), "{:?}", done.added);
    assert!(
        done.out.contains("UAT_SESSION=v1.0/01/01-UAT"),
        "{}",
        done.out
    );
    let sources = p.fields_of("v1.0/01/01-UAT")["source"].clone();
    assert_eq!(sources.as_array().map(Vec::len), Some(2), "{sources}");
    let test = fence_holding(&template, "new uat_test");
    for (id, name, expected) in [
        (
            "U1",
            "Add a task",
            "`node src/cli.js add x` prints `added 1: x`",
        ),
        (
            "U2",
            "Complete a task that does not exist",
            "`node src/cli.js done 7` fails and names the id 7",
        ),
    ] {
        let filled = typed::filled(
            &test,
            &[
                ("UAT_SESSION", "v1.0/01/01-UAT"),
                ("TEST_ID", id),
                ("NAME", name),
                ("EXPECTED", expected),
            ],
        );
        let done = typed::apply(&sh, &p, &filled);
        assert_eq!(done.code, 0, "{}{}", done.out, done.err);
        assert!(done.added.is_empty(), "{:?}", done.added);
    }
    let work = asset("gsd-core/workflows/verify-work.md");
    let present = fence_holding(&work, "--kinds uat_test --format context");
    let (out, err, code) = sh.run_in(p.root(), &format!("PHASE_ID=v1.0/01\n{present}"));
    assert_eq!(code, 0, "{err}");
    assert!(
        out.contains("v1.0/01/01-UAT/U1") && out.contains("v1.0/01/01-UAT/U2"),
        "{out}"
    );
    let pass = person_line(
        &work,
        "specforge gsd set-uat-result <uat_test> pass",
        &[("<uat_test>", "v1.0/01/01-UAT/U1"), ("<who>", "Sam")],
    );
    let pass: Vec<&str> = pass.iter().map(String::as_str).collect();
    let run = p.run_as_person(&pass);
    assert_eq!(run.code, 0, "{}", run.all());
    let words = "it prints a stack trace before the message";
    let issue = person_line(
        &work,
        "specforge gsd set-uat-result <uat_test> issue",
        &[
            ("<uat_test>", "v1.0/01/01-UAT/U2"),
            ("<who>", "Sam"),
            ("<their words>", words),
            ("<severity>", "cosmetic"),
        ],
    );
    let issue: Vec<&str> = issue.iter().map(String::as_str).collect();
    let run = p.run_as_person(&issue);
    assert_eq!(run.code, 0, "{}", run.all());
    let gap = typed::filled(
        &fence_holding(&work, "new uat_gap"),
        &[
            ("TEST", "v1.0/01/01-UAT/U2"),
            ("GAP_ID", "G1"),
            ("SEVERITY", "cosmetic"),
            (
                "TRUTH",
                "Completing an unknown id fails with a one-line message",
            ),
            ("REASON", &format!("User reported: {words}")),
        ],
    );
    let done = typed::apply(&sh, &p, &gap);
    assert_eq!(done.code, 0, "{}{}", done.out, done.err);
    assert!(done.added.is_empty(), "{:?}", done.added);
    assert_eq!(p.fields_of("v1.0/01/01-UAT/U1")["result"], "pass");
    assert_eq!(p.fields_of("v1.0/01/01-UAT/U2")["reported"], words);
    let counts = fence_holding(&work, "OPEN_TESTS=");
    let (out, err, code) = sh.run_in(p.root(), &format!("PHASE_ID=v1.0/01\n{counts}"));
    assert_eq!(code, 0, "{err}");
    assert!(out.contains("open tests: v1.0/01/01-UAT/U2"), "{out}");
    assert!(!out.contains("v1.0/01/01-UAT/U1"), "{out}");
    assert!(
        out.contains(r#""issues":1"#) && out.contains(r#""passed":1"#),
        "{out}"
    );
    let open = fence_holding(&work, "fields.status=open");
    let (out, err, code) = sh.run_in(p.root(), &format!("PHASE_ID=v1.0/01\n{open}"));
    assert_eq!(code, 0, "{err}");
    assert_eq!(out.trim(), "v1.0/01/01-UAT/U2/G1");
    let diagnose = asset("gsd-core/workflows/diagnose-issues.md");
    let read = fence_holding(&diagnose, "--kinds uat_test,uat_gap");
    let (out, err, code) = sh.run_in(p.root(), &format!("PHASE_ID=v1.0/01\n{read}"));
    assert_eq!(code, 0, "{err}");
    assert!(
        out.contains("v1.0/01/01-UAT/U2/G1") && out.contains(words),
        "{out}"
    );
    let spawn = diagnose
        .lines()
        .find(|l| l.contains("specforge query {GAP}"))
        .expect("the debug spawn's reading");
    let start = spawn.find("specforge query").expect("the line");
    let end = spawn[start..].find("\\n").expect("its end") + start;
    let line = spawn[start..end].replace("{GAP}", "v1.0/01/01-UAT/U2/G1");
    let (out, err, code) = sh.run_in(p.root(), &line);
    assert_eq!(code, 0, "{err}");
    assert!(
        out.contains("User reported: ") && out.contains("names the id 7"),
        "{out}"
    );
    let diagnosis = typed::filled(
        &fence_holding(&diagnose, "root_cause"),
        &[
            ("GAP", "v1.0/01/01-UAT/U2/G1"),
            (
                "ROOT_CAUSE",
                "done() rethrows the lookup error before printing",
            ),
            (
                "ARTIFACTS",
                r#"[{"path": "src/cli.js", "issue": "the catch rethrows"}]"#,
            ),
            ("MISSING", "Print the message without the stack"),
        ],
    );
    let done = typed::apply(&sh, &p, &diagnosis);
    assert_eq!(done.code, 0, "{}{}", done.out, done.err);
    assert_eq!(p.fields_of("v1.0/01/01-UAT")["status"], "diagnosed");
}

/// The verifier records each gap Step 9b keeps as a `verify_gap` of the phase's verification: on a taskboard copy, a
/// failed verdict on `01-02`'s TR2, recorded with the verification template's fence, then Step 10's gap fence, filled;
/// the planner's gap read finds the gap open with its reason, and the verification reads `gaps_found`. The fence takes
/// the verification's next gap number itself: run again, it records `VG2`.
#[test]
fn the_verifier_records_each_gap_as_a_verify_gap() {
    let Some(sh) = Shell::only(&[]) else {
        return;
    };
    let p = taskboard();
    let truth = "v1.0/01/01-02/TR2";
    let inputs = p.run(&[
        "query",
        truth,
        "--inputs",
        "gsd/E501",
        "--cite-file",
        "src/cli.js",
        "--format",
        "json",
    ]);
    let inputs: serde_json::Value = serde_json::from_str(&inputs.stdout).expect("the inputs");
    let verdict = typed::filled(
        &fence_holding(
            &asset("gsd-core/templates/verification-report.md"),
            "record-verdict",
        ),
        &[
            ("TRUTH", truth),
            ("OUTCOME", "failed"),
            ("BASIS", &inputs["basis"].to_string()),
            ("CITED", r#"[{"file": "src/cli.js"}]"#),
            (
                "FINDINGS",
                r#"[{"severity": "error", "message": "src/cli.js rethrows the lookup error"}]"#,
            ),
        ],
    );
    let done = typed::apply(&sh, &p, &verdict);
    assert_eq!(done.code, 0, "{}{}", done.out, done.err);
    let reason = "done 7 prints a stack trace before the message";
    let gap = typed::filled(
        &fence_holding(&asset("agents/gsd-verifier.md"), "new verify_gap"),
        &[
            ("VERIFICATION", "v1.0/01/01-VERIFICATION"),
            ("TRUTH", truth),
            ("REASON", reason),
            (
                "ARTIFACTS",
                r#"[{"path": "src/cli.js", "issue": "the catch rethrows"}]"#,
            ),
            (
                "MISSING",
                "Catch the lookup error in done\nPrint only its message",
            ),
        ],
    );
    let done = typed::apply(&sh, &p, &gap);
    assert_eq!(done.code, 0, "{}{}", done.out, done.err);
    assert!(done.added.is_empty(), "{:?}", done.added);
    let fields = p.fields_of("v1.0/01/01-VERIFICATION/VG1");
    assert_eq!(fields["truth"], truth);
    assert_eq!(fields["status"], "open");
    assert_eq!(fields["artifacts"][0]["path"], "src/cli.js");
    assert_eq!(
        fields["missing"],
        serde_json::json!(["Catch the lookup error in done", "Print only its message"])
    );
    assert_eq!(
        p.fields_of("v1.0/01/01-VERIFICATION")["status"],
        "gaps_found"
    );
    let read = fence_holding(
        &asset("gsd-core/references/planner-gap-closure.md"),
        "--kinds verify_gap,uat_gap",
    );
    let (out, err, code) = sh.run_in(p.root(), &format!("PHASE_ID=v1.0/01\n{read}"));
    assert_eq!(code, 0, "{err}");
    assert!(
        out.contains("v1.0/01/01-VERIFICATION/VG1") && out.contains(reason),
        "{out}"
    );
    // the fence numbers the next gap after the verification's last
    let again = typed::apply(&sh, &p, &gap);
    assert_eq!(again.code, 0, "{}{}", again.out, again.err);
    assert_eq!(p.fields_of("v1.0/01/01-VERIFICATION/VG2")["truth"], truth);
}

/// The verifier records each check only a person can make (Step 8) as a `human_item` of the phase's verification: on a
/// taskboard copy, Step 8's fence, filled, creates `HI1` with its test, expectation and reason, and a second run
/// takes `HI2`.
#[test]
fn the_verifier_records_each_human_check_as_a_human_item() {
    let Some(sh) = Shell::only(&[]) else {
        return;
    };
    let p = taskboard();
    let fence = fence_holding(&asset("agents/gsd-verifier.md"), "new human_item");
    for (id, check) in [
        ("HI1", "Run `node src/cli.js list` in a narrow terminal"),
        ("HI2", "Read the error after `node src/cli.js done 99`"),
    ] {
        let filled = typed::filled(
            &fence,
            &[
                ("VERIFICATION", "v1.0/01/01-VERIFICATION"),
                ("CHECK", check),
                ("EXPECTED", "the line wraps without breaking a word"),
                ("WHY_HUMAN", "how it reads is a judgement no test makes"),
            ],
        );
        let done = typed::apply(&sh, &p, &filled);
        assert_eq!(done.code, 0, "{}{}", done.out, done.err);
        let fields = p.fields_of(&format!("v1.0/01/01-VERIFICATION/{id}"));
        assert_eq!(fields["test"], check, "{fields}");
        assert_eq!(
            fields["why_human"],
            "how it reads is a judgement no test makes"
        );
    }
}

/// A phase whose only failure is deferred has no gap left to close: on a taskboard copy, a failed verdict on `01-02`'s
/// TR2, then Step 10's deferral fence, filled to defer it to `v1.0/02`, takes the truth out of the score and leaves
/// the verification off `gaps_found`.
#[test]
fn a_phase_whose_only_failure_is_deferred_has_no_gaps() {
    let Some(sh) = Shell::only(&[]) else {
        return;
    };
    let p = taskboard();
    let truth = "v1.0/01/01-02/TR2";
    let inputs = p.run(&[
        "query",
        truth,
        "--inputs",
        "gsd/E501",
        "--cite-file",
        "src/cli.js",
        "--format",
        "json",
    ]);
    let inputs: serde_json::Value = serde_json::from_str(&inputs.stdout).expect("the inputs");
    let verdict = typed::filled(
        &fence_holding(
            &asset("gsd-core/templates/verification-report.md"),
            "record-verdict",
        ),
        &[
            ("TRUTH", truth),
            ("OUTCOME", "failed"),
            ("BASIS", &inputs["basis"].to_string()),
            ("CITED", r#"[{"file": "src/cli.js"}]"#),
            (
                "FINDINGS",
                r#"[{"severity": "error", "message": "src/cli.js rethrows the lookup error"}]"#,
            ),
        ],
    );
    let done = typed::apply(&sh, &p, &verdict);
    assert_eq!(done.code, 0, "{}{}", done.out, done.err);
    let verification = "v1.0/01/01-VERIFICATION";
    let failed = p.fields_of(verification);
    assert_eq!(failed["status"], "gaps_found", "{failed}");
    let defer = typed::filled(
        &fence_holding(&asset("agents/gsd-verifier.md"), "deferred \"$DEFERRED\""),
        &[
            ("VERIFICATION", verification),
            (
                "DEFERRED",
                &serde_json::json!([{
                    "truth": truth,
                    "addressed_in": "v1.0/02",
                    "evidence": "Phase 2's goal: errors print one line"
                }])
                .to_string(),
            ),
        ],
    );
    let done = typed::apply(&sh, &p, &defer);
    assert_eq!(done.code, 0, "{}{}", done.out, done.err);
    let fields = p.fields_of(verification);
    assert_ne!(fields["status"], "gaps_found", "{fields}");
    let total = |f: &serde_json::Value| -> u32 {
        f["score"]
            .as_str()
            .and_then(|s| s.split('/').nth(1)?.parse().ok())
            .unwrap_or_else(|| panic!("a score: {f}"))
    };
    assert_eq!(total(&fields), total(&failed) - 1, "{failed} -> {fields}");
}
