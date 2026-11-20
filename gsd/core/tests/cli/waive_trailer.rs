//! A person waives gsd/E521 on a recorded commit at a terminal (ADR 0381), and the commit keeps what gsd-git
//! recorded: the waiver is one block set beside it, and the sibling commit with the same fault keeps its error.

use super::commands_support::{gsd_as_person, strict};
use super::harness::Project;
use super::profile_gates::{committed, e521_errors};

/// The `execute` project with two commits of plan `01-01` whose subjects' title found task `T1`, a `feat` and a `fix`,
/// both recorded by gsd-git without a `Gsd-Task:` trailer: each is gsd/E521 under gsd-strict.
pub(super) fn title_linked() -> Project {
    committed(&["feat(01-01): Store module", "fix(01-01): Store module"])
}

/// The id of the one recorded commit whose `type` is `kind`.
pub(super) fn commit_of(p: &Project, kind: &str) -> String {
    let found: Vec<String> = p
        .nodes_of("commit")
        .iter()
        .filter(|n| n["fields"]["type"] == kind)
        .map(|n| n["id"].as_str().expect("an id").to_string())
        .collect();
    let [id] = &found[..] else {
        panic!("one '{kind}' commit, found {found:?}");
    };
    id.clone()
}

#[test]
fn a_waiver_clears_one_commits_e521_and_its_sibling_keeps_its_own() {
    let p = title_linked();
    strict(&p);
    let (feat, fix) = (commit_of(&p, "feat"), commit_of(&p, "fix"));
    let (code, errors) = e521_errors(&p);
    assert_eq!((code, errors.len()), (1, 2), "{errors:?}");

    let run = gsd_as_person(
        &p,
        "waive-trailer",
        &[&feat, "recorded before the trailer existed", "Grace Hopper"],
    );
    assert_eq!(run.code, 0, "{}", run.all());
    // gsd-git collects again and leaves the waiver where the person set it.
    let run = p.collect(&["--runner", "gsd-git"]);
    assert_eq!(run.code, 0, "{}", run.all());

    let (code, errors) = e521_errors(&p);
    assert_eq!((code, errors.len()), (1, 1), "{errors:?}");
    let message = errors[0]["message"].as_str().unwrap_or_default();
    assert!(message.contains(&fix), "{message}");
    let waiver = &p.fields_of(&feat)["waiver"];
    assert_eq!(
        waiver["reason"], "recorded before the trailer existed",
        "{waiver}"
    );
    assert_eq!(waiver["by"], "Grace Hopper", "{waiver}");
    assert!(waiver["at"].is_string(), "{waiver}");
    let codes = p.codes();
    assert!(
        !codes.iter().any(|c| c == "E083" || c == "E084"),
        "{codes:?}"
    );
    // The host recorded the value once: one ledger line, the commit's waiver and its digest.
    let ledger = p.read("specforge-harness");
    let lines: Vec<&str> = ledger
        .lines()
        .filter(|l| l.starts_with(&format!("{feat} waiver ")))
        .collect();
    assert_eq!(lines.len(), 1, "{ledger}");
    assert_eq!(
        lines[0].split(' ').nth(2).map(str::len),
        Some(64),
        "{ledger}"
    );
}
