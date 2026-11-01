//! T3: the multi-line scripts, and the commit readers end to end.

use std::path::Path;
use std::process::{Command, Stdio};

use super::{Shell, asset, block, both, line, q, upstream};

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

/// Review of plan 03, item 1: the TDD gate (execute-phase.md, `TDD_MODE`) runs from `RED_IDS=$(` through its
/// `TDD GATE TRIPPED` exit. A test-file commit outside the plan, or a plan commit that touches no test file, is no RED
/// commit; a commit of the plan that touches a test file is. Re-review item 3: the plan named by its short id (`01-01`)
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

/// A repository at `dir` with one committed file, then changed (`dirty`) or not, or holding an untracked file only.
fn repository(dir: &Path, state: &str) {
    std::fs::create_dir_all(dir).expect("a dir");
    git(dir, &["init", "-q"]);
    if state == "untracked" {
        std::fs::write(dir.join("new"), "x").expect("a file");
        return;
    }
    std::fs::write(dir.join("f"), "x").expect("a file");
    git(dir, &["add", "f"]);
    git(dir, &["commit", "-qm", "f"]);
    if state == "dirty" {
        std::fs::write(dir.join("f"), "y").expect("a change");
    }
}

/// T3: pr-branch's sub-repository scan and its path check, bash where upstream's were node scripts (G4 plan 05 T4:
/// no jq either). With `planning.sub_repos` empty here they are not reached; they still keep upstream's refusals.
#[test]
fn the_sub_repo_scan_keeps_its_refusals() {
    let Some(sh) = Shell::without_jq() else {
        return;
    };
    let top = tempfile::tempdir().expect("a dir");
    let root = top.path().join("root");
    repository(&root.join("a"), "dirty");
    repository(&root.join("b"), "clean");
    repository(&root.join("c"), "untracked");
    repository(&top.path().join("outside"), "dirty");
    std::os::unix::fs::symlink(top.path().join("outside"), root.join("l")).expect("a link");
    let (text, up) = (
        asset("gsd-core/workflows/pr-branch.md"),
        upstream("gsd-core/workflows/pr-branch.md"),
    );
    let ours = block(
        &text,
        "printf '%s' \"$SUB_REPOS_JSON\" | tr",
        "> \"$DIRTY_FILE\"",
    );
    let theirs = block(&up, "node -e \"", "\"$DIRTY_FILE\"");
    let dirty = top.path().join("dirty");
    let setup = format!(
        "ROOT={}\nDIRTY_FILE={}\nSUB_REPOS_JSON={}",
        q(&root.display().to_string()),
        q(&dirty.display().to_string()),
        q(r#"["a","b","c","l","../outside","a\nb",5,"d;rm","missing"]"#)
    );
    both(
        &sh,
        &setup,
        &ours,
        Some(&theirs),
        "cat \"$DIRTY_FILE\"",
        "a",
    );
    both(
        &sh,
        &format!("{setup}\nSUB_REPOS_JSON=x"),
        &ours,
        Some(&theirs),
        "cat \"$DIRTY_FILE\"",
        "",
    );

    let ours = block(&text, "SUB_REPO_DIR=$( (", ") 2>/dev/null)");
    let theirs = block(&up, "SUB_REPO_DIR=$(node", "2>/dev/null)");
    let real = std::fs::canonicalize(root.join("a")).expect("a real path");
    for (rel, want) in [
        ("a", real.display().to_string()),
        ("../outside", String::new()),
        ("l", String::new()),
        ("missing", String::new()),
    ] {
        let setup = format!(
            "ROOT={}\nREPO_REL={}",
            q(&root.display().to_string()),
            q(rel)
        );
        both(
            &sh,
            &setup,
            &ours,
            Some(&theirs),
            "printf '%s' \"$SUB_REPO_DIR\"",
            &want,
        );
    }
}

/// Re-review of plan 05, item 2: pr-branch's filter (`FILTER_PATHS`, what each cherry-picked commit loses, and
/// `FORBIDDEN_RE`, what `verify` asserts is gone) is the spec files for both layouts: under `spec/`, and, with the spec
/// root at the project root (`.`), the `.spec` files at any depth, never the whole tree. Run in a scratch repository: a
/// commit that touches code, spec files and the harness ledger keeps only its code.
#[test]
fn the_pr_filter_keeps_the_code_whatever_the_spec_root() {
    let Some(sh) = Shell::without_jq() else {
        return;
    };
    let text = asset("gsd-core/workflows/pr-branch.md");
    let declare = block(&text, "TRANSIENT_DIRS=", "MILESTONE_PHASES_RE=");
    let derive = block(&text, "Derive the mode's two projections", "FORBIDDEN_RE=");
    let (_, derive) = derive.split_once("```bash\n").expect("the derive block");
    let filter = block(&text, "while IFS= read -r P; do", "done <<FILTER_PATHS_EOF");
    for (root, spec_files) in [
        (".", ["a.spec", "sub/b.spec"]),
        ("spec", ["spec/a.spec", "spec/notes.md"]),
    ] {
        let dir = tempfile::tempdir().expect("a dir");
        git(dir.path(), &["init", "-q"]);
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
        let script = format!(
            "PR_STRICT=true\n{}\n{derive}\nfi\n\
             printf 'a.spec\\nsub/b.spec\\nspec/a.spec\\nsrc/x.js\\nspecforge-harness\\nREADME.md\\n' | grep -E \"$FORBIDDEN_RE\" | tr '\\n' ' '; echo\n\
             git cherry-pick --no-commit feature >/dev/null 2>&1 || true\n{filter}\n$FILTER_PATHS\nFILTER_PATHS_EOF\n\
             git diff --cached --name-only | tr '\\n' ' '",
            declare.replace("{{project:spec_root}}", root)
        );
        let (out, err, code) = sh.run_in(dir.path(), &script);
        let forbidden = if root == "." {
            "a.spec sub/b.spec spec/a.spec specforge-harness "
        } else {
            "spec/a.spec specforge-harness "
        };
        assert_eq!(
            out,
            format!("{forbidden}\nsrc/x.js "),
            "spec root {root} ({code}): {err}"
        );
    }
}
