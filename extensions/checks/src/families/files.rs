//! The files family: what the files a value names hold. The host resolves each path under its root and answers the
//! probes; a checker reads only those answers.

use specforge_checks_kit::files::{glob, DigestPin, LineEnds, LineRange, Text};
use specforge_checks_kit::Invalid;
use specforge_extension_sdk::prelude::*;

use super::{checker, choice_arg, finding_of, no_args, Args, Word, FILES};
use crate::codes::*;

/// The most of a file `encoding` and `line_range` read: past it, a line range not yet reached is not decided.
const READ: u32 = 1024 * 1024;

/// The line ends `encoding` wants.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Ends {
    Lf,
    Crlf,
    Any,
}

impl Word for Ends {
    const VOCABULARY: Vocabulary = Vocabulary::new(&["lf", "crlf", "any"]);
    const ALL: &'static [Self] = &[Ends::Lf, Ends::Crlf, Ends::Any];
}

/// The one probe a path checker asks of a value's text, or what the kit refused in it.
type PathProbe = fn(&str, Args) -> Result<Probe, Invalid>;
/// What a path checker finds in the one answer to it.
type PathJudge = fn(&str, Args, &ProbeOutcome) -> Vec<CheckFinding>;

/// A checker of one path under its `root` argument: E103 for text the kit refuses.
fn path_checker(
    c: &mut ContributionsBuilder,
    name: &str,
    codes: &[&str],
    args: impl FnOnce(&mut CheckerBuilder),
    probe: PathProbe,
    judge: PathJudge,
) {
    let shape = |k: &mut CheckerBuilder| {
        k.arg("root", |f| {
            f.field_type(FieldType::String);
        });
        args(k);
        k.value(ValueForm::Path).root_arg("root");
    };
    let plan = move |text: &str, args: Args| {
        probe(text, args)
            .map(|probe| vec![probe])
            .map_err(|invalid| finding_of(E103, invalid))
    };
    checker(
        c,
        &FILES,
        name,
        codes,
        shape,
        plan,
        move |text, args, outcomes| judge(text, args, &outcomes[0]),
    );
}

fn stat(text: &str, args: Args) -> Result<Probe, Invalid> {
    Ok(Probe::FsStat {
        root: args.root(),
        path: text.to_string(),
    })
}

/// E016 for `path`, missing under the root, on the bytes `at` of the value; with the sibling the host found inside
/// the root, when it found one.
fn missing(path: &str, args: Args, outcome: &ProbeOutcome, at: [usize; 2]) -> CheckFinding {
    let finding = CheckFinding::new(
        E016,
        format!("'{path}' does not exist under <{}>", args.root()),
    )
    .at(at[0], at[1]);
    match &outcome.data {
        OutcomeData::Stat {
            near: Some(near), ..
        } => finding.with_help(format!("did you mean '{near}'?")),
        _ => finding,
    }
}

fn whole(text: &str) -> [usize; 2] {
    [0, text.len()]
}

/// What a path names, when the host found it.
fn entry(outcome: &ProbeOutcome) -> Option<StatEntry> {
    match (&outcome.class, &outcome.data) {
        (OutcomeClass::Ok, OutcomeData::Stat { entry, .. }) => *entry,
        _ => None,
    }
}

/// E016 for `text`, missing or not `wanted`, as `fs.stat` answered it.
fn of_entry(
    text: &str,
    args: Args,
    outcome: &ProbeOutcome,
    wanted: StatEntry,
) -> Vec<CheckFinding> {
    let found = match (outcome.class, entry(outcome)) {
        (OutcomeClass::Missing, _) => missing(text, args, outcome, whole(text)),
        (_, Some(other)) if other != wanted => {
            let (is, not) = match wanted {
                StatEntry::File => ("a directory", "a file"),
                StatEntry::Dir => ("a file", "a directory"),
            };
            CheckFinding::new(E016, format!("'{text}' is {is}, not {not}"))
        }
        _ => return Vec::new(),
    };
    vec![found]
}

fn file(text: &str, args: Args, outcome: &ProbeOutcome) -> Vec<CheckFinding> {
    of_entry(text, args, outcome, StatEntry::File)
}

fn dir(text: &str, args: Args, outcome: &ProbeOutcome) -> Vec<CheckFinding> {
    of_entry(text, args, outcome, StatEntry::Dir)
}

/// A glob's literal leading directories, walked, and the rest, matched under them.
pub(crate) fn split_glob(pattern: &str) -> (&str, &str) {
    let cut = match pattern.find(['*', '?']) {
        Some(wild) => pattern[..wild].rfind('/'),
        None => pattern.rfind('/'),
    };
    match cut {
        Some(at) => (&pattern[..at], &pattern[at + 1..]),
        None => ("", pattern),
    }
}

fn walk(text: &str, args: Args) -> Result<Probe, Invalid> {
    glob::validate(text)?;
    let (base, rest) = split_glob(text);
    let root = args.root();
    let path = match base {
        "" => format!("<{root}>"),
        base => base.to_string(),
    };
    Ok(Probe::FsWalk {
        root,
        path,
        glob: rest.to_string(),
    })
}

fn matched(text: &str, args: Args, outcome: &ProbeOutcome) -> Vec<CheckFinding> {
    let (count, truncated) = match (&outcome.class, &outcome.data) {
        (OutcomeClass::Ok, OutcomeData::Walk { entries, truncated }) => {
            (entries.len() as u64, *truncated)
        }
        (OutcomeClass::Missing, _) => (0, false),
        _ => return Vec::new(),
    };
    let min = args.count("min").unwrap_or(1);
    if count < min && !truncated {
        let found = CheckFinding::new(
            E016,
            format!(
                "'{text}' matches {count} file(s) under <{}>, fewer than {min}",
                args.root()
            ),
        );
        return vec![found];
    }
    match args.count("max") {
        Some(max) if count > max => vec![CheckFinding::new(
            W176,
            format!("'{text}' matches {count} file(s), more than {max}"),
        )],
        _ => Vec::new(),
    }
}

fn sized(text: &str, args: Args, outcome: &ProbeOutcome) -> Vec<CheckFinding> {
    if outcome.class == OutcomeClass::Missing {
        return vec![missing(text, args, outcome, whole(text))];
    }
    let OutcomeData::Stat {
        entry: Some(StatEntry::File),
        size,
        ..
    } = outcome.data
    else {
        return file(text, args, outcome);
    };
    let small = args.count("min").filter(|min| size < *min);
    let large = args.count("max").filter(|max| size > *max);
    let found = match (small, large) {
        (Some(min), _) => format!("'{text}' is {size} bytes, fewer than {min}"),
        (_, Some(max)) => format!("'{text}' is {size} bytes, more than {max}"),
        _ => return Vec::new(),
    };
    vec![CheckFinding::new(W176, found)]
}

fn pinned(text: &str, args: Args) -> Result<Probe, Invalid> {
    let pin = DigestPin::parse(text)?;
    Ok(Probe::FsDigest {
        root: args.root(),
        path: pin.path.to_string(),
    })
}

fn digest(text: &str, args: Args, outcome: &ProbeOutcome) -> Vec<CheckFinding> {
    let Ok(pin) = DigestPin::parse(text) else {
        return Vec::new();
    };
    let found = match (&outcome.class, &outcome.data) {
        (OutcomeClass::Missing, _) => missing(pin.path, args, outcome, [0, pin.path.len()]),
        (OutcomeClass::Ok, OutcomeData::Digest { sha256, .. }) if *sha256 != pin.sha256 => {
            CheckFinding::new(
                E096,
                format!("'{}' has the sha256 {sha256}, not the pinned one", pin.path),
            )
            .at(pin.at.start, pin.at.end)
            .with_help(format!(
                "pin '{}@sha256:{sha256}' if the change is meant",
                pin.path
            ))
        }
        (OutcomeClass::Denied, _) => CheckFinding::new(
            W176,
            format!("'{}' is no regular file that can be read", pin.path),
        )
        .at(0, pin.path.len()),
        _ => return Vec::new(),
    };
    vec![found]
}

fn read(path: &str, args: Args) -> Probe {
    Probe::FsRead {
        root: args.root(),
        path: path.to_string(),
        bytes: READ,
    }
}

/// The bytes read and whether the file goes on past them, when the host read it.
fn bytes(outcome: &ProbeOutcome) -> Option<(&[u8], bool)> {
    match (&outcome.class, &outcome.data) {
        (
            OutcomeClass::Ok,
            OutcomeData::Bytes {
                bytes, truncated, ..
            },
        ) => Some((bytes, *truncated)),
        _ => None,
    }
}

fn encoded(text: &str, args: Args, outcome: &ProbeOutcome) -> Vec<CheckFinding> {
    if outcome.class == OutcomeClass::Missing {
        return vec![missing(text, args, outcome, whole(text))];
    }
    let Some((mut read, truncated)) = bytes(outcome) else {
        return Vec::new();
    };
    // A read cut inside a character is not a broken file.
    if let (true, Err(error)) = (truncated, std::str::from_utf8(read)) {
        if error.error_len().is_none() {
            read = &read[..error.valid_up_to()];
        }
    }
    let text_is = Text::classify(read);
    let wanted = args.choice("line_ends").unwrap_or(Ends::Any);
    let problem = if !text_is.utf8 {
        "is not UTF-8"
    } else if text_is.bom && !args.flag("bom", false) {
        "starts with a byte order mark"
    } else {
        match (wanted, text_is.line_ends) {
            (Ends::Lf, LineEnds::Crlf | LineEnds::Mixed) => "has lines that end in CRLF, not LF",
            (Ends::Crlf, LineEnds::Lf | LineEnds::Mixed) => "has lines that end in LF, not CRLF",
            _ => return Vec::new(),
        }
    };
    vec![CheckFinding::new(W176, format!("'{text}' {problem}"))]
}

fn ranged(text: &str, args: Args) -> Result<Probe, Invalid> {
    LineRange::parse(text).map(|range| read(range.path, args))
}

fn lines(text: &str, args: Args, outcome: &ProbeOutcome) -> Vec<CheckFinding> {
    let Ok(range) = LineRange::parse(text) else {
        return Vec::new();
    };
    if outcome.class == OutcomeClass::Missing {
        return vec![missing(range.path, args, outcome, [0, range.path.len()])];
    }
    let Some((read, truncated)) = bytes(outcome) else {
        return Vec::new();
    };
    let ends = read.iter().filter(|byte| **byte == b'\n').count();
    let count = ends + usize::from(!read.is_empty() && !read.ends_with(b"\n"));
    // Lines past what was read may still be there.
    if range.end as usize <= count || truncated {
        return Vec::new();
    }
    let found = CheckFinding::new(
        W176,
        format!(
            "'{}' has {count} line(s); line {} is past its end",
            range.path, range.end
        ),
    )
    .at(range.at.start, range.at.end);
    vec![found]
}

fn bounds(k: &mut CheckerBuilder) {
    for name in ["min", "max"] {
        k.arg(name, |f| {
            f.field_type(FieldType::Integer).minimum(0);
        });
    }
}

pub(super) fn contribute(c: &mut ContributionsBuilder) {
    path_checker(c, "file", &[E016], no_args, stat, file);
    path_checker(c, "dir", &[E016], no_args, stat, dir);
    path_checker(c, "glob", &[E016, W176, E103], bounds, walk, matched);
    path_checker(c, "size", &[E016, W176], bounds, stat, sized);
    path_checker(
        c,
        "digest",
        &[E016, E096, E103, W176],
        no_args,
        pinned,
        digest,
    );
    let encoding = |k: &mut CheckerBuilder| {
        k.arg("bom", |f| {
            f.field_type(FieldType::Bool);
        });
        choice_arg::<Ends>("line_ends")(k);
    };
    path_checker(
        c,
        "encoding",
        &[E016, W176],
        encoding,
        |t, a| Ok(read(t, a)),
        encoded,
    );
    path_checker(c, "line_range", &[E016, W176, E103], no_args, ranged, lines);
}
