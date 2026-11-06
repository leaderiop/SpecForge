//! The build and test gates' and the branch rules' blocks, as `specforge gsd reference` serves them or as the agent
//! file holds them, run under bash.

use super::Shell;
use crate::harness::{Project, project};

const POST_MERGE: &str = "gsd-core/workflows/execute-phase/steps/post-merge-gate.md";

/// The reference `path` as `specforge gsd reference` serves it, its settings filled.
fn served(p: &Project, path: &str) -> String {
    let run = p.run(&["gsd", "reference", path]);
    assert_eq!(run.code, 0, "{path}: {}", run.all());
    run.stdout
}

/// The first shell fence of `text` after the first line holding `mark`.
fn fence_after(text: &str, mark: &str) -> String {
    let after = text
        .split_once(mark)
        .unwrap_or_else(|| panic!("no {mark}"))
        .1;
    let body = after
        .split_once("```bash\n")
        .unwrap_or_else(|| panic!("no fence after {mark}"))
        .1;
    body.split_once("\n```")
        .unwrap_or_else(|| panic!("the fence after {mark} ends"))
        .0
        .to_string()
}

/// A file a project holds can name a command: the python build gate spliced the name into the string `bash -c` reads.
#[test]
fn a_python_file_name_never_runs_as_a_command() {
    let Some(sh) = Shell::only(&["git", "touch", "mkdir", "printf", "bash"]) else {
        return;
    };
    let p = project("execute");
    p.write("pyproject.toml", "[project]\nname = \"x\"\n");
    p.write("x;touch pwned;y.py", "print(1)\n");
    let step_a = fence_after(&served(&p, POST_MERGE), "**Step A");
    sh.run_in(p.root(), &step_a);
    assert!(
        p.root().join("pwned").exists(),
        "today the file's name runs as a command"
    );
}
