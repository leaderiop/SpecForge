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
    /// An optional entity of the kind, positional: the prompt's subject, which the handler may pick when it is absent
    /// (ADR 0314).
    Subject(&'static str),
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
    /// `--name N`: a positive integer, or this word (`--jobs auto`). The host takes it as text; [`values`] refuses any
    /// other value before the render.
    Count(&'static str),
    /// The free-text tail: bare words after the options, as upstream's `[task description]` (ADR 0314).
    Rest,
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
    /// An optional positional subject: absent, the handler picks it (`/gsd-plan-phase`).
    pub const fn subject(name: &'static str, kind: &'static str) -> Arg {
        Arg {
            name,
            kind: Kind::Subject(kind),
            required: false,
        }
    }
    /// An optional reference, written `--name <id>` with its full id (`--phase v2.0/01`): the milestone is part of
    /// what the person named.
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
    /// The free-text tail of the command line, written bare and last.
    pub const fn rest(name: &'static str) -> Arg {
        Arg {
            name,
            kind: Kind::Rest,
            required: false,
        }
    }
    /// A positive integer or `word`, written `--name <N|word>`.
    pub const fn count_or(name: &'static str, word: &'static str) -> Arg {
        Arg {
            name,
            kind: Kind::Count(word),
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
                Kind::Reference(kind) | Kind::Subject(kind) | Kind::Bound(kind) => {
                    a.reference(kind)
                }
                Kind::Flag => a.bool(),
                Kind::Text | Kind::Count(_) => a.string(),
                Kind::Integer => a.integer(),
                Kind::OneOf(values) => a.one_of(values),
                Kind::Rest => a.rest(),
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

/// `value` in double quotes, for a free-text option. Not `shell::in_double_quotes`: the argument text is data the
/// agent reads, never shell (ADR 0340 D6), so only the quote and the backslash that delimit the value are escaped.
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
            (Kind::Subject(_), Value::String(path)) => words.push(written(path).to_string()),
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
            (Kind::OneOf(_) | Kind::Count(_), Value::String(word)) => {
                words.push(format!("{option} {word}"));
            }
            // Bare, where the argument stands in the table: quick's is first, so `resume abc` still opens the block.
            (Kind::Rest, Value::String(text)) if !text.is_empty() => words.push(text.clone()),
            _ => {}
        }
    }
    words.join(" ")
}

/// The argument text of a prompt call.
pub fn text(call: &PromptCall<'_>, args: &[Arg]) -> String {
    render_text(args, |name| call.arg(name).cloned())
}

/// The value an `{{arg:<name>|…}}` placeholder takes for `value` (ADR 0340 D2), unescaped: an entity's full path, a
/// bound's last segment, `true`/`false`, a decimal, or the string; an empty string is absent.
pub fn unescaped_value(arg: &Arg, value: &Value) -> Option<String> {
    match (&arg.kind, value) {
        (Kind::Bound(_), Value::String(path)) => Some(written(path).to_string()),
        (_, Value::String(text)) if text.is_empty() => None,
        (_, Value::String(text)) => Some(text.clone()),
        (Kind::Flag, Value::Bool(on)) => Some(on.to_string()),
        (Kind::Integer, Value::Number(n)) => Some(n.to_string()),
        _ => None,
    }
}

/// Every argument a prompt declares, with the value its `{{arg:<name>|…}}` placeholder takes (unescaped), or `None` when
/// it was not given.
pub struct Values(Vec<(String, Option<String>)>);

impl Values {
    /// No declared argument.
    pub const NONE: Values = Values(Vec::new());

    /// The value of `name`: `None` when the prompt does not declare it, `Some(None)` when it was not given.
    pub fn get(&self, name: &str) -> Option<Option<&str>> {
        self.0
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, value)| value.as_deref())
    }
}

impl<N: Into<String>> FromIterator<(N, Option<String>)> for Values {
    fn from_iter<I: IntoIterator<Item = (N, Option<String>)>>(iter: I) -> Values {
        Values(iter.into_iter().map(|(n, v)| (n.into(), v)).collect())
    }
}

/// Why `value` is not one `arg` takes, beyond what the host checks: a count is a positive integer or its word. Worded
/// as the host words a refused value.
pub fn refusal(arg: &Arg, value: &Value) -> Option<String> {
    let Kind::Count(word) = arg.kind else {
        return None;
    };
    let positive =
        |n: &str| !n.is_empty() && !n.starts_with('0') && n.bytes().all(|b| b.is_ascii_digit());
    match value {
        Value::String(v) if v == word || positive(v) => None,
        Value::String(v) => Some(format!(
            "{} must be {word} or an integer of at least 1, got '{v}'",
            arg.name
        )),
        other => Some(format!(
            "{} must be {word} or an integer of at least 1, got {other}",
            arg.name
        )),
    }
}

/// Every declared name of `args` with the value its placeholder takes in `call`, or `None` when it was not given. The
/// subject `(name, id)` the run is about, given or picked by the handler, takes its id. A value [`refusal`] refuses
/// fails the call before the render.
pub fn values(
    call: &PromptCall<'_>,
    args: &[Arg],
    subject: Option<(&str, &str)>,
) -> Result<Values, String> {
    args.iter()
        .map(|arg| {
            let given = call.arg(arg.name);
            if let Some(why) = given.and_then(|v| refusal(arg, v)) {
                return Err(why);
            }
            let value = match subject {
                Some((name, id)) if name == arg.name => Some(id.to_string()),
                _ => given.and_then(|v| unescaped_value(arg, v)),
            };
            Ok((arg.name, value))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{Arg, refusal, unescaped_value};

    #[test]
    fn a_value_by_kind() {
        let value = |arg: Arg, v| unescaped_value(&arg, &v);
        assert_eq!(
            value(Arg::reference("phase", "phase"), json!("v1.0/02")).as_deref(),
            Some("v1.0/02")
        );
        assert_eq!(
            value(Arg::subject("phase", "phase"), json!("v1.0/02")).as_deref(),
            Some("v1.0/02")
        );
        assert_eq!(
            value(Arg::bound("from", "phase"), json!("v1.0/02")).as_deref(),
            Some("02")
        );
        assert_eq!(
            value(Arg::flag("tdd"), json!(true)).as_deref(),
            Some("true")
        );
        assert_eq!(
            value(Arg::flag("tdd"), json!(false)).as_deref(),
            Some("false")
        );
        assert_eq!(
            value(Arg::integer("max_cycles"), json!(3)).as_deref(),
            Some("3")
        );
        assert_eq!(
            value(Arg::text("name"), json!("Bob's app")).as_deref(),
            Some("Bob's app")
        );
        assert_eq!(value(Arg::text("name"), json!("")), None);
        assert_eq!(value(Arg::rest("task"), json!("")), None);
        assert_eq!(
            value(Arg::one_of("mode", &["a", "b"]), json!("b")).as_deref(),
            Some("b")
        );
    }

    #[test]
    fn a_count_is_a_positive_integer_or_its_word() {
        let jobs = Arg::count_or("jobs", "auto");
        for ok in ["auto", "1", "12", "100"] {
            assert_eq!(refusal(&jobs, &json!(ok)), None, "{ok}");
        }
        for bad in ["0", "07", "-1", "+3", "2x", "", "auto; touch X", "AUTO"] {
            assert_eq!(
                refusal(&jobs, &json!(bad)).as_deref(),
                Some(
                    format!("jobs must be auto or an integer of at least 1, got '{bad}'").as_str()
                ),
                "{bad}"
            );
        }
        assert_eq!(refusal(&Arg::text("name"), &json!("0")), None);
    }
}
