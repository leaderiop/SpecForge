//! G4 (plan 05 T3): execute-phase's wave worktree manifest is tab-separated lines (`orchestrator_root<TAB>root`, then
//! `worktree<TAB>path<TAB>branch`), written and read by bash with `git`, `mktemp`, `mv` and `cat` alone on the `PATH`.
//! Every way the list cannot be read blocks the cleanup, as F9's checks did.

use super::{Shell, asset, line, q};

/// The tools the manifest lines may call: bash's builtins do the rest.
const TOOLS: &[&str] = &["git", "mktemp", "mv", "cat"];

fn text() -> String {
    asset("gsd-core/workflows/execute-phase.md")
}

/// The two orchestrator-root reads, which are one line.
fn read_root(text: &str) -> &str {
    let reads: Vec<&str> = text
        .lines()
        .map(str::trim_start)
        .filter(|l| l.starts_with("PRIMARY_WT=$(while"))
        .collect();
    assert_eq!(reads.len(), 2, "the templated and the cleanup-tail reads");
    assert_eq!(reads[0], reads[1]);
    reads[0]
}

#[test]
fn the_manifest_is_written_then_its_root_read_back() {
    let Some(sh) = Shell::only(TOOLS) else { return };
    let tmp = tempfile::tempdir().expect("a dir");
    let text = text();
    let create = line(&text, "M=$(mktemp");
    let write = line(&text, "printf 'orchestrator_root\\t%s\\n'");
    let read = read_root(&text);
    for (root, want) in [("/work/lane 2", "/work/lane 2"), ("", "")] {
        let script = format!(
            "TMPDIR={}\nORCH_ROOT={}\n{create}\n{write}\n{read}\n\
             printf '%s|' \"${{WAVE_WORKTREE_MANIFEST##*.}}\"; cat \"$WAVE_WORKTREE_MANIFEST\"; printf '[%s]' \"$PRIMARY_WT\"; \
             [ ! -e \"$WAVE_WORKTREE_MANIFEST.tmp\" ]",
            q(&tmp.path().display().to_string()),
            q(root)
        );
        let (out, err, code) = sh.run_in(tmp.path(), &script);
        assert_eq!(code, 0, "no temp file is left: {out}{err}");
        assert_eq!(
            out,
            format!("tsv|orchestrator_root\t{root}\n[{want}]"),
            "{err}"
        );
        assert_eq!(err, "", "{root}");
    }
    // an unset or missing manifest gives no root, quietly: the caller falls back to `git worktree list`
    for setup in [
        "WAVE_WORKTREE_MANIFEST=",
        "WAVE_WORKTREE_MANIFEST=/nonexistent",
    ] {
        let (out, err, _) = sh.run_in(
            tmp.path(),
            &format!("{setup}\n{read}\nprintf '[%s]' \"$PRIMARY_WT\""),
        );
        assert_eq!((out.as_str(), err.as_str()), ("[]", ""), "{setup}");
    }
}

#[test]
fn the_worktree_paths_are_listed_and_an_unreadable_manifest_blocks() {
    let Some(sh) = Shell::only(TOOLS) else { return };
    let tmp = tempfile::tempdir().expect("a dir");
    let list = line(&text(), "{ if [ -z \"${WAVE_WORKTREE_MANIFEST:-}\" ]").to_string();
    let at = |name: &str| q(&tmp.path().join(name).display().to_string());
    let run = |manifest: &str| {
        let script = format!(
            "export WAVE_WORKTREE_MANIFEST={manifest}\nWT_PATHS_FILE={}\n( {list}\n); echo \"[$?]\"; cat \"$WT_PATHS_FILE\"",
            at("paths")
        );
        sh.run_in(tmp.path(), &script)
    };
    for (body, want, cause) in [
        (
            "orchestrator_root\t/r\nworktree\t/w/a\tx\nworktree\t\ty\nworktree\t/w/b c\tz",
            "[0]\n/w/a\n/w/b c\n",
            "",
        ),
        ("orchestrator_root\t/r\n", "[0]\n", ""),
        ("", "[1]\n", "manifest is empty"),
        ("\n\n", "[1]\n", "manifest is empty"),
        (
            "orchestrator_root\t/r\norchestrator_root\t/s\n",
            "[1]\n",
            "more than one orchestrator_root",
        ),
        ("{\"worktrees\":[]}\n", "[1]\n", "unknown record"),
    ] {
        std::fs::write(tmp.path().join("m.tsv"), body).expect("a manifest");
        let (out, err, _) = run(&at("m.tsv"));
        assert_eq!(out, want, "{body:?}: {err}");
        assert!(err.contains(cause), "{body:?}: {err}");
        if want.starts_with("[1]") {
            assert!(err.contains("BLOCKED"), "{body:?}: {err}");
        }
    }
    for (manifest, cause) in [("", "is unset"), ("/nonexistent", "does not exist")] {
        let (out, err, _) = run(manifest);
        assert_eq!(out, "[1]\n", "{manifest}: {err}");
        assert!(err.contains(cause) && err.contains("BLOCKED"), "{err}");
    }
}
