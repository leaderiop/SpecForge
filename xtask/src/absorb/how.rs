//! The value transforms of the table's homes, each pure: an old value (as `field_form::write` gives it) in, the
//! typed edit's input out.

use serde_json::{Value, json};
use specforge_ops::authoring::edit::Input;
use specforge_parser::lex::{is_entity_id, is_identifier};
use specforge_parser::{CmpOp, Expr, ExprSpan, parse_expression};
use std::collections::BTreeMap;

use super::table::{Compare, How};

/// Why a scheme reference in a list of texts is not written: the writer spells it bare, which names a `ref`.
pub const SCHEME_REF: &str =
    "a scheme reference is written bare and names a `ref`, which a typed edit cannot create";

/// `value` as written: a text or a date as one value, a list as its items, a number or a boolean as itself.
pub fn as_written(value: &Value) -> Input {
    match value {
        Value::String(text) => Input::One(text.clone()),
        Value::Array(_) => Input::Many(items(value)),
        Value::Number(n) => Input::Number(n.clone()),
        Value::Bool(b) => Input::Bool(*b),
        other => Input::One(other.to_string()),
    }
}

/// The items of a list, or a single value as one item.
pub fn items(value: &Value) -> Vec<String> {
    match value {
        Value::Array(items) => items.iter().map(text).collect(),
        Value::Null => Vec::new(),
        other => vec![text(other)],
    }
}

fn text(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        other => other.to_string(),
    }
}

/// Whether a list item is a scheme reference id (`gh.issue:42`): see [`SCHEME_REF`].
pub fn is_scheme_ref(item: &str) -> bool {
    item.contains(':') && is_entity_id(item)
}

/// The input a word-mapping `how` writes for `value`; none when the word maps to nothing.
pub fn word(how: How, value: &Value) -> Option<Input> {
    let word = value.as_str().unwrap_or_default();
    match how {
        How::Outcome => match word {
            "proposed" => Some(Input::One("pending".to_string())),
            "deprecated" => Some(Input::One("revisit".to_string())),
            _ => None,
        },
        How::Settled => (word == "accepted").then_some(Input::Bool(true)),
        How::Options => {
            let options: Vec<BTreeMap<String, Input>> = items(value)
                .into_iter()
                .map(|name| BTreeMap::from([("name".to_string(), Input::One(name))]))
                .collect();
            (!options.is_empty()).then_some(Input::Blocks(options))
        }
        _ => None,
    }
}

/// A mapped word: the word written, or why none is, and whether the old one is collapsed into it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Mapped {
    Same(&'static str),
    Collapsed(&'static str),
    Unknown(String),
}

/// The quality classes a requirement's `category` takes.
const CATEGORIES: [&str; 10] = [
    "functional",
    "performance",
    "compatibility",
    "usability",
    "reliability",
    "security",
    "maintainability",
    "portability",
    "safety",
    "regulatory",
];

/// An old constraint category as a requirement's quality class.
pub fn category(word: &str) -> Mapped {
    match CATEGORIES.iter().find(|c| **c == word) {
        Some(class) => Mapped::Same(class),
        None if word == "interoperability" => Mapped::Collapsed("compatibility"),
        None => Mapped::Unknown(format!(
            "'{word}' is none of the ten quality classes ({})",
            CATEGORIES.join(", ")
        )),
    }
}

/// An old constraint priority as a requirement's MoSCoW word.
pub fn priority(word: &str) -> Mapped {
    match word {
        "critical" => Mapped::Same("must"),
        "high" => Mapped::Same("should"),
        "medium" | "low" => Mapped::Collapsed("could"),
        other => Mapped::Unknown(format!("'{other}' is no priority")),
    }
}

/// What an old `metric` (and `threshold`) writes: the metric record, if one line bounds it; the lines that join the
/// text; the direction a strict bound collapsed into; why no record is written; whether the threshold was taken.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Metric {
    pub record: Option<BTreeMap<String, String>>,
    pub text: Vec<String>,
    pub strict: Option<&'static str>,
    pub unconverted: Option<String>,
    pub threshold_taken: bool,
}

/// One line of a metric, as the prove pass reads it.
enum Line {
    /// `<name> <op> <number>[unit]`: the record, and its direction when the operator was strict.
    Bound(BTreeMap<String, String>, Option<&'static str>),
    NotEqual,
    Prose,
}

/// Read an old metric with the prove pass's reader: exactly one line `<name> <op> <number>[unit]` is the record,
/// its threshold the number as written (`95.0ms` gives `95.0`); the other lines join the text. A one-name metric
/// and a `threshold` that is an operator and a number make that line. No bound line, several, or a `!=` write
/// every line into the text.
pub fn metric(metric: &str, threshold: Option<&str>) -> Metric {
    let mut lines: Vec<String> = metric
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(String::from)
        .collect();
    let mut out = Metric::default();
    if let (Some(threshold), [name]) = (threshold, lines.as_slice())
        && is_identifier(name)
    {
        let line = format!("{name} {}", threshold.trim());
        if matches!(read_line(&line), Line::Bound(..)) {
            lines = vec![line];
            out.threshold_taken = true;
        }
    }
    let read: Vec<Line> = lines.iter().map(|line| read_line(line)).collect();
    let bounds = read.iter().filter(|l| matches!(l, Line::Bound(..))).count();
    let not_equal = read.iter().any(|l| matches!(l, Line::NotEqual));
    if bounds == 1 && !not_equal {
        let mut prose = Vec::new();
        for (line, read) in lines.iter().zip(read) {
            match read {
                Line::Bound(record, strict) => {
                    out.strict = strict;
                    out.record = Some(record);
                }
                _ => prose.push(line.clone()),
            }
        }
        if !prose.is_empty() {
            out.text.push(prose.join("\n"));
        }
    } else {
        if !lines.is_empty() {
            out.text.push(lines.join("\n"));
        }
        out.unconverted = Some(if not_equal {
            "a `!=` line bounds no direction; every line joins the text".to_string()
        } else if bounds == 0 {
            "no line is one bound `<name> <op> <number>`; every line joins the text".to_string()
        } else {
            format!("{bounds} bound lines, and a metric holds one; every line joins the text")
        });
    }
    if let (Some(threshold), false) = (threshold, out.threshold_taken) {
        out.text.push(format!("threshold {}", threshold.trim()));
    }
    out
}

fn read_line(line: &str) -> Line {
    let Ok(expr) = parse_expression(line) else {
        return Line::Prose;
    };
    let Expr::Cmp(op, lhs, rhs) = expr.expr else {
        return Line::Prose;
    };
    let (Expr::Var(name), Expr::Num(_, unit)) = (&lhs.expr, &rhs.expr) else {
        return Line::Prose;
    };
    let (direction, strict) = match op {
        CmpOp::Le => ("at_most", false),
        CmpOp::Lt => ("at_most", true),
        CmpOp::Ge => ("at_least", false),
        CmpOp::Gt => ("at_least", true),
        CmpOp::Eq => ("exactly", false),
        CmpOp::Ne => return Line::NotEqual,
    };
    let Some(written) = slice(line, rhs.span) else {
        return Line::Prose;
    };
    let threshold = written
        .strip_suffix(unit.as_str())
        .unwrap_or(written)
        .trim();
    let mut record = BTreeMap::from([
        ("name".to_string(), name.clone()),
        ("threshold".to_string(), threshold.to_string()),
        ("direction".to_string(), direction.to_string()),
    ]);
    if !unit.is_empty() {
        record.insert("unit".to_string(), unit.clone());
    }
    Line::Bound(record, strict.then_some(direction))
}

/// The text of a one-line span: 1-based byte columns, the end one past the last byte.
fn slice(line: &str, span: ExprSpan) -> Option<&str> {
    line.get(span.start_col.checked_sub(1)?..span.end_col.checked_sub(1)?)
}

/// What a computed value is checked against.
#[derive(Debug, Clone, PartialEq)]
pub enum Expect {
    /// The value the old one means.
    Value(Value),
    /// No new value equals it: always ledgered with the new one.
    Never,
}

/// The computed value the old `value` means under `compare`.
pub fn expected(compare: Compare, value: &Value) -> Expect {
    match compare {
        Compare::Superseded => Expect::Value(json!(value.as_str() != Some("superseded"))),
        Compare::Means(words) => words
            .iter()
            .find(|(old, _)| value.as_str() == Some(old))
            .map_or(Expect::Never, |(_, new)| Expect::Value(json!(new))),
    }
}

/// The tier a feature status gives its requirement.
pub fn feature_tier(status: &str) -> &'static str {
    match status {
        "proposed" | "deferred" => "v2",
        "deprecated" => "out_of_scope",
        _ => "v1",
    }
}

/// The old default status of a feature, which a feature that writes none has.
pub const FEATURE_DEFAULT: &str = "proposed";

/// An input as JSON, for the ledger.
pub fn input_json(input: &Input) -> Value {
    serde_json::to_value(input).unwrap_or(Value::Null)
}
