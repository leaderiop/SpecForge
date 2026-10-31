//! What the tests of the writing commands share (D-r41-20): running `specforge gsd <command>`, the presets, a project's
//! bytes before and after, and the harness ledger line a written value must leave.

use std::collections::BTreeMap;
use std::path::Path;

use sha2::{Digest, Sha256};

use crate::harness::{Project, Run, taskboard};

/// `specforge gsd <command> <args>`.
pub fn gsd(p: &Project, command: &str, args: &[&str]) -> Run {
    let mut all = vec!["gsd", command];
    all.extend_from_slice(args);
    p.run(&all)
}

/// `specforge gsd <command> <args>` as a person at a terminal runs it: the six commands that record what a person says
/// (ADR 0301) are refused to anyone else.
pub fn gsd_as_person(p: &Project, command: &str, args: &[&str]) -> Run {
    let mut all = vec!["gsd", command];
    all.extend_from_slice(args);
    p.run_as_person(&all)
}

/// [`gsd_as_person`] for a command that records what a person says, [`gsd`] for the rest.
pub fn gsd_for(p: &Project, command: &str, args: &[&str]) -> Run {
    const HUMAN: [&str; 6] = [
        "answer-checkpoint",
        "confirm-setup",
        "set-uat-result",
        "approve-validation",
        "waive-window",
        "dispose-refactor",
    ];
    if HUMAN.contains(&command) {
        gsd_as_person(p, command, args)
    } else {
        gsd(p, command, args)
    }
}

/// Every `.spec` file of the project and its harness ledger, by path: the bytes a refused command must leave alone.
pub fn bytes(p: &Project) -> BTreeMap<String, String> {
    fn walk(root: &Path, dir: &Path, out: &mut BTreeMap<String, String>) {
        for entry in std::fs::read_dir(dir).expect("read dir").flatten() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().into_owned();
            if path.is_dir() {
                if !matches!(name.as_str(), ".git" | ".specforge" | "node_modules") {
                    walk(root, &path, out);
                }
            } else if name.ends_with(".spec") || name == "specforge-harness" {
                let relative = path.strip_prefix(root).expect("under root");
                out.insert(
                    relative.display().to_string(),
                    std::fs::read_to_string(&path).expect("read"),
                );
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(p.root(), p.root(), &mut out);
    out
}

/// The ledger line `specforge-harness` holds for `value_json` (compact JSON) of `field` of `entity`.
pub fn ledger_line(entity: &str, field: &str, value_json: &str) -> String {
    let digest = Sha256::digest(format!("{entity}\0{field}\0{value_json}").as_bytes());
    format!("{entity} {field} {digest:x}")
}

/// Whether the ledger holds the line of this value.
pub fn ledgered(p: &Project, entity: &str, field: &str, value_json: &str) -> bool {
    p.read("specforge-harness")
        .lines()
        .any(|l| l == ledger_line(entity, field, value_json))
}

/// A project under the `gsd-strict` profile (W318, W404 and the rest of GSD's warnings are errors).
pub fn strict(p: &Project) {
    p.settings(serde_json::json!({ "profile": "gsd-strict" }));
}

/// A project under `gsd-plus` (strict, and E516 and the traceability gates).
pub fn plus(p: &Project) {
    p.settings(serde_json::json!({ "profile": "gsd-plus" }));
}

/// Appends `text` to a file of the project.
pub fn plant(p: &Project, file: &str, text: &str) {
    let now = if p.exists(file) {
        p.read(file)
    } else {
        String::new()
    };
    p.write(file, &format!("{now}\n{text}"));
}

/// The refusal a run printed must name `code` and every word of `words`, and exit 1. The human form of an
/// `invalid_input` carries only its message (`error: ...`), so that code is not looked for in it.
pub fn refused(run: &Run, code: &str, words: &[&str]) {
    assert_eq!(run.code, 1, "{}", run.all());
    let all = run.all();
    assert!(
        code == "invalid_input" || all.contains(code),
        "{code} not in: {all}"
    );
    for word in words {
        assert!(all.contains(word), "'{word}' not in: {all}");
    }
}

/// The taskboard twin.
pub fn twin() -> Project {
    taskboard()
}

/// The JSON a command printed under `--format json`.
pub fn json(run: &Run) -> serde_json::Value {
    serde_json::from_str(&run.stdout)
        .unwrap_or_else(|e| panic!("the command prints JSON ({e}): {}", run.all()))
}

/// Removes the ledger lines of `field` of `entity`, as if the value had never been recorded.
pub fn unledger(p: &Project, entity: &str, field: &str) {
    let prefix = format!("{entity} {field} ");
    let kept: Vec<String> = p
        .read("specforge-harness")
        .lines()
        .filter(|l| !l.starts_with(&prefix))
        .map(String::from)
        .collect();
    p.write("specforge-harness", &format!("{}\n", kept.join("\n")));
}

/// The project's files are as `before` ([`bytes`]) saw them: the failure names the files that differ.
pub fn unchanged(p: &Project, before: &BTreeMap<String, String>) {
    let after = bytes(p);
    let names: Vec<&String> = before
        .keys()
        .chain(after.keys())
        .filter(|k| before.get(*k) != after.get(*k))
        .collect();
    assert!(names.is_empty(), "these files differ: {names:?}");
}
