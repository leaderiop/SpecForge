//! T3: the multi-line scripts, and the commit readers end to end.

use std::path::Path;
use std::process::{Command, Stdio};

use super::{Shell, asset, block, both, has_node, line, q, run_with_node, upstream};

/// T3, end to end: the commit lines' producer (`specforge query … --kinds commit`) and reader, in a project whose plan
/// 01-01 has a commit, with `specforge` on a `PATH` that holds no node. It confirms the field paths the readers name.
#[test]
fn the_commit_readers_read_the_commit_entities() {
    let Some(sh) = Shell::new() else { return };
    std::os::unix::fs::symlink(
        std::env::var_os("SPECFORGE_BIN").expect("SPECFORGE_BIN"),
        sh.bin.path().join("specforge"),
    )
    .expect("a link");
    let p = crate::harness::project("execute");
    p.commit();
    let hash = p.git_commit(
        "feat(01-01): Store module",
        &[("src/a.js", "x\n")],
        "2026-10-29T09:00:00Z",
    );
    let run = |script: &str| sh.run_in(p.root(), script);

    let text = asset("gsd-core/workflows/execute-phase.md");
    let produce = line(&text, "PLAN_COMMITS=$(specforge").replace("{plan_padded}", "01");
    let read = line(&text, "PLAN_COMMITS=$(printf");
    let (out, err, _) = run(&format!(
        "PHASE_NUMBER=01\n{produce}\n{read}\nprintf '%s' \"$PLAN_COMMITS\""
    ));
    assert_eq!(
        out,
        format!("{} feat(01-01): Store module", &hash[..10]),
        "{err}"
    );

    // the TDD gate's producer prints one sha; the reader names it with git (node's JSON.parse of it printed nothing)
    let read = line(&text, "RED_COMMIT=$(git show");
    let (out, err, _) = run(&format!(
        "RED_COMMIT={hash}\n{read}\nprintf '%s' \"$RED_COMMIT\""
    ));
    assert_eq!(
        out,
        format!("{} feat(01-01): Store module", &hash[..10]),
        "{err}"
    );
    let (out, _, _) = run(&format!("RED_COMMIT=\n{read}\nprintf '%s' \"$RED_COMMIT\""));
    assert_eq!(out, "", "no RED commit is no RED commit");
    if has_node() {
        let up = upstream("gsd-core/workflows/execute-phase.md");
        let theirs = line(&up, "RED_COMMIT=$(printf");
        let script = format!("RED_COMMIT={hash}\n{theirs}\nprintf '%s' \"$RED_COMMIT\"");
        assert_eq!(
            run_with_node(&script),
            "",
            "upstream's reader on a plain sha"
        );
    }

    let text = asset("gsd-core/workflows/execute-phase/steps/completion-reconciliation.md");
    let produce = line(&text, "COMMITS_SCOPE=$(").replace("{phase_number}-{plan_padded}", "01-01");
    let read = line(&text, "if [ \"$COMMITS_SCOPE_RC\" -eq 0 ]");
    let (out, err, _) = run(&format!(
        "{produce}\nCOMMITS_FOUND=\"\"\n{read}\nprintf '%s' \"$COMMITS_FOUND\""
    ));
    assert_eq!(out, hash, "{err}");
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

/// T3: pr-branch's sub-repository scan and its path check, bash and jq where upstream's were node scripts. With
/// `planning.sub_repos` empty here they are not reached; they still keep upstream's refusals.
#[test]
fn the_sub_repo_scan_keeps_its_refusals() {
    let Some(sh) = Shell::new() else { return };
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
        "printf '%s' \"$SUB_REPOS_JSON\" | jq",
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

/// T3: review's plan-coverage check (two text files, so no upstream run: its node script loads a runtime library).
#[test]
fn the_plan_coverage_check_uses_jq() {
    let Some(sh) = Shell::new() else { return };
    let dir = tempfile::tempdir().expect("a dir");
    let ours = block(
        &asset("gsd-core/workflows/review.md"),
        "jq -ncj",
        ".plan-coverage-$SLUG.json",
    );
    std::fs::write(
        dir.path().join("manifest"),
        "- 12.6-07\n-   01-02  \r\n- a.b+c\n- x(y)\n-\n- 01-1\nnot an id\n- [z]\n",
    )
    .expect("a manifest");
    std::fs::write(
        dir.path().join("review"),
        "Reviewed 01-02, a.b+c and x(y); 01-10 is not 01-1, and [z]\n12.6-070 is not it\n",
    )
    .expect("a review");
    let setup = format!(
        "RUN_DIR={0}\nSLUG=s\nMANIFEST={0}/manifest\nREVIEW_FILE={0}/review",
        q(&dir.path().display().to_string())
    );
    let show = "cat \"$RUN_DIR/.plan-coverage-$SLUG.json\"";
    both(
        &sh,
        &setup,
        &ours,
        None,
        show,
        r#"{"complete":false,"missing_ids":["12.6-07"],"total":6}"#,
    );
    // an unreadable manifest writes an empty file, as node's exception did
    let (out, _, code) = sh.run_in(
        dir.path(),
        &format!("{setup}\nMANIFEST=/nonexistent\n{ours}\n{show}"),
    );
    assert_eq!((out.as_str(), code != 0), ("", false), "the loop goes on");
}
