//! A person waives gsd/E521 on a recorded commit at a terminal (ADR 0381), and the commit keeps what gsd-git
//! recorded: the waiver is one block set beside it, and the sibling commit with the same fault keeps its error.

use std::collections::BTreeSet;

use serde_json::Value;

use super::commands_support::{bytes, gsd_as_person, json, refused, strict, unchanged, unledger};
use super::harness::{Project, Run};
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

/// The commit name of the backticked `specforge gsd waive-trailer <id> '<reason>' <who>` span of a gsd/E521
/// suggestion, once the placeholders follow it as the help writes them.
fn waiver_id(suggestion: &str) -> Option<String> {
    let line = suggestion
        .split('`')
        .find(|span| span.starts_with("specforge gsd waive-trailer "))?;
    let mut words = line["specforge gsd waive-trailer ".len()..].split(' ');
    let id = words.next()?;
    (words.collect::<Vec<_>>() == ["'<reason>'", "<who>"]).then(|| id.to_string())
}

/// Each form of gsd/E521's help keeps its trailer advice and names the waiver with its commit's name; that line, run by
/// a person with a reason and a name, clears the commit's error once gsd-git collects again.
#[test]
fn the_e521_help_names_a_waiver_that_clears_it() {
    let p = committed(&[
        "feat(01-01): Store module",
        "feat(01-01): an unplanned change",
    ]);
    strict(&p);
    let (code, errors) = e521_errors(&p);
    assert_eq!((code, errors.len()), (1, 2), "{errors:?}");
    let mut forms = Vec::new();
    for error in &errors {
        let message = error["message"].as_str().expect("a message");
        let suggestion = error["suggestion"].as_str().expect("a suggestion");
        let named = message
            .strip_prefix("commit '")
            .and_then(|rest| rest.split('\'').next())
            .expect("the commit's name in the message");
        // The trailer advice stays: the task's full id, or the plan's full id and a slash before the task.
        let form = if suggestion.contains("'Gsd-Task: v1.0/01/01-01/T1'") {
            "by title"
        } else if suggestion.contains("'Gsd-Task: v1.0/01/01-01/<task>'") {
            "no task"
        } else {
            panic!("no trailer advice: {suggestion}")
        };
        forms.push(form);
        let id = waiver_id(suggestion).unwrap_or_else(|| panic!("no waiver named: {suggestion}"));
        assert_eq!(id, named, "{error}");
        // The person runs the help's own line, never an id the test found another way.
        let run = gsd_as_person(
            &p,
            "waive-trailer",
            &[&id, "recorded before the trailer existed", "Grace Hopper"],
        );
        assert_eq!(run.code, 0, "{form}: {}", run.all());
        let run = p.collect(&["--runner", "gsd-git"]);
        assert_eq!(run.code, 0, "{form}: {}", run.all());
        let (_, left) = e521_errors(&p);
        assert!(
            left.iter()
                .all(|e| !e["message"].as_str().unwrap_or_default().contains(&id)),
            "{form}: {left:?}"
        );
    }
    forms.sort_unstable();
    assert_eq!(forms, ["by title", "no task"]);
    assert_eq!(e521_errors(&p), (0, Vec::new()));
}

/// The ledger lines of `id`'s recorded values, one `<id> <field> <digest>` per value, in the ledger's order.
fn ledger_of(p: &Project, id: &str) -> Vec<String> {
    p.read("specforge-harness")
        .lines()
        .filter(|l| l.starts_with(&format!("{id} ")))
        .map(str::to_string)
        .collect()
}

/// `fields` without the commit's `waiver`.
fn without_waiver(mut fields: Value) -> Value {
    fields.as_object_mut().expect("fields").remove("waiver");
    fields
}

/// A ledger line holds the digest of the exact value, so equal lines prove byte-identical recorded values.
#[test]
fn a_waived_commit_keeps_every_field_gsd_git_recorded() {
    let p = title_linked();
    strict(&p);
    let id = commit_of(&p, "feat");
    let fields = p.fields_of(&id);
    assert!(fields.get("waiver").is_none(), "{fields}");
    let lines = ledger_of(&p, &id);
    assert!(lines.len() > 1, "one line per recorded field: {lines:?}");
    let before = bytes(&p);

    let run = gsd_as_person(
        &p,
        "waive-trailer",
        &[&id, "recorded before the trailer existed", "Grace Hopper"],
    );
    assert_eq!(run.code, 0, "{}", run.all());
    // The waiver wrote the plan file that holds the commit and the ledger, nothing else.
    let after_waiver = bytes(&p);
    let moved: Vec<&String> = after_waiver
        .keys()
        .chain(before.keys())
        .filter(|k| before.get(*k) != after_waiver.get(*k))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    let [file, ledger] = &moved[..] else {
        panic!("two files moved: {moved:?}");
    };
    assert_eq!(ledger.as_str(), "specforge-harness", "{moved:?}");
    assert!(
        file.ends_with(".spec") && after_waiver[*file].contains(&id),
        "{file}"
    );
    let waived = ledger_of(&p, &id);
    let (added, kept): (Vec<String>, Vec<String>) = waived
        .iter()
        .cloned()
        .partition(|l| l.starts_with(&format!("{id} waiver ")));
    assert_eq!((added.len(), &kept), (1, &lines), "{waived:?}");
    let waived_fields = p.fields_of(&id);

    // A collect with nothing new proves gsd-git skips the commit it holds: no plan, not a byte written.
    let run = p.collect(&["--runner", "gsd-git"]);
    assert_eq!(run.code, 0, "{}", run.all());
    assert!(
        json(&run)["runners"][0].get("plan").is_none(),
        "{}",
        run.stdout
    );
    unchanged(&p, &after_waiver);
    assert_eq!(without_waiver(p.fields_of(&id)), fields);
    assert_eq!(ledger_of(&p, &id), waived);

    // A collect that records another commit proves the plan it writes leaves this one, waiver included, alone.
    p.git_commit(
        "test(01-01): Store module\n\nGsd-Task: 01-01/T1\n",
        &[("src/tasks.js", &"a\n".repeat(3))],
        LATER,
    );
    let run = p.collect(&["--runner", "gsd-git"]);
    assert_eq!(run.code, 0, "{}", run.all());
    assert!(
        json(&run)["runners"][0].get("plan").is_some(),
        "{}",
        run.stdout
    );
    assert_eq!(commit_of(&p, "test").len(), 12);
    assert_eq!(p.fields_of(&id), waived_fields);
    assert_eq!(ledger_of(&p, &id), waived);
}

/// After `title_linked`'s plan metadata commit.
const LATER: &str = "2026-10-29T10:00:00Z";

/// `specforge gsd waive-trailer <args>`, as a person at a terminal runs it.
fn waive(p: &Project, args: &[&str]) -> Run {
    gsd_as_person(p, "waive-trailer", args)
}

/// Under gsd-strict, `specforge check` exits 1 with `count` gsd/E521 errors.
fn judged(p: &Project, count: usize) {
    let (code, errors) = e521_errors(p);
    assert_eq!((code, errors.len()), (1, count), "{errors:?}");
}

#[test]
fn a_waiver_that_names_no_commit_or_gives_no_reason_clears_nothing() {
    let p = title_linked();
    strict(&p);
    let feat = commit_of(&p, "feat");
    let before = bytes(&p);
    let run = waive(&p, &[]);
    assert_eq!(run.code, 2, "{}", run.all());
    assert!(run.all().contains("COMMIT"), "{}", run.all());
    for commit in ["", "   "] {
        let run = waive(&p, &[commit, "a reason", "ada"]);
        refused(&run, "invalid_input", &["'commit' is required"]);
    }
    let run = waive(&p, &[&feat]);
    assert_eq!(run.code, 2, "{}", run.all());
    assert!(run.all().contains("REASON"), "{}", run.all());
    for reason in ["", "  "] {
        let run = waive(&p, &[&feat, reason, "ada"]);
        refused(&run, "invalid_input", &["'reason' is required"]);
    }
    unchanged(&p, &before);
    judged(&p, 2);
}

/// A commit's id is its 12-hex name and holds no `/`, so no shorter suffix of it resolves; nor does its full hash, an
/// abbreviation git would take, or another case.
#[test]
fn only_the_twelve_hex_name_resolves() {
    let p = title_linked();
    strict(&p);
    let feat = commit_of(&p, "feat");
    let hash = p.fields_of(&feat)["hash"]
        .as_str()
        .expect("the recorded hash")
        .to_string();
    assert_eq!((hash.len(), &hash[..12]), (40, feat.as_str()));
    let before = bytes(&p);
    for name in [hash.clone(), hash[..7].to_string(), feat.to_uppercase()] {
        let run = waive(&p, &[&name, "a reason", "ada"]);
        refused(&run, "invalid_input", &[&format!("no commit '{name}'")]);
    }
    unchanged(&p, &before);
    judged(&p, 2);
}

#[test]
fn a_control_character_in_the_reason_is_refused_and_a_tab_or_line_feed_is_kept() {
    let p = title_linked();
    strict(&p);
    let feat = commit_of(&p, "feat");
    let before = bytes(&p);
    let run = waive(&p, &[&feat, "why\u{1b}[31m", "ada"]);
    refused(
        &run,
        "invalid_input",
        &["'reason' holds a control character (U+001B)"],
    );
    unchanged(&p, &before);
    judged(&p, 2);
    let reason = "first\tline\nsecond";
    let run = waive(&p, &[&feat, reason, "ada"]);
    assert_eq!(run.code, 0, "{}", run.all());
    assert_eq!(p.fields_of(&feat)["waiver"]["reason"], reason);
    judged(&p, 1);
}

#[test]
fn a_second_waiver_is_refused_and_the_first_stays() {
    let p = title_linked();
    strict(&p);
    let feat = commit_of(&p, "feat");
    let run = waive(&p, &[&feat, "recorded before the trailer existed", "ada"]);
    assert_eq!(run.code, 0, "{}", run.all());
    let first = bytes(&p);
    let run = waive(&p, &[&feat, "another reason", "grace"]);
    refused(&run, "invalid_input", &["already waived", &feat]);
    unchanged(&p, &first);
    assert_eq!(p.fields_of(&feat)["waiver"]["by"], "ada");
    judged(&p, 1);
}

/// gsd/E521 judges a commit whose task its subject's title found; a trailer-linked commit and a plan's `docs` commit
/// have nothing to waive.
#[test]
fn a_commit_gsd_e521_does_not_judge_is_refused() {
    let p = committed(&[
        "feat(01-01): Store module",
        "fix(01-01): Store module\n\nGsd-Task: 01-01/T1\n",
    ]);
    strict(&p);
    judged(&p, 1);
    let before = bytes(&p);
    for kind in ["fix", "docs"] {
        let commit = commit_of(&p, kind);
        let run = waive(&p, &[&commit, "a reason", "ada"]);
        refused(&run, "invalid_input", &["does not judge", &commit]);
        assert!(p.fields_of(&commit)["waiver"].is_null(), "{kind}");
    }
    unchanged(&p, &before);
    judged(&p, 1);
    let run = waive(&p, &[&commit_of(&p, "feat"), "a reason", "ada"]);
    assert_eq!(run.code, 0, "{}", run.all());
    assert_eq!(e521_errors(&p), (0, Vec::new()));
}

/// Only the command records a waiver: one typed into the `.spec` has no ledger line, and the host reports it.
#[test]
fn a_waiver_without_its_ledger_line_is_e083() {
    let p = title_linked();
    let feat = commit_of(&p, "feat");
    let run = waive(&p, &[&feat, "a reason", "ada"]);
    assert_eq!(run.code, 0, "{}", run.all());
    unledger(&p, &feat, "waiver");
    let codes = p.codes();
    assert!(codes.iter().any(|c| c == "E083"), "{codes:?}");
    let run = p.run(&["check"]);
    assert_ne!(run.code, 0, "{}", run.all());
}
