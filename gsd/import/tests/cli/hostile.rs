//! Hostile `.planning/` folders through the binary (ADR 0391 D6, D8, D9, D10, D11): what reaches the terminal is
//! escaped, a settings line is shell-quoted, a secret is never printed, an unknown file is named and not echoed, and
//! every refusal (a link out of the project, another spec root, a cap, a path that no longer exists) writes nothing.

use gsd_import::limits::MAX_OPS;
use serde_json::json;

use crate::harness::{Project, Run, taskboard, vault};
use crate::{imported, tree};

const CONFIG: &str = ".planning/config.json";

/// `specforge gsd import` on `p`, asserted refused (exit 1) saying each of `says`, with nothing written.
fn refused(p: &Project, says: &[&str]) -> Run {
    let before = tree(p);
    let run = p.run(&["gsd", "import"]);
    assert_eq!(run.code, 1, "{}", run.all());
    for words in says {
        assert!(run.all().contains(words), "{words:?} not in {}", run.all());
    }
    assert!(tree(p) == before, "a refused import wrote: {}", run.all());
    run
}

/// `specforge gsd import` on `p`, asserted to exit 0.
fn import(p: &Project, args: &[&str]) -> Run {
    let mut all = vec!["gsd", "import"];
    all.extend_from_slice(args);
    let run = p.run(&all);
    assert_eq!(run.code, 0, "{}", run.all());
    run
}

#[test]
fn terminal_controls_are_escaped() {
    let p = imported(taskboard());
    p.edit(
        ".planning/ROADMAP.md",
        "### Phase 1: Core\n",
        "### Phase 1: Core\u{1b}[31m red\u{202e}evil\rinjected\n",
    );
    let run = import(&p, &[]);
    let all = run.all();
    for raw in ['\u{1b}', '\u{202e}', '\r'] {
        assert!(
            !all.contains(raw),
            "a raw {raw:?} reached the terminal: {all:?}"
        );
    }
    assert!(
        run.stdout.contains("Core\\u{1b}[31m red\\u{202e}evil"),
        "the title is shown escaped: {}",
        run.stdout
    );
    assert!(
        !all.lines().any(|line| line.starts_with("injected")),
        "the title made a line of its own: {all}"
    );
    // A .spec file holds no control character: the phase is reported, not written, and the host refuses nothing.
    assert!(
        run.stdout
            .contains("phase v1.0/01: its title holds a control or bidirectional character"),
        "{}",
        run.stdout
    );
}

#[test]
fn settings_lines_are_quoted() {
    let p = imported(taskboard());
    p.edit(
        CONFIG,
        "\"model_profile\": \"balanced\"",
        "\"model_profile\": \"x'; rm -rf ~ #\"",
    );
    p.edit(
        CONFIG,
        "\"granularity\": \"coarse\"",
        "\"granularity\": \"$(id)\"",
    );
    p.edit(CONFIG, "\"mode\": \"yolo\"", "\"mode\": \"yo\\u0007lo\"");
    let run = import(&p, &["--dry-run"]);
    for line in [
        "specforge settings set @gsd/core model_profile 'x'\\''; rm -rf ~ #'",
        "specforge settings set @gsd/core granularity '$(id)'",
    ] {
        assert!(
            run.stdout.lines().any(|l| l.trim() == line),
            "no `{line}`: {}",
            run.stdout
        );
    }
    assert!(
        !run.stdout.contains("settings set @gsd/core mode "),
        "a value with a bell is printed: {}",
        run.stdout
    );
    assert!(
        run.stdout
            .contains("control character in the value of mode"),
        "{}",
        run.stdout
    );
    assert!(!run.all().contains('\u{7}'));
}

#[test]
fn a_secret_is_never_printed() {
    let p = imported(taskboard());
    p.edit(
        CONFIG,
        "\"brave_search\": false",
        "\"brave_search\": \"sk-test-123\"",
    );
    let text = import(&p, &["--dry-run"]);
    let json = import(&p, &["--dry-run", "--format", "json"]);
    for run in [&text, &json] {
        assert!(
            !run.all().contains("sk-test-123"),
            "the secret is printed: {}",
            run.all()
        );
        assert!(run.stdout.contains("BRAVE_API_KEY"), "{}", run.stdout);
    }
    let answer: serde_json::Value = serde_json::from_str(&json.stdout).expect("JSON");
    let entry = answer["output"]["entries"]
        .as_array()
        .expect("entries")
        .iter()
        .find(|e| e["section"] == "brave_search")
        .unwrap_or_else(|| panic!("no brave_search entry: {answer}"));
    assert_eq!(entry["outcome"], "setting");
    assert!(
        entry["reason"]
            .as_str()
            .expect("a reason")
            .contains("BRAVE_API_KEY")
    );
}

#[test]
fn an_unknown_file_is_reported() {
    let p = imported(taskboard());
    p.write(".planning/notes.txt", "NOTE-BODY-XYZ\n");
    let run = import(&p, &["--dry-run"]);
    let named: Vec<&str> = run
        .stdout
        .lines()
        .filter(|l| l.contains("notes.txt"))
        .collect();
    assert_eq!(named.len(), 1, "{}", run.stdout);
    assert!(named[0].contains("unmapped"), "{}", named[0]);
    assert!(!run.all().contains("NOTE-BODY-XYZ"), "{}", run.all());
}

#[cfg(unix)]
#[test]
fn a_symlink_out_of_the_project_is_not_read() {
    let p = imported(taskboard());
    let outside = tempfile::tempdir().expect("a temp dir");
    let target = outside.path().join("01-UAT.md");
    std::fs::write(&target, "OUTSIDE-BODY\n").expect("write the outside file");
    let uat = p.root().join(".planning/phases/01-core/01-UAT.md");
    std::fs::remove_file(&uat).expect("remove the UAT file");
    std::os::unix::fs::symlink(&target, &uat).expect("link out of the project");
    let run = refused(
        &p,
        &[".planning/phases/01-core/01-UAT.md", "outside_project"],
    );
    assert!(!run.all().contains("OUTSIDE-BODY"));
}

#[test]
fn another_spec_root_is_refused() {
    let p = imported(taskboard());
    p.settings_in("specs", json!({}));
    refused(&p, &["'specs'"]);
}

#[test]
fn vault_without_security_reports_needs_extension() {
    let p = imported(vault());
    let run = import(&p, &[]);
    let needs = |file: &str, section: &str| {
        run.stdout
            .lines()
            .any(|l| l.starts_with(file) && l.contains(&format!("  {section}  needs-extension  ")))
    };
    assert!(
        needs(".planning/phases/01-core/01-SECURITY.md", "-"),
        "{}",
        run.stdout
    );
    for plan in ["01-01", "01-02"] {
        assert!(
            needs(
                &format!(".planning/phases/01-core/{plan}-PLAN.md:"),
                "<threat_model>"
            ),
            "{plan}: {}",
            run.stdout
        );
    }
    let count = run
        .stdout
        .lines()
        .find(|l| l.starts_with("import: "))
        .expect("the count line");
    for kind in [
        "threat",
        "trust_boundary",
        "security_audit",
        "accepted_risk",
        "audit_run",
    ] {
        assert!(!count.contains(kind), "{kind}: {count}");
    }
}

#[test]
fn a_duplicate_plan_is_reported() {
    let p = imported(taskboard());
    let plan = p.read(".planning/phases/01-core/01-01-PLAN.md");
    p.write(".planning/phases/01-copy/01-01-PLAN.md", &plan);
    let run = import(&p, &[]);
    // The served files are read in path order: `01-copy` before `01-core`.
    assert!(
        run.stdout.contains(
            "plan v1.0/01/01-01 is given twice (first at .planning/phases/01-copy/01-01-PLAN.md:1): not imported"
        ),
        "{}",
        run.stdout
    );
    assert_eq!(p.nodes_of("plan").len(), 3);
}

#[test]
fn a_plan_past_its_cap_is_refused() {
    let p = imported(taskboard());
    let mut text = "# Requirements\n\n## v1 Requirements\n\n### Core\n".to_string();
    for n in 1..=MAX_OPS + 1 {
        text.push_str(&format!("- [ ] **A-{n:04}**: x\n"));
    }
    p.write(".planning/REQUIREMENTS.md", &text);
    refused(&p, &[&format!("truncated at {MAX_OPS}")]);
}

#[test]
fn a_stale_path_is_refused_by_the_host() {
    let p = imported(taskboard());
    p.edit(
        ".planning/phases/01-core/01-01-PLAN.md",
        "files_modified: [package.json, src/tasks.js, test/tasks.test.js]",
        "files_modified: [package.json, src/gone.js, test/tasks.test.js]",
    );
    refused(&p, &["E080", "src/gone.js"]);
}

/// The vault's `.planning/` claims threat ID T-01-02 in plans 01-01 and 01-02 (the `gsd.security/E101` duplicate its
/// twin keeps by design). An import is judged for host errors only (ADR 0392), so both claims land and `check`
/// reports the two E101.
#[test]
fn the_vault_imports_in_time() {
    let p = imported(vault()).with_extension("@gsd/security", "GSD_SECURITY_WASM");
    let start = std::time::Instant::now();
    let run = p.run(&["gsd", "import"]);
    let elapsed = start.elapsed();
    eprintln!("the vault with @gsd/security imported in {elapsed:?}");
    assert_eq!(run.code, 0, "{}", run.all());
    assert!(elapsed < std::time::Duration::from_secs(30), "{elapsed:?}");
    let e101 = p
        .codes()
        .into_iter()
        .filter(|code| code == "gsd.security/E101")
        .count();
    assert_eq!(e101, 2);
}
