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
        for (_, code) in gsd_kit::markdown::inline_code(&served(p, file)) {
            if code.contains("specforge query \"${quick_id}\" --kinds") {
                found.push(code);
            }
        }
    }
    found
}

/// `code` run under bash in the project for `quick_id` (and a task description, and `env`), with `specforge` on the
/// `PATH`.
fn bash(p: &Project, code: &str, quick_id: &str, env: &[(&str, &str)]) -> std::process::Output {
    let bin = std::path::PathBuf::from(std::env::var_os("SPECFORGE_BIN").expect("SPECFORGE_BIN"));
    let path = format!(
        "{}:{}",
        bin.parent().expect("a directory").display(),
        std::env::var("PATH").unwrap_or_default()
    );
    std::process::Command::new("bash")
        .args(["-c", code])
        .env("PATH", path)
        .env("quick_id", quick_id)
        .env("DESCRIPTION", "Trim the titles")
        .env("description", "Trim the titles")
        .envs(env.iter().copied())
        .current_dir(p.root())
        .stdin(std::process::Stdio::null())
        .output()
        .expect("bash")
}

/// Whether `check` succeeds for `quick_id`.
fn holds(p: &Project, check: &str, quick_id: &str) -> bool {
    bash(p, check, quick_id, &[]).status.success()
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

/// The commands of `text` that contain `needle`, in line order: a code span, or a fence's code line (trimmed), or
/// a code span of the Markdown a fence's code line builds (a prompt template's string).
fn commands(text: &str, needle: &str) -> Vec<String> {
    use gsd_kit::markdown::{fences, inline_code};
    let mut found: Vec<(usize, String)> = inline_code(text)
        .into_iter()
        .filter(|(_, span)| span.contains(needle))
        .collect();
    for fence in fences(text) {
        for (n, line) in fence.text.lines().enumerate() {
            if line.contains(needle) {
                let span = inline_code(line)
                    .into_iter()
                    .find(|(_, s)| s.contains(needle));
                let command = span.map_or_else(|| line.trim().to_string(), |(_, s)| s);
                found.push((fence.code.start + n, command));
            }
        }
    }
    found.sort_by_key(|(n, _)| *n);
    found.into_iter().map(|(_, command)| command).collect()
}

/// The plan exists before any step that names it (the fence that mints its id wrote it), so a research step writes no
/// plan, every research it writes goes to spec/quick.spec, and the planner is told to fill the plan, not make it.
#[test]
fn a_research_step_finds_the_plan_the_init_step_wrote() {
    let p = project("quick-context");
    for file in [
        "quick/steps/research-phase.md",
        "quick-batch/steps/research-phase.md",
    ] {
        let text = served(&p, file);
        assert!(
            commands(&text, "specforge new plan \"").is_empty(),
            "{file} writes a plan"
        );
        for line in text
            .lines()
            .filter(|l| l.contains("specforge new research"))
        {
            assert!(line.contains("--in spec/quick.spec"), "{file}: {line}");
        }
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

/// With `--discuss` and `--research`, quick records the research's context pack once both exist, as
/// `/gsd-plan-phase` does for a phase's: the served command, run for the fixture's quick plan, records the pack of
/// `gsd/quick-context` on its research.
#[test]
fn quick_records_its_researchs_context_pack() {
    let p = project("quick-context");
    let text = served(&p, "quick.md");
    let record = commands(&text, "specforge gsd record-research");
    assert_eq!(record.len(), 1, "{record:#?}");
    assert!(holds(&p, &record[0], "251101-abc"), "{}", record[0]);
    let pack = p.run(&["context", "gsd/quick-context", "251101-abc"]);
    assert_eq!(pack.code, 0, "{}", pack.all());
    let pack: serde_json::Value = serde_json::from_str(&pack.stdout).expect("a pack");
    let recorded = &p.fields_of("251101-abc-RESEARCH")["context_pack"];
    assert_eq!(recorded["route"], "gsd/quick-context", "{recorded}");
    assert_eq!(recorded["digest"], pack["digest"], "{recorded}");
}

/// The reading-list queries the quick prompts build from `quick_id` (`'... ' + quick_id + '...'`), as served: each
/// hands the sub-agent what it names, and the quick executor's route line serves the context, its decisions and the
/// research. The context's reaches the plan's decisions (they name the context's plan, not
/// the context, so depth 1 would miss them) and the research's its findings.
#[test]
fn a_quick_reading_list_query_reaches_the_context_decisions_and_findings() {
    let p = project("quick-context");
    let mut ran = 0;
    for file in [
        "quick.md",
        "quick/steps/research-phase.md",
        "quick-batch/steps/planner-wave.md",
    ] {
        for line in commands(&served(&p, file), "specforge query ' + quick_id + '") {
            let query = line.replace("' + quick_id + '", "251101-abc");
            let out = bash(&p, &query, "251101-abc", &[]);
            let text = String::from_utf8_lossy(&out.stdout);
            assert!(out.status.success(), "{file}: {query}");
            let wants: &[&str] = if query.contains("-CONTEXT ") {
                &["251101-abc-CONTEXT", "251101-abc-D-01"]
            } else {
                &["251101-abc-RESEARCH", "251101-abc-RESEARCH/F1"]
            };
            for want in wants {
                assert!(
                    text.contains(want),
                    "{file}: `{query}` misses {want}:\n{text}"
                );
            }
            ran += 1;
        }
    }
    assert_eq!(ran, 4, "the reading-list queries");
    // the quick executor reads the executor route, whose quick section serves the plan's context and research
    let routes = commands(&served(&p, "quick.md"), "specforge context gsd/executor");
    assert_eq!(routes.len(), 1, "{routes:?}");
    let out = bash(&p, &routes[0], "251101-abc", &[]);
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success(), "{}: {text}", routes[0]);
    for want in [
        "251101-abc-CONTEXT",
        "251101-abc-D-01",
        "251101-abc-RESEARCH",
        "251101-abc-RESEARCH/F1",
    ] {
        assert!(
            text.contains(want),
            "`{}` misses {want}:\n{text}",
            routes[0]
        );
    }
}

/// A batch item's executor writes its summary on the working tree (`@gsd/core` runs no worktree isolation): the crash-
/// window guard and the post-dispatch check read it there, and find nothing when the item has none.
#[test]
fn a_batch_items_summary_is_looked_for_on_the_working_tree() {
    let (without, with) = (project("quick-context"), project("quick-context"));
    let summary = "\nsummary 251101-abc-SUMMARY \"Done\" {\n  plan                   251101-abc\n  status                 complete\n  one_liner              \"Titles are trimmed\"\n  subsystem              \"tasks\"\n  requirements_completed []\n}\n";
    with.write(QUICK, &format!("{}{summary}", with.read(QUICK)));
    let file = "quick-batch/steps/worktree-dispatch.md";
    let checks = commands(&served(&without, file), "${quick_id}-SUMMARY\" --depth 0");
    // the crash-window guard and the post-dispatch check
    assert_eq!(checks.len(), 2, "{checks:?}");
    for check in &checks {
        let found = bash(&with, check, "251101-abc", &[]);
        assert!(found.status.success(), "the summary is missed: {check}");
        let none = bash(&without, check, "251101-abc", &[]);
        assert!(
            !none.status.success(),
            "a summary is found where none is: {check}"
        );
    }
}
