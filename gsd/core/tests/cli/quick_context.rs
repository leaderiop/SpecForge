//! A quick plan's own context, decisions and research (`/gsd-quick --discuss --research`), over
//! `tests/projects/quick-context/`: they name the quick plan in `plan`, as a phase's name it in `phase`.

use crate::harness::{Project, project};

const QUICK: &str = "spec/quick.spec";

/// The `(code, message)` of every `gsd/E206` the project reports.
fn e206(p: &Project) -> Vec<String> {
    p.findings()
        .into_iter()
        .filter(|(code, _)| code == "gsd/E206")
        .map(|(_, message)| message)
        .collect()
}

#[test]
fn a_quick_plan_holds_its_context_and_research() {
    project("quick-context").check_exact(&[]);
}

#[test]
fn a_second_context_of_a_quick_plan_is_e206() {
    let p = project("quick-context");
    let quick = p.read(QUICK);
    p.write(
        QUICK,
        &format!(
            "{quick}\ncontext 251101-abc-CONTEXT-B \"Again\" {{\n  plan     251101-abc\n  boundary \"b\"\n  canonical_refs [ {{ topic \"t\" paths [] }} ]\n}}\n"
        ),
    );
    let found = e206(&p);
    assert!(
        found
            .iter()
            .any(|m| m.contains("context '251101-abc-CONTEXT-B' is a second context")),
        "{found:#?}"
    );
}

#[test]
fn a_context_naming_both_a_phase_and_a_plan_is_e206() {
    let p = project("quick-context");
    p.write(
        "spec/v1.0.spec",
        "milestone v1.0 \"One\" {\n  name \"One\"\n}\n\nphase 01 \"Core\" {\n  milestone v1.0\n  name \"Core\"\n  slug \"core\"\n  goal \"g\"\n  success_criteria [\"a\", \"b\"]\n}\n",
    );
    p.edit(
        QUICK,
        "  plan           251101-abc\n  gathered",
        "  phase          v1.0/01\n  plan           251101-abc\n  gathered",
    );
    let found = e206(&p);
    assert!(
        found
            .iter()
            .any(|m| m.contains("context 'v1.0/01/251101-abc-CONTEXT' names both")),
        "{found:#?}"
    );
}

#[test]
fn a_context_of_a_phases_plan_is_e206() {
    let p = project("quick-context");
    p.write(
        "spec/v1.0.spec",
        "milestone v1.0 \"One\" {\n  name \"One\"\n}\n\nphase 01 \"Core\" {\n  milestone v1.0\n  name \"Core\"\n  slug \"core\"\n  goal \"g\"\n  success_criteria [\"a\", \"b\"]\n}\n\nplan 01-01 \"Core plan\" {\n  phase v1.0/01\n  type execute\n  files_modified [\"src/a.js\"]\n  objective \"o\"\n}\n",
    );
    p.edit(
        QUICK,
        "  plan           251101-abc\n  gathered",
        "  plan           v1.0/01/01-01\n  gathered",
    );
    let found = e206(&p);
    assert!(
        found
            .iter()
            .any(|m| m.contains("context '251101-abc-CONTEXT'") && m.contains("not a quick plan")),
        "{found:#?}"
    );
}

/// A quick plan's research may have any name ...
#[test]
fn a_research_of_a_quick_plan_may_have_any_name() {
    let p = project("quick-context");
    p.edit(QUICK, "research 251101-abc-RESEARCH", "research foo");
    p.edit(QUICK, "research   251101-abc-RESEARCH", "research   foo");
    assert_eq!(e206(&p), Vec::<String>::new());
}

/// ... but `RESEARCH`, the project's research's name: the research pass reads that name as the project's.
#[test]
fn a_research_of_a_quick_plan_named_research_is_e206() {
    let p = project("quick-context");
    p.edit(QUICK, "research 251101-abc-RESEARCH", "research RESEARCH");
    p.edit(
        QUICK,
        "research   251101-abc-RESEARCH",
        "research   RESEARCH",
    );
    let found = e206(&p);
    assert!(
        found
            .iter()
            .any(|m| m.contains("research 'RESEARCH' names quick plan '251101-abc'")),
        "{found:#?}"
    );
}

/// The "is the quick plan planned" checks the served quick workflows run: every backticked
/// `specforge query "${quick_id}" --kinds ...` command of quick.md and the batch's planner wave.
fn planned_checks(p: &Project) -> Vec<String> {
    let mut found = Vec::new();
    for file in ["quick.md", "quick-batch/steps/planner-wave.md"] {
        for code in served(p, file).split('`').skip(1).step_by(2) {
            if !code.contains('\n') && code.contains("specforge query \"${quick_id}\" --kinds") {
                found.push(code.to_string());
            }
        }
    }
    found
}

/// Whether `check` succeeds for `quick_id` (and a task description) in the project, run under bash with `specforge` on the `PATH`.
fn holds(p: &Project, check: &str, quick_id: &str) -> bool {
    let bin = std::path::PathBuf::from(std::env::var_os("SPECFORGE_BIN").expect("SPECFORGE_BIN"));
    let path = format!(
        "{}:{}",
        bin.parent().expect("a directory").display(),
        std::env::var("PATH").unwrap_or_default()
    );
    let out = std::process::Command::new("bash")
        .args(["-c", check])
        .env("PATH", path)
        .env("quick_id", quick_id)
        .env("DESCRIPTION", "Trim the titles")
        .env("description", "Trim the titles")
        .current_dir(p.root())
        .stdin(std::process::Stdio::null())
        .output()
        .expect("bash");
    out.status.success()
}

/// A plan the discussion or research step made first (as the prompt's "The quick plan" says) is not planned; once
/// the planner gives it a task, it is. Each check of the served text agrees.
#[test]
fn a_quick_plan_is_planned_once_it_holds_a_task() {
    let p = project("quick-context");
    let stub = p.run(&[
        "new",
        "plan",
        "251101-stb",
        "Stub",
        "--in",
        QUICK,
        "--type",
        "execute",
        "--files_modified",
        "",
        "--objective",
        "Stub",
    ]);
    assert_eq!(stub.code, 0, "{}", stub.all());
    let checks = planned_checks(&p);
    assert_eq!(checks.len(), 3, "{checks:#?}");
    for check in &checks {
        assert!(
            !holds(&p, check, "251101-stb"),
            "a stub plan counts as planned: {check}"
        );
    }
    let task = p.run(&[
        "new",
        "task",
        "T1",
        "Trim",
        "--in",
        QUICK,
        "--plan",
        "251101-stb",
        "--type",
        "auto",
        "--files",
        "src/tasks.js",
        "--action",
        "Trim the title",
        "--verify_text",
        "The title has no spaces at its ends",
        "--done",
        "The title is trimmed",
    ]);
    assert_eq!(task.code, 0, "{}", task.all());
    for check in &checks {
        assert!(
            holds(&p, check, "251101-stb"),
            "a plan with a task is not planned: {check}"
        );
    }
}

/// The served workflow `file`.
fn served(p: &Project, file: &str) -> String {
    let run = p.run(&["gsd", "reference", &format!("gsd-core/workflows/{file}")]);
    assert_eq!(run.code, 0, "{file}: {}", run.all());
    run.stdout
}

/// The commands of `text` that contain `needle`: a backticked span, or a whole line outside backticks (a bash block).
fn commands(text: &str, needle: &str) -> Vec<String> {
    let mut found = Vec::new();
    for line in text.lines().filter(|l| l.contains(needle)) {
        let spans: Vec<&str> = line.split('`').collect();
        match spans.iter().skip(1).step_by(2).find(|s| s.contains(needle)) {
            Some(span) => found.push(span.to_string()),
            None => found.push(line.trim().to_string()),
        }
    }
    found
}

/// A research step makes the quick plan before the research that names it (the orchestrator does, before it
/// spawns the researcher; a second run, as a resumed batch makes, keeps it), every research it writes is written to
/// spec/quick.spec, and the planner is told the plan exists, so it fills it instead of making it again.
#[test]
fn a_research_step_makes_the_quick_plan_before_its_research() {
    let p = project("quick-context");
    for file in [
        "quick/steps/research-phase.md",
        "quick-batch/steps/research-phase.md",
    ] {
        let text = served(&p, file);
        let make = commands(&text, "specforge new plan \"${quick_id}\"");
        assert_eq!(make.len(), 1, "{file} makes the quick plan once: {make:#?}");
        let (Some(at), Some(research)) = (
            text.find(make[0].as_str()),
            text.find("specforge new research"),
        ) else {
            panic!("{file}: no research write");
        };
        assert!(at < research, "{file} makes the plan after its research");
        for line in text
            .lines()
            .filter(|l| l.contains("specforge new research"))
        {
            assert!(line.contains("--in spec/quick.spec"), "{file}: {line}");
        }
        let id = if file.starts_with("quick-batch") {
            "251101-bat"
        } else {
            "251101-one"
        };
        for run in 1..=2 {
            assert!(holds(&p, &make[0], id), "{file}, run {run}: {}", make[0]);
        }
        assert_eq!(p.fields_of(id)["type"], "execute", "{file}");
    }
    for file in ["quick.md", "quick-batch/steps/planner-wave.md"] {
        let text = served(&p, file);
        let line = text
            .lines()
            .find(|l| l.contains("Create a SINGLE plan"))
            .expect("the planner's constraint");
        assert!(
            line.contains("already exists") && line.contains("never `specforge new plan`"),
            "{file}: {line}"
        );
    }
}

/// With `--discuss` and `--research`, quick records the research's digest of the context once both exist, as
/// `/gsd-plan-phase` does for a phase's: the served command, run for the fixture's quick plan, copies the context's
/// digest into its research.
#[test]
fn quick_records_its_researchs_context_digest() {
    let p = project("quick-context");
    let text = served(&p, "quick.md");
    let record = commands(&text, "specforge gsd record-research");
    assert_eq!(record.len(), 1, "{record:#?}");
    assert!(holds(&p, &record[0], "251101-abc"), "{}", record[0]);
    assert_eq!(
        p.fields_of("251101-abc-RESEARCH")["context_digest"],
        p.fields_of("251101-abc-CONTEXT")["digest"]
    );
}
