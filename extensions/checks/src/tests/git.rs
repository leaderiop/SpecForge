//! The git family: what each checker plans for a name and finds in canned answers, as the host would send them.

use serde_json::json;
use specforge_protocol_types::{
    CheckFinding, OutcomeClass, OutcomeData, Probe, ProbeOutcome, Root,
};

use super::{e103_at, planned};

fn answer(probe: Probe, found: bool) -> ProbeOutcome {
    ProbeOutcome {
        probe,
        class: if found {
            OutcomeClass::Ok
        } else {
            OutcomeClass::Missing
        },
        data: OutcomeData::Git { found, oid: None },
        observed_at: 0,
        from_cache: false,
    }
}

fn judged_all(
    checker: &str,
    text: &str,
    args: serde_json::Value,
    outcomes: Vec<ProbeOutcome>,
) -> Vec<CheckFinding> {
    specforge_extension_sdk::testing::MockHost::new(crate::specforge_extension_build())
        .judge(checker, text, args, outcomes)
}

fn codes(findings: &[CheckFinding]) -> Vec<&str> {
    findings.iter().map(|f| f.code.as_str()).collect()
}

#[test]
fn git_commit_asks_the_object_and_reports_a_missing_one_as_e106() {
    let plan = planned("commit", "ABCDEF1", json!({}));
    let probe = Probe::GitObject {
        rev: "abcdef1".into(),
    };
    assert_eq!(plan.probes, std::slice::from_ref(&probe));
    assert_eq!(e103_at("commit", "abcdeg1", json!({})), [5, 6]);
    // Shorter than `min_len`: refused before git is asked.
    assert!(planned("commit", "abcde", json!({})).probes.is_empty());
    assert_eq!(
        planned("commit", "abcde", json!({"min_len": 5}))
            .probes
            .len(),
        1
    );

    let gone = judged_all(
        "commit",
        "abcdef1",
        json!({}),
        vec![answer(probe.clone(), false)],
    );
    assert_eq!(codes(&gone), ["E106"]);
    assert!(judged_all("commit", "abcdef1", json!({}), vec![answer(probe, true)]).is_empty());
}

#[test]
fn git_ref_asks_its_kinds_names_and_needs_one_of_them() {
    let tag = Probe::GitRef {
        name: "refs/tags/v1.0".into(),
    };
    let branch = Probe::GitRef {
        name: "refs/heads/v1.0".into(),
    };
    assert_eq!(
        planned("ref", "v1.0", json!({"kind": "tag"})).probes,
        std::slice::from_ref(&tag)
    );
    assert_eq!(
        planned("ref", "v1.0", json!({"kind": "branch"})).probes,
        std::slice::from_ref(&branch)
    );
    assert_eq!(
        planned("ref", "v1.0", json!({})).probes,
        [tag.clone(), branch.clone()]
    );
    assert_eq!(planned("ref", "refs/notes/x", json!({})).probes.len(), 1);
    assert_eq!(e103_at("ref", "v1..0", json!({})), [2, 4]);
    assert!(planned("ref", "-v1", json!({})).probes.is_empty());

    let one = vec![answer(tag.clone(), false), answer(branch.clone(), true)];
    assert!(judged_all("ref", "v1.0", json!({}), one).is_empty());
    let none = vec![answer(tag, false), answer(branch, false)];
    assert_eq!(codes(&judged_all("ref", "v1.0", json!({}), none)), ["E106"]);
}

#[test]
fn git_file_at_asks_the_file_at_its_revision_and_points_at_the_path() {
    let text = "v1.0:src/lib.rs";
    let probe = Probe::GitFile {
        rev: "v1.0".into(),
        path: "src/lib.rs".into(),
    };
    assert_eq!(
        planned("file_at", text, json!({})).probes,
        std::slice::from_ref(&probe)
    );
    assert_eq!(e103_at("file_at", "v1.0:../x", json!({})), [5, 9]);
    assert!(planned("file_at", "v1.0", json!({})).probes.is_empty());

    let gone = judged_all(
        "file_at",
        text,
        json!({}),
        vec![answer(probe.clone(), false)],
    );
    assert_eq!(codes(&gone), ["E106"]);
    assert_eq!(gone[0].at, Some([5, text.len()]));
    assert!(judged_all("file_at", text, json!({}), vec![answer(probe, true)]).is_empty());
}

#[test]
fn git_tracked_asks_at_head_under_its_root_and_warns_w180() {
    let probe = Probe::GitTracked {
        root: Root::Spec,
        path: "a.spec".into(),
    };
    assert_eq!(
        planned("tracked", "a.spec", json!({"root": "spec"})).probes,
        std::slice::from_ref(&probe)
    );
    let untracked = judged_all(
        "tracked",
        "a.spec",
        json!({}),
        vec![answer(probe.clone(), false)],
    );
    assert_eq!(codes(&untracked), ["W180"]);
    assert!(judged_all("tracked", "a.spec", json!({}), vec![answer(probe, true)]).is_empty());
}

#[test]
fn git_a_repository_that_could_not_answer_decides_nothing() {
    let probe = Probe::GitObject {
        rev: "abcdef1".into(),
    };
    let denied = ProbeOutcome {
        class: OutcomeClass::Denied,
        data: OutcomeData::None,
        ..answer(probe, false)
    };
    assert!(judged_all("commit", "abcdef1", json!({}), vec![denied]).is_empty());
}
