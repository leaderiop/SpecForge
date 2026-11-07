//! `specforge wait` (ADR 0342): wait until a URL answers, a file changes or a line matches, or for a time. Its answer
//! is printed as every verb's is; it exits 0 when a condition holds or a plain delay ends, and 124 when the time runs
//! out first, the answer printed all the same.

use std::path::PathBuf;
use std::time::Duration;

use specforge_effects::Ports;
use specforge_effects::UnixMillis;
use specforge_ops::verbs::wait::{
    self as waited, Changed, Matching, MonotonicTimer, Poll, Reason, Wait, WaitAnswer,
};

use super::VerbArgs;
use crate::outcome::Exit;
use crate::terminal::{Terminal, outln};

/// What `--effects` does to a wait: a probe asks about now.
const EFFECTS_HELP: &str = "The effects mode: online (the default), auto and cached all probe the URL live, since a \
    wait asks about now and a fresh record of an earlier wait would end this one at once; replay answers each probe \
    from the effects journal alone. Files are read live in every mode";

#[derive(clap::Args)]
#[command(mut_arg("effects", |arg| arg.help(EFFECTS_HELP)))]
pub(crate) struct WaitArgs {
    /// How long, at most: `0` (check once), `30` (seconds), `500ms`, `3s`, `5m`, `1h` (24h the most). With no
    /// condition, a delay of this long
    #[arg(
        long = "for",
        value_name = "DURATION",
        value_parser = parse_for,
        required_unless_present = "retry_after",
        conflicts_with = "retry_after"
    )]
    for_: Option<Duration>,

    /// In place of --for, how long as a `Retry-After` header says it: seconds (`120`, a fraction taken to the
    /// millisecond) or an HTTP date (a past one is no wait), held to 24h
    #[arg(long, value_name = "SECONDS|HTTP-DATE", value_parser = parse_retry_after)]
    retry_after: Option<Duration>,

    /// Holds once this URL answers, any HTTP status (a GET under the project's effects ceiling)
    #[arg(long, value_name = "URL")]
    url: Option<String>,

    /// Holds once a file this glob names (from the project's root wherever the wait is typed, in the project's roots,
    /// as `files list` reads it; a directory: the files in it) was written at or after --since
    #[arg(long, value_name = "GLOB")]
    changed: Option<String>,

    /// With --changed: the instant a write counts from, in milliseconds since the Unix epoch (a wait's `started_at`);
    /// the wait's own start when absent
    #[arg(long, value_name = "MS", requires = "changed")]
    since: Option<u64>,

    /// Holds once a line of the file --in names matches this pattern (as `files search` reads one)
    #[arg(long = "match", value_name = "REGEX", requires = "in_file")]
    pattern: Option<String>,

    /// With --match: the one file whose lines are matched, never a directory (from the current directory, in the
    /// project's roots)
    #[arg(long = "in", value_name = "FILE", requires = "pattern")]
    in_file: Option<PathBuf>,

    #[command(flatten)]
    verb: VerbArgs,
}

impl WaitArgs {
    /// The exit code: 124 when the time ran out before a condition held.
    pub(crate) fn run(self, terminal: &Terminal, ports: &Ports) -> i32 {
        let mut deadline = false;
        let exit = self.verb.answered(
            terminal,
            ports,
            |verbs| {
                let wait = Wait {
                    for_: self
                        .for_
                        .or(self.retry_after)
                        .expect("clap asks for --for or --retry-after"),
                    url: self.url.clone(),
                    changed: self.changed.clone().map(|glob| Changed {
                        glob,
                        since: self.since.map(UnixMillis),
                    }),
                    matching: self
                        .pattern
                        .clone()
                        .zip(self.in_file.clone())
                        .map(|(pattern, file)| Matching { pattern, file }),
                    mode: self.verb.mode(),
                };
                let answered =
                    waited::wait(&verbs, &wait, Poll::BACK_OFF, &MonotonicTimer::start())?;
                deadline = answered.answer.reason == Reason::Deadline;
                Ok(answered)
            },
            |terminal, answer: &WaitAnswer| {
                outln!(
                    terminal,
                    "{} after {} ms ({} checks)",
                    answer.reason.name(),
                    answer.waited_ms,
                    answer.polls
                );
            },
        );
        match exit {
            Exit::Passed if deadline => 124,
            exit => exit.code(),
        }
    }
}

/// A duration as `--for` reads one, at most [`waited::LONGEST`].
fn parse_for(text: &str) -> Result<Duration, String> {
    let span = super::parse_duration(text)?;
    if span > waited::LONGEST {
        return Err(format!(
            "{text:?} is longer than a wait may be: 24h the most"
        ));
    }
    Ok(span)
}

/// A `Retry-After` value as `--retry-after` reads one, counted from now.
fn parse_retry_after(text: &str) -> Result<Duration, String> {
    waited::retry_after(text, std::time::SystemTime::now())
}
