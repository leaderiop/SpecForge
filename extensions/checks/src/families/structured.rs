//! The structured family: that a JSON, YAML or TOML file parses, that a JSON pointer into one names something, and that
//! a Markdown heading anchor exists. Each reads the file the value names, at most 1 MiB of it, through the host.

use specforge_checks_kit::links::fragment_id;
use specforge_checks_kit::pointer;
use specforge_checks_kit::slug;
use specforge_checks_kit::tree::{self, Format};
use specforge_extension_sdk::prelude::*;

use super::files::missing;
use super::{checker, choice_arg, finding_of, no_args, Args, Word, STRUCTURED};
use crate::codes::*;

/// The most of a file the family reads: past it, a file is not checked (W176).
const READ: u64 = 1024 * 1024;
/// The most anchors an E098 lists.
const LISTED: usize = 8;

/// The format a `pointer` reads, when its `format` argument names one.
#[derive(Clone, Copy)]
struct Named(Format);

impl Word for Named {
    const VOCABULARY: Vocabulary = Vocabulary::new(&["json", "yaml", "toml"]);
    const ALL: &'static [Self] = &[
        Named(Format::Json),
        Named(Format::Yaml),
        Named(Format::Toml),
    ];
}

/// A value read as a path and, after its first `#`, a fragment: where the `#` is, when there is one.
fn split(text: &str) -> (&str, Option<(usize, &str)>) {
    match text.find('#') {
        Some(at) => (&text[..at], Some((at, &text[at + 1..]))),
        None => (text, None),
    }
}

fn read(path: &str, args: Args) -> Probe {
    Probe::FsRead {
        root: args.root(),
        path: path.to_string(),
        range: ByteRange::Whole { max: READ },
    }
}

/// The whole file, when the host read it whole; else what the value is told instead (E016, or W176 for a file over
/// what is read or that is no regular file), or nothing when the host reported it itself.
fn whole<'o>(
    path: &str,
    args: Args,
    outcome: &'o ProbeOutcome,
) -> Result<&'o [u8], Vec<CheckFinding>> {
    let told = |code: &str, why: String| Err(vec![CheckFinding::new(code, why).at(0, path.len())]);
    match (&outcome.class, &outcome.data) {
        (OutcomeClass::Missing, _) => Err(vec![missing(path, args, outcome, [0, path.len()])]),
        (OutcomeClass::Ok, OutcomeData::Bytes { truncated: true, length, .. }) => told(
            W176,
            format!("'{path}' is {length} bytes, over the 1 MiB a structured check reads: it is not checked"),
        ),
        (OutcomeClass::Ok, OutcomeData::Bytes { bytes, .. }) => Ok(bytes),
        (OutcomeClass::Denied, _) => told(W176, format!("'{path}' is no regular file that can be read")),
        _ => Err(Vec::new()),
    }
}

/// E097 for `path`, which does not parse as `format`.
fn unparsed(path: &str, format: Format, error: &tree::ParseError) -> CheckFinding {
    CheckFinding::new(
        E097,
        format!("'{path}' does not parse as {}: {error}", format.as_str()),
    )
    .at(0, path.len())
}

/// A checker that `path` parses as `format`.
fn parses(c: &mut ContributionsBuilder, name: &str, format: Format) {
    let plan = |text: &str, args: Args| Ok(vec![read(text, args)]);
    let judge = move |text: &str, args: Args, outcomes: &[ProbeOutcome]| match whole(
        text,
        args,
        &outcomes[0],
    ) {
        Ok(bytes) => match tree::parse(format, bytes) {
            Ok(_) => Vec::new(),
            Err(error) => vec![unparsed(text, format, &error)],
        },
        Err(told) => told,
    };
    checker(
        c,
        &STRUCTURED,
        name,
        &[E016, E097, W176],
        path_shape(no_args),
        plan,
        judge,
    );
}

/// The format of a `pointer` value's file: its `format` argument, else its extension.
fn format_of(path: &str, args: Args) -> Option<Format> {
    args.choice::<Named>("format")
        .map(|named| named.0)
        .or_else(|| Format::of_path(path))
}

fn pointer_plan(text: &str, args: Args) -> Result<Vec<Probe>, CheckFinding> {
    let (path, fragment) = split(text);
    if let Some((at, fragment)) = fragment {
        pointer::tokens(fragment).map_err(|invalid| {
            let at = Some(invalid.at.map_or(at + 1..text.len(), |range| {
                range.start + at + 1..range.end + at + 1
            }));
            finding_of(E103, specforge_checks_kit::Invalid { at, ..invalid })
        })?;
    }
    if format_of(path, args).is_none() {
        let why = format!(
            "the format of '{path}' is not known from its extension: name it with `format`"
        );
        return Err(CheckFinding::new(E097, why).at(0, path.len()));
    }
    Ok(vec![read(path, args)])
}

fn pointer_judge(text: &str, args: Args, outcomes: &[ProbeOutcome]) -> Vec<CheckFinding> {
    let (path, fragment) = split(text);
    let (Some(format), Ok(tokens)) = (
        format_of(path, args),
        pointer::tokens(fragment.map_or("", |(_, f)| f)),
    ) else {
        return Vec::new();
    };
    let bytes = match whole(path, args, &outcomes[0]) {
        Ok(bytes) => bytes,
        Err(told) => return told,
    };
    let document = match tree::parse(format, bytes) {
        Ok(document) => document,
        Err(error) => return vec![unparsed(path, format, &error)],
    };
    let Err(reached) = pointer::resolve(&document, &tokens) else {
        return Vec::new();
    };
    let at = fragment.map_or(text.len(), |(at, _)| at);
    let deepest = match pointer::written(&tokens, reached) {
        written if written.is_empty() => "the document itself".to_string(),
        written => format!("'{written}'"),
    };
    let finding = CheckFinding::new(
        E098,
        format!(
            "'{}' names nothing in '{path}': {deepest} is the deepest part that exists",
            pointer::written(&tokens, tokens.len())
        ),
    )
    .at(at, text.len());
    vec![finding]
}

fn anchor_judge(text: &str, args: Args, outcomes: &[ProbeOutcome]) -> Vec<CheckFinding> {
    let (path, fragment) = split(text);
    let bytes = match whole(path, args, &outcomes[0]) {
        Ok(bytes) => bytes,
        Err(told) => return told,
    };
    let Some((at, fragment)) = fragment.filter(|(_, f)| !f.is_empty()) else {
        return Vec::new();
    };
    let markdown = String::from_utf8_lossy(bytes);
    let wanted = fragment_id(fragment);
    if slug::anchors(&markdown).contains(wanted.as_ref()) {
        return Vec::new();
    }
    let headings = slug::heading_anchors(&markdown);
    let listed = headings
        .iter()
        .take(LISTED)
        .map(|anchor| format!("#{anchor}"))
        .collect::<Vec<_>>()
        .join(", ");
    let more = headings.len().saturating_sub(LISTED);
    let help = match (headings.is_empty(), more) {
        (true, _) => "it has no heading".to_string(),
        (false, 0) => format!("its headings: {listed}"),
        (false, more) => format!("its headings: {listed} and {more} more"),
    };
    let finding = CheckFinding::new(E098, format!("'{path}' has no anchor '#{wanted}'"))
        .at(at, text.len())
        .with_help(help);
    vec![finding]
}

/// A path value under its `root` argument, with `args`.
fn path_shape(args: impl FnOnce(&mut CheckerBuilder)) -> impl FnOnce(&mut CheckerBuilder) {
    move |k| {
        k.arg("root", |f| {
            f.field_type(FieldType::String);
        });
        args(k);
        k.value(ValueForm::Path).root_arg("root");
    }
}

pub(super) fn contribute(c: &mut ContributionsBuilder) {
    parses(c, "json", Format::Json);
    parses(c, "yaml", Format::Yaml);
    parses(c, "toml", Format::Toml);
    checker(
        c,
        &STRUCTURED,
        "pointer",
        &[E016, E097, E098, E103, W176],
        path_shape(choice_arg::<Named>("format")),
        pointer_plan,
        pointer_judge,
    );
    checker(
        c,
        &STRUCTURED,
        "markdown_anchor",
        &[E016, E098, W176],
        path_shape(no_args),
        |text, args| Ok(vec![read(split(text).0, args)]),
        anchor_judge,
    );
}
