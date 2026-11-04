//! The git family: the commits, refs and tracked paths a value names, as the repository the project is in answers.
//! A name the kit refuses is E103 and never reaches git.

use specforge_checks_kit::git::{FileAt, ObjectName, RefName};
use specforge_extension_sdk::prelude::*;

use super::{finding_of, word, Args, GIT, TEXT};
use crate::codes::*;

const SWITCH: &str = "families.git";
const KIND: Vocabulary = Vocabulary::new(&["tag", "branch", "any"]);
/// The fewest hex digits a `commit` value may abbreviate a commit to.
const MIN_LEN: u64 = 7;

/// The probes a checker's plan asks of a value, or the finding that stops them.
type Probing = fn(&str, &Args) -> Result<Vec<Probe>, CheckFinding>;
/// What it finds once the probes answered.
type Judging = fn(&str, &Args, &[ProbeOutcome]) -> Vec<CheckFinding>;

fn git_checker(
    c: &mut ContributionsBuilder,
    name: &str,
    codes: &[&str],
    args: impl FnOnce(&mut CheckerBuilder),
    probe: Probing,
    judge: Judging,
) {
    c.checker(name, |k| {
        args(k);
        k.applies_to(TEXT)
            .scope(GIT)
            .codes(codes)
            .when(SWITCH)
            .plan(move |item| match probe(&item.text, &item.args) {
                _ if item.text.is_empty() => PlannedItem::default(),
                Ok(probes) => PlannedItem::probes(probes),
                Err(found) => PlannedItem::findings(vec![found]),
            })
            .judge(move |item| match item.outcomes.as_slice() {
                [] => Vec::new(),
                outcomes => judge(&item.text, &item.args, outcomes),
            });
    });
}

/// Whether any probe found what it asked for.
fn found(outcomes: &[ProbeOutcome]) -> bool {
    outcomes.iter().any(|outcome| {
        outcome.class == OutcomeClass::Ok
            && matches!(
                outcome.data,
                OutcomeData::Git { found: true, .. } | OutcomeData::None
            )
    })
}

/// Whether every probe was answered "not there": a repository that could not be asked decides nothing.
fn absent(outcomes: &[ProbeOutcome]) -> bool {
    !found(outcomes)
        && outcomes
            .iter()
            .all(|outcome| outcome.class == OutcomeClass::Missing)
}

fn object(text: &str, args: &Args) -> Result<Vec<Probe>, CheckFinding> {
    let name = ObjectName::parse(text).map_err(finding_of)?;
    let min = args
        .get("min_len")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(MIN_LEN);
    if (text.len() as u64) < min {
        return Err(CheckFinding::new(
            E103,
            format!("'{text}' has {} hex digits, fewer than {min}", text.len()),
        ));
    }
    Ok(vec![Probe::GitObject {
        rev: name.as_str().to_string(),
    }])
}

fn commit(text: &str, _: &Args, outcomes: &[ProbeOutcome]) -> Vec<CheckFinding> {
    if !absent(outcomes) {
        return Vec::new();
    }
    vec![CheckFinding::new(
        E106,
        format!("this repository has no commit '{text}'"),
    )]
}

/// The full names a `ref` value may stand for under its `kind`.
fn ref_names(text: &str, args: &Args) -> Vec<String> {
    if text.starts_with("refs/") {
        return vec![text.to_string()];
    }
    let tag = format!("refs/tags/{text}");
    let branch = format!("refs/heads/{text}");
    match word(args, "kind") {
        Some("tag") => vec![tag],
        Some("branch") => vec![branch],
        _ => vec![tag, branch],
    }
}

fn named(text: &str, args: &Args) -> Result<Vec<Probe>, CheckFinding> {
    RefName::check(text).map_err(finding_of)?;
    Ok(ref_names(text, args)
        .into_iter()
        .map(|name| Probe::GitRef { name })
        .collect())
}

fn reference(text: &str, args: &Args, outcomes: &[ProbeOutcome]) -> Vec<CheckFinding> {
    if !absent(outcomes) {
        return Vec::new();
    }
    let what = match word(args, "kind") {
        Some("tag") => "tag",
        Some("branch") => "branch",
        _ => "tag or branch",
    };
    vec![CheckFinding::new(
        E106,
        format!("this repository has no {what} '{text}'"),
    )]
}

fn at_revision(text: &str, _: &Args) -> Result<Vec<Probe>, CheckFinding> {
    let at = FileAt::parse(text).map_err(finding_of)?;
    Ok(vec![Probe::GitFile {
        rev: at.rev.to_string(),
        path: at.path.to_string(),
    }])
}

fn file_at(text: &str, _: &Args, outcomes: &[ProbeOutcome]) -> Vec<CheckFinding> {
    let Ok(at) = FileAt::parse(text) else {
        return Vec::new();
    };
    if !absent(outcomes) {
        return Vec::new();
    }
    let path_from = at.rev.len() + 1;
    vec![CheckFinding::new(
        E106,
        format!(
            "'{}' holds no file '{}', or is no revision here",
            at.rev, at.path
        ),
    )
    .at(path_from, text.len())]
}

/// Where a bare path starts: the `root` argument, which the host admits only as a root's name, else the project.
fn root(args: &Args) -> Root {
    args.get("root")
        .and_then(|name| serde_json::from_value(name.clone()).ok())
        .unwrap_or(Root::Project)
}

fn tracked(c: &mut ContributionsBuilder) {
    c.checker("tracked", |k| {
        k.arg("root", |f| {
            f.field_type(FieldType::String);
        });
        k.applies_to(TEXT)
            .value(ValueForm::Path)
            .root_arg("root")
            .scope(GIT)
            .codes(&[W180])
            .when(SWITCH)
            .plan(|item| match item.text.as_str() {
                "" => PlannedItem::default(),
                path => PlannedItem::probes(vec![Probe::GitTracked {
                    root: root(&item.args),
                    path: path.to_string(),
                }]),
            })
            .judge(|item| {
                if item.outcomes.is_empty() || !absent(&item.outcomes) {
                    return Vec::new();
                }
                vec![CheckFinding::new(
                    W180,
                    format!("'{}' is not in the commit at HEAD", item.text),
                )]
            });
    });
}

pub(super) fn contribute(c: &mut ContributionsBuilder) {
    let min_len = |k: &mut CheckerBuilder| {
        k.arg("min_len", |f| {
            f.field_type(FieldType::Integer).minimum(4).maximum(64);
        });
    };
    git_checker(c, "commit", &[E103, E106], min_len, object, commit);
    let kind = |k: &mut CheckerBuilder| {
        k.arg("kind", |f| {
            f.vocabulary(&KIND);
        });
    };
    git_checker(c, "ref", &[E103, E106], kind, named, reference);
    let none = |_: &mut CheckerBuilder| {};
    git_checker(c, "file_at", &[E103, E106], none, at_revision, file_at);
    tracked(c);
}
