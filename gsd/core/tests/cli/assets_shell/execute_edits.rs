//! The execute family's shell lines, run as the assets hold them on a taskboard copy under bash with only `specforge`
//! on the `PATH`.

use super::{Shell, asset, block, span, typed};
use crate::harness::taskboard;

/// The `<required_reading>` block of execute-phase's executor spawn: the spawn's prompt sits in a `text` fence, so the
/// block is read from the dispatch marker on, not as an HTML block.
fn executor_reading(text: &str) -> String {
    let at = text
        .find("[gsd:dispatch phase=")
        .expect("the executor spawn's dispatch marker");
    block(&text[at..], "<required_reading>", "</required_reading>")
}

/// The executor spawn's reading opens with the executor route's line, which serves the plan's pack on the taskboard
/// as written: its body names the plan's tasks, no section serves files the plan modifies, and no line of the block
/// names a path variable or the context window.
#[test]
fn the_executor_spawn_reads_the_executor_route() {
    let Some(sh) = Shell::only(&[]) else {
        return;
    };
    let reading = executor_reading(&asset("gsd-core/workflows/execute-phase.md"));
    let first = reading
        .lines()
        .map(str::trim)
        .find(|l| !l.is_empty() && *l != "<required_reading>")
        .expect("a first line");
    assert_eq!(first, r#"specforge context gsd/executor "{plan_id}""#);
    for gone in [
        "{phase_dir}",
        "{plan_file}",
        "${STATE_PATH}",
        "CONTEXT_WINDOW",
    ] {
        assert!(!reading.contains(gone), "{gone}:\n{reading}");
    }
    let p = taskboard();
    let line = first.replace("{plan_id}", "v1.0/02/02-01");
    let (out, err, code) = sh.run_in(p.root(), &format!("{line} --format json"));
    assert_eq!(code, 0, "{out}{err}");
    let pack: serde_json::Value = serde_json::from_str(&out).expect("the pack is JSON");
    let sections = pack["sections"].as_array().expect("sections");
    let body = sections
        .iter()
        .find(|s| s["name"] == "body")
        .expect("a body section");
    assert!(
        body["members"]
            .as_array()
            .is_some_and(|m| m.iter().any(|id| id == "v1.0/02/02-01/T1")),
        "{body}"
    );
    assert!(
        sections
            .iter()
            .all(|s| !s["name"].as_str().unwrap_or("").contains("files")),
        "{sections:?}"
    );
}

/// The summary template's fence, with a decision the executor took first, closes a plan as a model fills it: the
/// summary, its deliverable and its deviation exist, `complete-plan` wrote `completed`, and the check reports nothing
/// new.
#[test]
fn the_summary_template_closes_a_plan() {
    let Some(sh) = Shell::only(&[]) else {
        return;
    };
    let p = taskboard();
    p.write("src/store.js", "export function saveBoard() {}\n");
    p.write(
        "test/store.test.js",
        "test(\"a save then a load\", () => {});\n",
    );
    let started = p.run(&["gsd", "start-plan", "v1.0/02/02-01"]);
    assert_eq!(started.code, 0, "{}", started.all());
    let decision = typed::filled(
        &typed::typed_fence(&asset("agents/gsd-executor.md"), "**Decisions first:**"),
        &[
            ("PLAN_ID", "v1.0/02/02-01"),
            ("DECISION_ID", "D-04"),
            ("DECISION", "The store syncs the file before the rename"),
        ],
    );
    let summary = typed::filled(
        &typed::typed_fence(
            &asset("gsd-core/templates/summary.md"),
            "## The Summary's Typed Edits",
        ),
        &[
            ("PLAN_ID", "v1.0/02/02-01"),
            ("TITLE", "Save and load tasks.json: summary"),
            ("STATUS", "complete"),
            (
                "ONE_LINER",
                "Tasks survive a restart: saved after every change, loaded on start",
            ),
            ("SUBSYSTEM", "store"),
            ("TAG", "node"),
            (
                "REQUIRES",
                r#"[{"phase": "v1.0/01", "provides": "the board"}]"#,
            ),
            ("PROVIDES", "loadBoard and saveBoard"),
            ("AFFECTS", "listing"),
            ("TECH_ADDED", "node:fs"),
            ("TECH_PATTERN", "write to a temporary name, then rename"),
            ("KEY_DECISIONS", "D-04"),
            ("PATTERN", "one store module owns the file"),
            ("REQUIREMENTS_COMPLETED", "STORE-01,STORE-02"),
            (
                "THREAT_FLAG",
                "threat_flag: file | src/store.js | writes tasks.json",
            ),
            ("NEXT_READINESS", "listing can read the saved board"),
            ("DELIVERABLE", "A saved board loads back with its tasks"),
            ("REQUIREMENT", "STORE-01"),
            ("HUMAN_JUDGMENT", "false"),
            ("RATIONALE", "the store tests prove it"),
            (
                "CHECKS",
                r#"[{"kind": "unit", "ref": "test/store.test.js#a save then a load"}]"#,
            ),
            ("DEVIATION_KIND", "deviation"),
            ("RULE", "Rule 1 - Bug"),
            (
                "DEVIATION_TEXT",
                "A missing file threw; it now loads an empty board, and a test covers it",
            ),
            ("DEVIATION_FILE", "src/store.js"),
        ],
    );
    let first = typed::apply(&sh, &p, &decision);
    assert_eq!(first.code, 0, "{}{}", first.out, first.err);
    let closed = typed::apply(&sh, &p, &summary);
    assert_eq!(closed.code, 0, "{}{}", closed.out, closed.err);
    assert!(
        first.added.is_empty() && closed.added.is_empty(),
        "{:?} {:?}",
        first.added,
        closed.added
    );
    let fields = p.fields_of("v1.0/02/02-01/02-01-SUMMARY");
    assert!(fields["completed"].is_string(), "{fields}");
    assert_eq!(fields["key_decisions"], serde_json::json!(["v1.0/02/D-04"]));
    for id in ["D1", "DV1"] {
        let id = format!("v1.0/02/02-01/02-01-SUMMARY/{id}");
        assert!(p.fields_of(&id).is_object(), "{id} is missing");
    }
}

/// A tripped TDD gate, in the executor's halt protocol and in the MVP+TDD reference alike, is an open blocker whose
/// `gate` names the task, and the check reports nothing new; once the gate is met, the clearing fence resolves it, and
/// a second run resolves nothing.
#[test]
fn a_gate_trip_raises_a_blocker_on_its_task() {
    let Some(sh) = Shell::only(&[]) else {
        return;
    };
    for (path, heading, clears) in [
        (
            "agents/gsd-executor.md",
            "3. Record the trip as an open blocker",
            "5. The gate clears",
        ),
        (
            "gsd-core/references/execute-mvp-tdd.md",
            "4. Record the trip as an open blocker",
            "5. The gate clears",
        ),
    ] {
        let p = taskboard();
        let trip = typed::filled(
            &typed::typed_fence(&asset(path), heading),
            &[
                (
                    "HALT_REASON",
                    "NO_FAILING_TEST: T1 adds behavior with no failing test",
                ),
                ("PLAN_ID", "v1.0/02/02-01"),
                ("TASK_ID", "v1.0/02/02-01/T1"),
            ],
        );
        let run = typed::apply(&sh, &p, &trip);
        assert_eq!(run.code, 0, "{path}: {}{}", run.out, run.err);
        assert!(run.added.is_empty(), "{path}: {:?}", run.added);
        let blockers = p.nodes_of("blocker");
        assert_eq!(blockers.len(), 1, "{path}: {blockers:?}");
        let fields = &blockers[0]["fields"];
        assert_eq!(fields["gate"], "v1.0/02/02-01/T1", "{path}");
        assert!(fields["resolved"].is_null(), "{path}: the blocker is open");
        let clear = typed::filled(
            &typed::typed_fence(&asset(path), clears),
            &[
                ("TASK_ID", "v1.0/02/02-01/T1"),
                ("REASON", "the failing test was committed first"),
            ],
        );
        let run = typed::apply(&sh, &p, &clear);
        assert_eq!(run.code, 0, "{path}: {}{}", run.out, run.err);
        assert!(run.added.is_empty(), "{path}: {:?}", run.added);
        let blockers = p.nodes_of("blocker");
        assert_eq!(blockers.len(), 1, "{path}: {blockers:?}");
        assert!(
            blockers[0]["fields"]["resolved"].is_object(),
            "{path}: the blocker is resolved: {blockers:?}"
        );
        // a second run finds no open gate blocker and resolves nothing
        let again = typed::apply(&sh, &p, &clear);
        assert_eq!(again.code, 0, "{path}: {}{}", again.out, again.err);
    }
}

/// The user-setup template's fence, in both forms, writes the plan's setup step as a model fills it: the step exists,
/// its `done` is unset (a person's to confirm), and the check reports nothing new.
#[test]
fn the_user_setup_template_writes_a_setup_step() {
    let Some(sh) = Shell::only(&[]) else {
        return;
    };
    for path in [
        "gsd-core/templates/user-setup.md",
        "gsd-core/templates/user-setup.compact.md",
    ] {
        let p = taskboard();
        let setup = typed::filled(
            &typed::typed_fence(&asset(path), "## The User Setup's Typed Edits"),
            &[
                ("PLAN_ID", "v1.0/02/02-01"),
                ("SETUP_ID", "US1"),
                ("SERVICE", "stripe"),
                ("WHY", "Payment processing requires API keys"),
                (
                    "ENV_VARS",
                    r#"[{"name": "STRIPE_SECRET_KEY", "source": "Stripe Dashboard → Developers → API keys"}]"#,
                ),
                (
                    "DASHBOARD",
                    r#"[{"task": "Create webhook endpoint", "location": "Stripe Dashboard → Webhooks", "details": "URL: https://example.test/hook"}]"#,
                ),
                (
                    "LOCAL_DEV",
                    "stripe listen --forward-to localhost:3000/hook",
                ),
            ],
        );
        let run = typed::apply(&sh, &p, &setup);
        assert_eq!(run.code, 0, "{path}: {}{}", run.out, run.err);
        assert!(run.added.is_empty(), "{path}: {:?}", run.added);
        let fields = p.fields_of("v1.0/02/02-01/US1");
        assert_eq!(fields["service"], "stripe", "{path}: {fields}");
        assert_eq!(fields["env_vars"][0]["name"], "STRIPE_SECRET_KEY", "{path}");
        assert!(fields["done"].is_null(), "{path}: a person confirms it");
    }
}

/// execute-phase's human-verification step records its items as a UAT session of the phase with one test per item,
/// as a model fills it: both exist, the test has no result (the person's), and the check reports nothing new.
#[test]
fn the_uat_step_writes_a_uat_session() {
    let Some(sh) = Shell::only(&[]) else {
        return;
    };
    let p = taskboard();
    let uat = typed::filled(
        &typed::typed_fence(
            &asset("gsd-core/workflows/execute-phase.md"),
            "**Step A: Persist human verification items",
        ),
        &[
            ("PHASE_ID", "v1.0/02"),
            ("{the phase number}", "02"),
            ("TITLE", "Phase 02: human verification"),
            ("SOURCE", "v1.0/01/01-01/01-01-SUMMARY"),
            ("STARTED", "2026-11-13T10:00:00Z"),
            ("TEST_TITLE", "The usage line reads well"),
            (
                "EXPECTED",
                "`node src/cli.js` prints one usage line a person understands",
            ),
        ],
    );
    let run = typed::apply(&sh, &p, &uat);
    assert_eq!(run.code, 0, "{}{}", run.out, run.err);
    assert!(run.added.is_empty(), "{:?}", run.added);
    let session = p.fields_of("v1.0/02/02-UAT");
    assert_eq!(session["phase"], "v1.0/02", "{session}");
    let test = p.fields_of("v1.0/02/02-UAT/U1");
    assert_eq!(test["session"], "v1.0/02/02-UAT", "{test}");
    assert!(
        test["result"].is_null(),
        "the result is the person's: {test}"
    );
}

/// The manager's background prompts tell a stuck agent to raise a blocker on its phase; each backticked line runs as
/// written once the agent fills the text, and the milestone and phase number progress-report prints, and leaves one
/// open blocker.
#[test]
fn the_manager_blocker_line_raises_a_blocker() {
    let Some(sh) = Shell::only(&[]) else {
        return;
    };
    let text = asset("gsd-core/workflows/manager.md");
    let lines: Vec<String> = text
        .split('`')
        .filter(|span| span.starts_with("specforge gsd raise-blocker"))
        .map(|span| span.replace("\\\"", "\""))
        .collect();
    assert_eq!(lines.len(), 2, "{lines:?}");
    for line in lines {
        let p = taskboard();
        let filled = line
            .replace("<the blocker>", "the plan needs a schema decision")
            .replace("<the error>", "permission denied for npm test")
            .replace("{milestone_version}", "v1.0")
            .replace("{N}", "02");
        let run = typed::apply(&sh, &p, &filled);
        assert_eq!(run.code, 0, "{filled}: {}{}", run.out, run.err);
        assert!(run.added.is_empty(), "{filled}: {:?}", run.added);
        let blockers = p.nodes_of("blocker");
        assert_eq!(blockers.len(), 1, "{filled}: {blockers:?}");
        assert_eq!(blockers[0]["fields"]["phase"], "v1.0/02", "{filled}");
        assert!(blockers[0]["fields"]["resolved"].is_null(), "{filled}");
    }
}

/// A confirmed setup step moves the executor pack `start-plan` recorded, so gsd/W524 fires on the running plan; the
/// continuation fence of execute-phase's checkpoint handling records it again, as the orchestrator fills it, and the
/// check reports nothing new.
#[test]
fn the_continuation_re_records_the_executor_pack() {
    let Some(sh) = Shell::only(&[]) else {
        return;
    };
    let p = taskboard();
    let setup = typed::filled(
        &typed::typed_fence(
            &asset("gsd-core/templates/user-setup.md"),
            "## The User Setup's Typed Edits",
        ),
        &[
            ("PLAN_ID", "v1.0/02/02-01"),
            ("SETUP_ID", "US1"),
            ("SERVICE", "stripe"),
            ("WHY", "Payment processing requires API keys"),
            (
                "ENV_VARS",
                r#"[{"name": "STRIPE_SECRET_KEY", "source": "Stripe Dashboard → Developers → API keys"}]"#,
            ),
            ("DASHBOARD", "[]"),
            ("LOCAL_DEV", "none"),
        ],
    );
    let run = typed::apply(&sh, &p, &setup);
    assert_eq!(run.code, 0, "{}{}", run.out, run.err);
    let start = asset("gsd-core/workflows/execute-plan.md")
        .lines()
        .map(str::trim)
        .find(|l| l.starts_with("specforge gsd start-plan \"$PLAN_ID\""))
        .expect("execute-plan's start-plan line")
        .to_string();
    let run = typed::apply(&sh, &p, &format!("PLAN_ID=v1.0/02/02-01\n{start}"));
    assert_eq!(run.code, 0, "{}{}", run.out, run.err);
    let confirmed = p.run_as_person(&["gsd", "confirm-setup", "v1.0/02/02-01/US1", "--by", "ada"]);
    assert_eq!(confirmed.code, 0, "{}", confirmed.all());
    assert!(p.codes().iter().any(|c| c == "gsd/W524"), "{:?}", p.codes());
    let continuation = typed::filled(
        &typed::typed_fence(
            &asset("gsd-core/workflows/execute-phase.md"),
            "**Before every continuation spawn, re-record the executor pack.**",
        ),
        &[("PLAN_ID", "v1.0/02/02-01")],
    );
    let run = typed::apply(&sh, &p, &continuation);
    assert_eq!(run.code, 0, "{}{}", run.out, run.err);
    assert!(
        !p.codes().iter().any(|c| c == "gsd/W524"),
        "{:?}",
        p.codes()
    );
    assert!(run.added.is_empty(), "{:?}", run.added);
}

/// The executor's artifact self-check, as the model fills it, prints the id of an artifact whose file is missing.
#[test]
fn the_self_check_prints_a_missing_artifact() {
    let Some(sh) = Shell::only(&[]) else {
        return;
    };
    let p = taskboard();
    // the collector reads the project's git history
    p.commit();
    let check = typed::filled(
        &typed::typed_fence(
            &asset("agents/gsd-executor.md"),
            "**2. Check the artifacts exist:**",
        ),
        &[("PLAN_ID", "v1.0/02/02-01")],
    );
    let run = typed::apply(&sh, &p, &check);
    assert_eq!(run.code, 0, "{}{}", run.out, run.err);
    assert!(run.out.contains("v1.0/02/02-01/AR1"), "{}", run.out);
}

/// The reads the execute family names inline run as written: the executor's scope pick lists the plan's files,
/// execute-plan's setup read serves a setup step, and execute-phase's report reads serve the phase's verification.
#[test]
fn the_execute_reads_run_as_written() {
    let Some(sh) = Shell::only(&[]) else {
        return;
    };
    let p = taskboard();
    let scope = span(
        &asset("agents/gsd-executor.md"),
        "specforge query \"$PLAN_ID\" --depth 0 --pick '.nodes[0].fields.files_modified[]'",
    );
    let (out, err, code) = sh.run_in(p.root(), &format!("PLAN_ID=v1.0/02/02-01\n{scope}"));
    assert_eq!(code, 0, "{out}{err}");
    assert!(out.lines().any(|l| l == "src/store.js"), "{out}");

    let setup = typed::filled(
        &typed::typed_fence(
            &asset("gsd-core/templates/user-setup.md"),
            "## The User Setup's Typed Edits",
        ),
        &[
            ("PLAN_ID", "v1.0/02/02-01"),
            ("SETUP_ID", "US1"),
            ("SERVICE", "stripe"),
            ("WHY", "Payment processing requires API keys"),
            ("ENV_VARS", "[]"),
            ("DASHBOARD", "[]"),
            ("LOCAL_DEV", "none"),
        ],
    );
    assert_eq!(typed::apply(&sh, &p, &setup).code, 0);
    let read = typed::typed_fence(
        &asset("gsd-core/workflows/execute-plan.md"),
        "<step name=\"generate_user_setup\">",
    );
    let (out, err, code) = sh.run_in(p.root(), &format!("PLAN_ID=v1.0/02/02-01\n{read}"));
    assert_eq!(code, 0, "{out}{err}");
    assert!(out.contains("v1.0/02/02-01/US1"), "{out}");

    let phase = asset("gsd-core/workflows/execute-phase.md");
    for prefix in [
        "specforge query \"$PHASE_ID\" --kinds verification --format context",
        "specforge query \"$PHASE_ID\" --depth 2 --kinds verification,verify_gap --format context",
    ] {
        let line = span(&phase, prefix);
        let (out, err, code) = sh.run_in(p.root(), &format!("PHASE_ID=v1.0/01\n{line}"));
        assert_eq!(code, 0, "{line}: {out}{err}");
        assert!(out.contains("\"kind\":\"verification\""), "{line}: {out}");
    }
}

/// An out-of-scope discovery recorded as `executor-examples.md` shows is a deferred idea of the plan's phase, which
/// the forensic audit's Check 7 lists open until its `status` reads resolved: Check 7 once queried a kind no extension
/// declares, and a log file no command reads.
#[test]
fn an_out_of_scope_discovery_stays_open_in_the_forensic_audit_until_resolved() {
    let Some(sh) = Shell::only(&[]) else {
        return;
    };
    let p = taskboard();
    let record = typed::filled(
        &typed::typed_fence(
            &asset("gsd-core/references/executor-examples.md"),
            "### Recording an out-of-scope discovery",
        ),
        &[
            ("PLAN_ID", "v1.0/02/02-01"),
            ("IDEA_ID", "DI9"),
            (
                "FOUND",
                "The retry budget is hardcoded at 3 in src/store.js",
            ),
            ("WHY", "Pre-existing: no task of this plan touches retries"),
        ],
    );
    let applied = typed::apply(&sh, &p, &record);
    assert_eq!(applied.code, 0, "{}{}", applied.out, applied.err);
    // the phase's context gained the idea: the research written against it reports the drift
    assert_eq!(applied.added, ["gsd/W406"], "new codes");
    let check = format!(
        "MILESTONE=v1.0\n{}",
        typed::typed_fence(
            &asset("gsd-core/workflows/progress/steps/forensic-audit.md"),
            "**Check 7"
        )
    );
    let (out, err, code) = sh.run_in(p.root(), &check);
    assert_eq!(code, 0, "{out}{err}");
    let row = out
        .lines()
        .find(|l| l.contains("DI9"))
        .unwrap_or_else(|| panic!("Check 7 lists no DI9: {out}"));
    assert!(
        row.contains("open") && row.contains("hardcoded at 3"),
        "{row}"
    );
    let id = row.split('\x1f').next().unwrap_or(row).trim().to_string();
    let resolved = p.run(&["set", &id, "status", "resolved"]);
    assert_eq!(resolved.code, 0, "{}", resolved.all());
    let (out, err, code) = sh.run_in(p.root(), &check);
    assert_eq!(code, 0, "{out}{err}");
    assert!(
        out.lines()
            .any(|l| l.contains("DI9") && l.contains("resolved")),
        "{out}"
    );
}
