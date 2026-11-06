//! T3: the multi-line scripts, and the commit readers end to end.

use std::path::Path;
use std::process::{Command, Stdio};

use super::{Shell, asset, block, line};

/// T3, end to end: the commit lines' producer (`specforge query … --kinds commit`) and reader, in a project whose plan
/// 01-01 has a commit, with `specforge` on a `PATH` that holds no node and no jq. It confirms the field paths the lines
/// pick.
#[test]
fn the_commit_readers_read_the_commit_entities() {
    let Some(sh) = Shell::without_jq() else {
        return;
    };
    let p = crate::harness::project("execute");
    p.commit();
    let hash = p.git_commit(
        "feat(01-01): Store module",
        &[("src/a.js", "x\n")],
        "2026-10-29T09:00:00Z",
    );
    let run = |script: &str| sh.run_in(p.root(), script);

    let text = asset("gsd-core/workflows/execute-phase.md");
    let produce = line(&text, "PLAN_ROWS=$(specforge").replace("{plan_padded}", "01");
    let read = line(&text, "PLAN_COMMITS=$(").replace("{plan_padded}", "01");
    let (out, err, _) = run(&format!(
        "PHASE_NUMBER=01\n{produce}\n{read}\nprintf '%s' \"$PLAN_COMMITS\""
    ));
    assert_eq!(
        out,
        format!("{} feat(01-01): Store module", &hash[..10]),
        "{err}"
    );

    let text = asset("gsd-core/workflows/execute-phase/steps/completion-reconciliation.md");
    let produce = line(&text, "COMMITS_SCOPE=$(").replace("{phase_number}-{plan_padded}", "01-01");
    let read = line(&text, "if [ \"$COMMITS_SCOPE_RC\" -eq 0 ]");
    let (out, err, _) = run(&format!(
        "{produce}\nCOMMITS_FOUND=\"\"\n{read}\nprintf '%s' \"$COMMITS_FOUND\""
    ));
    assert_eq!(out, hash, "{err}");
}

/// The TDD gate (execute-phase.md, `TDD_MODE`) runs from `RED_IDS=$(` through its
/// `TDD GATE TRIPPED` exit. A test-file commit outside the plan, or a plan commit that touches no test file, is no RED
/// commit; a commit of the plan that touches a test file is. The plan named by its short id (`01-01`)
/// finds the same commits as by its full one, which a commit's `plan` field holds.
#[test]
fn the_tdd_gate_finds_the_red_commit_among_the_plans_own_commits() {
    let Some(sh) = Shell::without_jq() else {
        return;
    };
    let text = asset("gsd-core/workflows/execute-phase.md");
    let gate = |plan: &str| {
        format!(
            "PLAN_ID={plan}\nTASK_ID=T1\n{}\n      exit 1\n    fi\nprintf '%s' \"$RED_COMMIT\"",
            block(&text, "RED_IDS=$(", "TDD GATE TRIPPED")
        )
    };
    let p = crate::harness::project("execute");
    p.commit();
    p.git_commit(
        "feat(01-01): Store module",
        &[("src/a.js", "x\n")],
        "2026-10-29T09:00:00Z",
    );
    // a test file, but another plan's commit, then one no plan names
    p.git_commit(
        "test(01-02): failing test",
        &[("test/done.test.js", "t\n")],
        "2026-10-29T09:10:00Z",
    );
    p.git_commit(
        "chore: tidy the tests",
        &[("tests/x.rs", "t\n")],
        "2026-10-29T09:20:00Z",
    );
    for plan in ["v1.0/01/01-01", "01-01"] {
        let (out, err, code) = sh.run_in(p.root(), &gate(plan));
        assert_eq!(
            code, 1,
            "{plan}: no RED commit of 01-01, the gate trips\n{out}\n{err}"
        );
        assert!(out.contains("TDD GATE TRIPPED"), "{plan}: {out}\n{err}");
    }

    let red = p.git_commit(
        "test(01-01): failing store test",
        &[("test/tasks.test.js", "t\n")],
        "2026-10-29T09:30:00Z",
    );
    for plan in ["v1.0/01/01-01", "01-01"] {
        let (out, err, code) = sh.run_in(p.root(), &gate(plan));
        assert_eq!(code, 0, "{plan}: {out}\n{err}");
        assert_eq!(
            out,
            format!("{} test(01-01): failing store test", &red[..10]),
            "{plan}: {err}"
        );
    }
}

/// Runs `git args` in `dir` with no user configuration.
fn git(dir: &Path, args: &[&str]) {
    let out = Command::new("git")
        .args(["-c", "user.name=gsd", "-c", "user.email=gsd@example.com"])
        .args([
            "-c",
            "commit.gpgsign=false",
            "-c",
            "core.hooksPath=/dev/null",
        ])
        .args(args)
        .current_dir(dir)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .stdin(Stdio::null())
        .output()
        .expect("git runs");
    assert!(
        out.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// Pr-branch's filter (`FILTER_PATHS`, what each cherry-picked commit loses, and the diff's `spec` flag, what `verify`
/// asserts is gone) is the spec files for both layouts: under `spec/`, and, with the spec root at the project root (`.`),
/// the `.spec` files at any depth, never the whole tree. Run in a scratch repository: a commit that touches code, spec
/// files and the harness ledger keeps only its code, and `verify` sees no spec file on the branch it builds.
#[test]
fn the_pr_filter_keeps_the_code_whatever_the_spec_root() {
    let Some(sh) = Shell::without_jq() else {
        return;
    };
    let text = asset("gsd-core/workflows/pr-branch.md");
    let declare = block(&text, "TRANSIENT_DIRS=", "else SPEC_PATHSPEC=");
    let derive = block(
        &text,
        "Derive the mode's projection",
        "FILTER_PATHS=$(printf",
    );
    let (_, derive) = derive.split_once("```bash\n").expect("the derive block");
    let filter = block(&text, "while IFS= read -r P; do", "done <<FILTER_PATHS_EOF");
    for (root, spec_files) in [
        (".", ["a.spec", "sub/b.spec"]),
        ("spec", ["spec/a.spec", "spec/notes.md"]),
    ] {
        let dir = tempfile::tempdir().expect("a dir");
        git(dir.path(), &["init", "-q"]);
        let config = serde_json::json!({"name": "d", "version": "0.1.0", "spec_root": root});
        std::fs::write(dir.path().join("specforge.json"), config.to_string()).expect("a project");
        for f in spec_files.iter().chain(&["specforge-harness", "README.md"]) {
            let path = dir.path().join(f);
            std::fs::create_dir_all(path.parent().expect("a parent")).expect("a dir");
            std::fs::write(path, "base\n").expect("a file");
        }
        git(dir.path(), &["add", "-A"]);
        git(dir.path(), &["commit", "-qm", "base"]);
        git(dir.path(), &["checkout", "-qb", "feature"]);
        for f in spec_files.iter().chain(&["specforge-harness", "src/x.js"]) {
            let path = dir.path().join(f);
            std::fs::create_dir_all(path.parent().expect("a parent")).expect("a dir");
            std::fs::write(path, "changed\n").expect("a file");
        }
        git(dir.path(), &["add", "-A"]);
        git(dir.path(), &["commit", "-qm", "feat: x"]);
        git(dir.path(), &["checkout", "-q", "-"]);
        let flagged = |base: &str, head: &str| {
            format!(
                "specforge git diff --base {base} --head {head} --pick '.files[spec=true].path' | tr '\\n' ' '; echo"
            )
        };
        let script = format!(
            "{}\n{derive}\n\
             {}\n\
             git cherry-pick --no-commit feature >/dev/null 2>&1 || true\n{filter}\n$FILTER_PATHS\nFILTER_PATHS_EOF\n\
             git diff --cached --name-only | tr '\\n' ' '; echo\n\
             git commit -qm built; {}",
            declare.replace("{{project:spec_root}}", root),
            flagged("HEAD", "feature"),
            flagged("HEAD~1", "HEAD"),
        );
        let (out, err, code) = sh.run_in(dir.path(), &script);
        let flags = if root == "." {
            "a.spec specforge-harness sub/b.spec "
        } else {
            "spec/a.spec spec/notes.md specforge-harness "
        };
        assert_eq!(
            out,
            format!("{flags}\nsrc/x.js \n\n"),
            "spec root {root} ({code}): {err}"
        );
    }
}

/// The worktree pin, run as written: both roots are read by `specforge git repo`, an alias of the checkout compares
/// equal, a sibling checkout halts as a mismatch and not as a failed read, and a submodule with its own project may
/// still commit under the pin of the checkout that holds it.
#[test]
fn the_pin_compares_both_roots_with_one_reader() {
    let Some(sh) = Shell::without_jq() else {
        return;
    };
    let text = asset("gsd-core/references/worktree-path-safety.md");
    let at = text
        .find("# gsd:guard=supplied-root-pin")
        .expect("the guard");
    let end = at + text[at..].find("\n```").expect("its fence");
    let guard = &text[at..end];
    let run = |cwd: &Path, pin: &str| {
        let script = guard.replacen("{PINNED_ROOT}", pin, 1);
        sh.run_in(cwd, &format!("{script}\necho passed"))
    };
    let base = tempfile::tempdir().expect("a dir");
    let base = base.path().canonicalize().expect("its path");
    let (main, sibling, sub) = (base.join("main"), base.join("sibling"), base.join("sub"));
    let config = "{\"name\":\"demo\",\"version\":\"0.1.0\"}\n";
    for repo in [&main, &sub] {
        std::fs::create_dir_all(repo).expect("a repository");
        std::fs::write(repo.join("specforge.json"), config).expect("its project");
        git(repo, &["init", "-q", "-b", "main"]);
        git(repo, &["add", "-A"]);
        git(repo, &["commit", "-qm", "first"]);
    }
    let held = main.join("sub");
    git(
        &main,
        &[
            "-c",
            "protocol.file.allow=always",
            "submodule",
            "--quiet",
            "add",
            sub.to_str().expect("a path"),
            "sub",
        ],
    );
    git(&main, &["add", "-A"]);
    git(&main, &["commit", "-qm", "with a submodule"]);
    git(
        &main,
        &[
            "worktree",
            "add",
            "-q",
            sibling.to_str().expect("a path"),
            "-b",
            "other",
        ],
    );
    let top = main.to_str().expect("a path");

    for (cwd, pin) in [
        (main.clone(), top.to_string()),
        (main.clone(), format!("{top}/")),
        (held.clone(), top.to_string()),
        (sibling.clone(), sibling.to_str().expect("a path").into()),
    ] {
        let (out, err, code) = run(&cwd, &pin);
        assert_eq!(
            (out.trim(), code),
            ("passed", 0),
            "{cwd:?} under {pin}: {err}"
        );
    }
    #[cfg(unix)]
    {
        let alias = base.join("alias");
        std::os::unix::fs::symlink(&main, &alias).expect("an alias");
        let (out, err, _) = run(&main, alias.to_str().expect("a path"));
        assert_eq!(out.trim(), "passed", "the alias compares equal: {err}");
    }

    let halts = |cwd: &Path, pin: &str, stage: &str| {
        let (out, err, code) = run(cwd, pin);
        assert_eq!((out.trim(), code), ("", 1), "{cwd:?} under {pin}: {err}");
        assert!(err.contains(&format!("Guard stage: {stage}")), "{err}");
        err
    };
    let err = halts(&sibling, top, "root-mismatch");
    assert!(err.contains(&format!("pinned={top}")), "{err}");
    halts(&main, sibling.to_str().expect("a path"), "root-mismatch");
    let elsewhere = tempfile::tempdir().expect("no repository");
    let err = halts(
        &main,
        elsewhere.path().to_str().expect("a path"),
        "pinned-capture",
    );
    assert!(
        err.contains("Diagnostic: specforge git repo from the pinned root failed: error["),
        "{err}"
    );
    let err = halts(elsewhere.path(), top, "actual-capture");
    assert!(
        err.contains("Diagnostic: specforge git repo from the cwd failed: error["),
        "{err}"
    );
    let err = halts(&main, "/nonexistent/pin", "pinned-capture");
    assert!(err.contains("Pinned root: /nonexistent/pin"), "{err}");
    halts(&main, "{PINNED_ROOT}", "pin-unbound");
    halts(&main, "relative/pin", "form-gate");
}

/// Forensic-audit lists the tracked changes outside the spec root, ten at most, whatever else the tree holds.
#[test]
fn forensic_audit_leaves_the_spec_root_out() {
    let Some(sh) = Shell::without_jq() else {
        return;
    };
    let text = asset("gsd-core/workflows/progress/steps/forensic-audit.md");
    let read = line(&text, "specforge git status --except spec");
    let dir = tempfile::tempdir().expect("a dir");
    let paths: Vec<String> = (0..12)
        .map(|n| format!("src/{n:02}.txt"))
        .chain(["spec/a.spec".to_string(), "spectre.txt".to_string()])
        .collect();
    std::fs::create_dir_all(dir.path().join("src")).expect("a dir");
    std::fs::create_dir_all(dir.path().join("spec")).expect("a dir");
    for path in &paths {
        std::fs::write(dir.path().join(path), "one\n").expect("a file");
    }
    git(dir.path(), &["init", "-q", "-b", "main"]);
    git(dir.path(), &["add", "-A"]);
    git(dir.path(), &["commit", "-qm", "first"]);
    std::fs::write(dir.path().join("spec/a.spec"), "two\n").expect("a spec file");
    std::fs::write(dir.path().join("spectre.txt"), "two\n").expect("a file");
    let (out, err, _) = sh.run_in(dir.path(), read);
    assert_eq!(out.trim(), "spectre.txt", "{err}");
    for path in paths.iter().filter(|p| p.starts_with("src/")) {
        std::fs::write(dir.path().join(path), "two\n").expect("a file");
    }
    let (out, err, _) = sh.run_in(dir.path(), read);
    assert_eq!(out.lines().count(), 10, "ten at most: {out} {err}");
    assert!(!out.contains("spec/"), "{out}");
}

/// Ship's `[ci skip]` check asks of the note itself: a marker in its subject or body counts, and one on an older commit
/// does not, though the newest commit that matches is then that one.
#[test]
fn ship_reads_the_ci_skip_marker_on_the_note_itself() {
    let Some(sh) = Shell::without_jq() else {
        return;
    };
    let text = asset("gsd-core/workflows/ship.md");
    let line = line(
        &text,
        "if [ \"$STATUS\" = \"blocked\" ] && [ \"$CHECKS\" = \"0\" ]",
    );
    let dir = tempfile::tempdir().expect("a dir");
    git(dir.path(), &["init", "-q", "-b", "main"]);
    let commit = |args: &[&str]| {
        let mut all = vec!["commit", "-q", "--allow-empty"];
        all.extend_from_slice(args);
        git(dir.path(), &all);
    };
    let skips = |wanted: &str| {
        let script = format!(
            "STATUS=blocked; CHECKS=0; SHIP_NOTE_SHA=$(specforge git head --pick .sha)\n{line}\n  echo skips\nelse\n  echo runs\nfi"
        );
        let (out, err, _) = sh.run_in(dir.path(), &script);
        assert_eq!(out.trim(), wanted, "{err}");
    };
    commit(&["-m", "docs: the note [ci skip]"]);
    skips("skips");
    commit(&["-m", "docs: the note", "-m", "Why.\n\n[ci skip]"]);
    skips("skips");
    commit(&["-m", "docs: no marker here"]);
    skips("runs");
}

/// The executor's commit count, end to end: `start-plan` records the plan's head once, in the graph, so it holds across
/// the fresh shells of one plan and in a worktree of any name; an unborn branch records none, and every commit counts. No
/// tag is written, so `git push --tags` publishes nothing and `git describe` reads nothing.
#[test]
fn the_plan_ledger_counts_from_the_head_the_plan_started_at() {
    let Some(sh) = Shell::without_jq() else {
        return;
    };
    let text = asset("agents/gsd-executor.md");
    let start = line(
        &text,
        "specforge gsd start-plan \"${PHASE}-${PLAN}\" --head",
    );
    let read = line(&text, "PLAN_HEAD_BEFORE=$(specforge query").replace("{phase}-{plan}", "01-01");
    let count = line(&text, "COMMITS_ACTUAL=$(");
    let run = |dir: &Path, script: &str| {
        let (out, err, code) = sh.run_in(dir, &format!("PHASE=01\nPLAN=01\n{script}"));
        assert_eq!(code, 0, "{err}");
        out.trim().to_string()
    };
    let counted = |dir: &Path| run(dir, &format!("{read}\n{count}\necho \"$COMMITS_ACTUAL\""));

    let p = crate::harness::project("execute");
    let at = |p: &crate::harness::Project, field: &str| {
        p.fields_of("v1.0/01/01-01/START")[field].clone()
    };
    p.commit();
    let before = run(p.root(), "specforge git head --pick .sha");
    run(p.root(), start);
    assert_eq!(at(&p, "head"), before.as_str());
    p.git_commit(
        "feat(01-01): a",
        &[("src/a.js", "a\n")],
        "2026-10-29T09:00:00Z",
    );
    p.git_commit(
        "feat(01-01): b",
        &[("src/b.js", "b\n")],
        "2026-10-29T09:01:00Z",
    );
    assert_eq!(counted(p.root()), "2");
    // A second start keeps the first head.
    run(p.root(), start);
    assert_eq!(at(&p, "head"), before.as_str());
    assert_eq!(counted(p.root()), "2");
    assert_eq!(
        std::fs::read_dir(p.root().join(".git/refs/tags")).map_or(0, Iterator::count),
        0,
        "no tag holds a plan's start"
    );

    // A worktree whose name no tag could hold.
    let base = tempfile::tempdir().expect("a dir");
    for (i, name) in ["my wt", "x.lock", "a..b", "wt~1"].iter().enumerate() {
        let other = base.path().join(name);
        let path = other.to_str().expect("a path");
        let branch = format!("retry-{i}");
        git(p.root(), &["worktree", "add", "-q", path, "-b", &branch]);
        // The start was not committed, so the worktree opens the plan at its own head.
        run(&other, start);
        git(
            &other,
            &["commit", "--allow-empty", "-qm", "feat(01-01): c"],
        );
        assert_eq!(counted(&other), "1", "{name}: the base is in the graph");
        // Removed and added again, it reads nothing of the worktree it was.
        git(p.root(), &["worktree", "remove", "--force", path]);
        git(p.root(), &["worktree", "add", "-q", path, &branch]);
        run(&other, start);
        assert_eq!(counted(&other), "0", "{name}: nothing since this start");
    }

    // An unborn branch has no head to record, and a start with one given later keeps it.
    let unborn = crate::harness::project("execute");
    run(unborn.root(), start);
    assert_eq!(at(&unborn, "head"), serde_json::Value::Null);
    unborn.git_commit(
        "feat(01-01): a",
        &[("src/a.js", "a\n")],
        "2026-10-29T09:00:00Z",
    );
    assert_eq!(
        counted(unborn.root()),
        "1",
        "an absent head counts every commit"
    );
    let first = run(unborn.root(), "specforge git head --pick .sha");
    run(unborn.root(), start);
    assert_eq!(at(&unborn, "head"), first.as_str());
}
