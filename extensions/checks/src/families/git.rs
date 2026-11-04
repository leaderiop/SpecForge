//! The git family: the commits, refs and tracked paths a value names, as the repository the project is in answers.
//! A name the kit refuses is E103 and never reaches git.

use specforge_checks_kit::git::{FileAt, ObjectName, RefName};
use specforge_extension_sdk::prelude::*;

use super::{checker, choice_arg, finding_of, no_args, Args, Word, GIT};
use crate::codes::*;

/// What a `ref` value may name.
#[derive(Clone, Copy, PartialEq, Eq)]
enum RefKind {
    Tag,
    Branch,
    Any,
}

impl Word for RefKind {
    const VOCABULARY: Vocabulary = Vocabulary::new(&["tag", "branch", "any"]);
    const ALL: &'static [Self] = &[RefKind::Tag, RefKind::Branch, RefKind::Any];
}
/// The fewest hex digits a `commit` value may abbreviate a commit to.
const MIN_LEN: u64 = 7;

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

fn object(text: &str, args: Args) -> Result<Vec<Probe>, CheckFinding> {
    let name = ObjectName::parse(text).map_err(|invalid| finding_of(E103, invalid))?;
    let min = args.count("min_len").unwrap_or(MIN_LEN);
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

fn commit(text: &str, _: Args, outcomes: &[ProbeOutcome]) -> Vec<CheckFinding> {
    if !absent(outcomes) {
        return Vec::new();
    }
    vec![CheckFinding::new(
        E106,
        format!("this repository has no commit '{text}'"),
    )]
}

/// The full names a `ref` value may stand for under its `kind`.
fn ref_names(text: &str, args: Args) -> Vec<String> {
    if text.starts_with("refs/") {
        return vec![text.to_string()];
    }
    let tag = format!("refs/tags/{text}");
    let branch = format!("refs/heads/{text}");
    match args.choice("kind").unwrap_or(RefKind::Any) {
        RefKind::Tag => vec![tag],
        RefKind::Branch => vec![branch],
        RefKind::Any => vec![tag, branch],
    }
}

fn named(text: &str, args: Args) -> Result<Vec<Probe>, CheckFinding> {
    RefName::check(text).map_err(|invalid| finding_of(E103, invalid))?;
    Ok(ref_names(text, args)
        .into_iter()
        .map(|name| Probe::GitRef { name })
        .collect())
}

fn reference(text: &str, args: Args, outcomes: &[ProbeOutcome]) -> Vec<CheckFinding> {
    if !absent(outcomes) {
        return Vec::new();
    }
    let what = match args.choice("kind").unwrap_or(RefKind::Any) {
        RefKind::Tag => "tag",
        RefKind::Branch => "branch",
        RefKind::Any => "tag or branch",
    };
    vec![CheckFinding::new(
        E106,
        format!("this repository has no {what} '{text}'"),
    )]
}

fn at_revision(text: &str, _: Args) -> Result<Vec<Probe>, CheckFinding> {
    let at = FileAt::parse(text).map_err(|invalid| finding_of(E103, invalid))?;
    Ok(vec![Probe::GitFile {
        rev: at.rev.to_string(),
        path: at.path.to_string(),
    }])
}

fn file_at(text: &str, _: Args, outcomes: &[ProbeOutcome]) -> Vec<CheckFinding> {
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

fn tracking(text: &str, args: Args) -> Result<Vec<Probe>, CheckFinding> {
    Ok(vec![Probe::GitTracked {
        root: args.root(),
        path: text.to_string(),
    }])
}

fn tracked(text: &str, _: Args, outcomes: &[ProbeOutcome]) -> Vec<CheckFinding> {
    if !absent(outcomes) {
        return Vec::new();
    }
    vec![CheckFinding::new(
        W180,
        format!("'{text}' is not in the commit at HEAD"),
    )]
}

pub(super) fn contribute(c: &mut ContributionsBuilder) {
    let min_len = |k: &mut CheckerBuilder| {
        k.arg("min_len", |f| {
            f.field_type(FieldType::Integer).minimum(4).maximum(64);
        });
    };
    checker(c, &GIT, "commit", &[E103, E106], min_len, object, commit);
    let kind = choice_arg::<RefKind>("kind");
    checker(c, &GIT, "ref", &[E103, E106], kind, named, reference);
    checker(
        c,
        &GIT,
        "file_at",
        &[E103, E106],
        no_args,
        at_revision,
        file_at,
    );
    let rooted = |k: &mut CheckerBuilder| {
        k.arg("root", |f| {
            f.field_type(FieldType::String);
        });
        k.value(ValueForm::Path).root_arg("root");
    };
    checker(c, &GIT, "tracked", &[W180], rooted, tracking, tracked);
}
