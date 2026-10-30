//! The typed arguments of a lifecycle prompt (D-r42-8). Upstream's argument hint becomes a table: each row declares one
//! argument of the prompt, and the same table rebuilds the argument text the ported workflow reads as `$ARGUMENTS`, in
//! upstream's order (`02 --research --granularity fine`).
//!
//! Also ports gsd-core bin/lib modules: command-arg-projection.cjs.

use serde_json::Value;
use specforge_extension_sdk::prelude::*;

/// What one argument holds.
pub enum Kind {
    /// An entity of the kind (several as `a|b`): positional, and the prompt's subject.
    Reference(&'static str),
    /// An entity of the kind, written `--name <number>`: a bound inside the milestone the run is about (`--from 02`),
    /// so only its last segment is shown.
    Bound(&'static str),
    /// `--name`, off unless given.
    Flag,
    /// `--name "text"`.
    Text,
    /// `--name 3`.
    Integer,
    /// `--name value`, one of these.
    OneOf(&'static [&'static str]),
}

pub struct Arg {
    pub name: &'static str,
    pub kind: Kind,
    pub required: bool,
}

impl Arg {
    /// A required reference: the phase, plan or milestone the prompt is about.
    pub const fn reference(name: &'static str, kind: &'static str) -> Arg {
        Arg {
            name,
            kind: Kind::Reference(kind),
            required: true,
        }
    }
    /// An optional reference, written `--name <id>` with its full id (`--phase v2.0/01`): the milestone is part of
    /// what the person named (REVIEW-r43 M1).
    pub const fn optional_reference(name: &'static str, kind: &'static str) -> Arg {
        Arg {
            name,
            kind: Kind::Reference(kind),
            required: false,
        }
    }
    /// An optional bound, written `--name <number>`: a phase of the milestone the run is about (`--from 02`).
    pub const fn bound(name: &'static str, kind: &'static str) -> Arg {
        Arg {
            name,
            kind: Kind::Bound(kind),
            required: false,
        }
    }
    pub const fn flag(name: &'static str) -> Arg {
        Arg {
            name,
            kind: Kind::Flag,
            required: false,
        }
    }
    pub const fn text(name: &'static str) -> Arg {
        Arg {
            name,
            kind: Kind::Text,
            required: false,
        }
    }
    pub const fn integer(name: &'static str) -> Arg {
        Arg {
            name,
            kind: Kind::Integer,
            required: false,
        }
    }
    pub const fn one_of(name: &'static str, values: &'static [&'static str]) -> Arg {
        Arg {
            name,
            kind: Kind::OneOf(values),
            required: false,
        }
    }
}

/// The spelling of an argument on a command line: `skip_research` is `--skip-research`.
pub fn option_name(name: &str) -> String {
    name.replace('_', "-")
}

/// Declares `args` on a prompt.
pub fn declare(p: &mut PromptBuilder, args: &[Arg]) {
    for arg in args {
        p.arg(arg.name, |a| {
            match &arg.kind {
                Kind::Reference(kind) | Kind::Bound(kind) => a.reference(kind),
                Kind::Flag => a.bool(),
                Kind::Text => a.string(),
                Kind::Integer => a.integer(),
                Kind::OneOf(values) => a.one_of(values),
            };
            if arg.required {
                a.required();
            }
        });
    }
}

/// The last segment of an entity path: the number or version a person writes (`v1.0/02` is `02`).
fn written(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

/// `value` in double quotes, for a free-text option.
fn quoted(value: &str) -> String {
    format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
}

/// The argument text for `values` (name -> JSON), in the order of `args`.
pub fn render_text(args: &[Arg], values: impl Fn(&str) -> Option<Value>) -> String {
    let mut words: Vec<String> = Vec::new();
    for arg in args {
        let Some(value) = values(arg.name) else {
            continue;
        };
        let option = format!("--{}", option_name(arg.name));
        match (&arg.kind, &value) {
            (Kind::Reference(_), Value::String(path)) if arg.required => {
                words.push(written(path).to_string());
            }
            (Kind::Reference(_), Value::String(path)) => words.push(format!("{option} {path}")),
            (Kind::Bound(_), Value::String(path)) => {
                words.push(format!("{option} {}", written(path)));
            }
            (Kind::Flag, Value::Bool(true)) => words.push(option),
            (Kind::Text, Value::String(text)) if !text.is_empty() => {
                words.push(format!("{option} {}", quoted(text)));
            }
            (Kind::Integer, Value::Number(n)) => words.push(format!("{option} {n}")),
            (Kind::OneOf(_), Value::String(word)) => words.push(format!("{option} {word}")),
            _ => {}
        }
    }
    words.join(" ")
}

/// The argument text of a prompt call.
pub fn text(call: &PromptCall<'_>, args: &[Arg]) -> String {
    render_text(args, |name| call.arg(name).cloned())
}
