//! A prompt's arguments (ADR 0291): read by each argument's field type, admitted by the field form every other value
//! of that type is admitted by, and sent to the export as typed JSON. A reference is resolved by the graph and sent as
//! its full path. Both surfaces read through here: MCP's named arguments (every value a string, or any JSON value for
//! `specforge.prompt`) and the command line's words.

use serde_json::{Map, Value, json};
use specforge_graph::{FieldForm, Found};
use specforge_protocol_types::command_args::option_name;
use specforge_protocol_types::{FieldType, PromptArgDescriptor};

use crate::view::{self, ProjectView};
use crate::{OpError, OpErrorKind};

/// The arguments a caller gave.
pub enum PromptArgs<'a> {
    /// By declared name.
    Named(&'a Map<String, Value>),
    /// The command line's words, or a `$ARGUMENTS` string split by [`words`]: a required argument that is not a flag
    /// is positional, in declaration order; the others are `--<name> <value>`; a bool is `--<name>`.
    Words(&'a [String]),
}

/// `text` as words: split at spaces, a double-quoted run is one word.
pub fn words(text: &str) -> Vec<String> {
    let (mut out, mut word, mut quoted, mut started) = (Vec::new(), String::new(), false, false);
    for c in text.chars() {
        match c {
            '"' => {
                quoted = !quoted;
                started = true;
            }
            c if c.is_whitespace() && !quoted => {
                if started {
                    out.push(std::mem::take(&mut word));
                    started = false;
                }
            }
            c => {
                word.push(c);
                started = true;
            }
        }
    }
    if started {
        out.push(word);
    }
    out
}

fn bad(name: &str, message: impl std::fmt::Display) -> OpError {
    OpError::new(
        OpErrorKind::InvalidInput,
        "invalid_input",
        format!("argument '{name}': {message}"),
    )
    .with_data(json!({ "argument": name }))
}

fn field_type(arg: &PromptArgDescriptor) -> FieldType {
    FieldType::parse(&arg.field_type).unwrap_or(FieldType::String)
}

/// The words, by the name of the argument each gives.
fn named(args: &[PromptArgDescriptor], words: &[String]) -> Result<Map<String, Value>, OpError> {
    let positional: Vec<&PromptArgDescriptor> = args
        .iter()
        .filter(|a| a.required && field_type(a) != FieldType::Bool)
        .collect();
    let (mut given, mut next) = (Map::new(), 0);
    let mut words = words.iter();
    while let Some(word) = words.next() {
        let Some(long) = word.strip_prefix("--") else {
            let arg = positional
                .get(next)
                .ok_or_else(|| bad(word, "no positional argument takes this word"))?;
            next += 1;
            given.insert(arg.name.clone(), Value::String(word.clone()));
            continue;
        };
        let arg = args
            .iter()
            .find(|a| option_name(&a.name) == long)
            .ok_or_else(|| bad(long, "no such option"))?;
        let value = if field_type(arg) == FieldType::Bool {
            Value::Bool(true)
        } else {
            let value = words
                .next()
                .ok_or_else(|| bad(&arg.name, "the option takes a value"))?;
            Value::String(value.clone())
        };
        if given.insert(arg.name.clone(), value).is_some() {
            return Err(bad(&arg.name, "given twice"));
        }
    }
    Ok(given)
}

/// `value` as a `field_type` argument: a string is read as the type's written form (`"2"` an integer, `"true"` a
/// bool, `"a,b"` a list); a JSON value of the type is itself. Anything else is left for the form to refuse.
fn typed(field_type: FieldType, value: &Value) -> Value {
    let Value::String(text) = value else {
        return value.clone();
    };
    match field_type {
        FieldType::Integer => text
            .trim()
            .parse::<i64>()
            .map_or_else(|_| value.clone(), Value::from),
        FieldType::Bool => match text.as_str() {
            "true" => Value::Bool(true),
            "false" => Value::Bool(false),
            _ => value.clone(),
        },
        t if t.is_list() => Value::Array(
            text.split(',')
                .map(str::trim)
                .filter(|item| !item.is_empty())
                .map(|item| Value::String(item.to_string()))
                .collect(),
        ),
        _ => value.clone(),
    }
}

/// `spelling` as the full path of an entity of one of `kinds`.
fn resolved(
    view: &ProjectView<'_>,
    arg: &PromptArgDescriptor,
    kinds: &[String],
    spelling: &str,
) -> Result<String, OpError> {
    let around = |e: OpError| {
        let data = json!({ "argument": arg.name });
        e.prefixed(format!("argument '{}': ", arg.name))
            .with_data(data)
    };
    let path = match view.graph().find(spelling) {
        Found::One(path) => path.to_string(),
        Found::Ambiguous(paths) => return Err(around(view::ambiguous(spelling, &paths))),
        Found::Mistyped(_) | Found::None(_) => {
            return Err(around(view::not_found(view.graph(), spelling)));
        }
    };
    let kind = view.graph().node(&path).map(|n| n.kind.raw.to_string());
    match kind {
        Some(kind) if !kinds.is_empty() && !kinds.contains(&kind) => Err(bad(
            &arg.name,
            format!("'{spelling}' is a {kind}, not a {}", kinds.join(" or ")),
        )),
        _ => Ok(path),
    }
}

/// The arguments the export is sent: each declared argument given or defaulted, read by its type, admitted by its
/// form, a reference as its full path. An undeclared name, a value its form refuses and a missing required argument
/// are `invalid_input` about that argument; an unknown entity is `entity_not_found`. An absent optional argument is
/// absent.
pub fn read(
    view: &ProjectView<'_>,
    declared: &[PromptArgDescriptor],
    given: PromptArgs<'_>,
) -> Result<Map<String, Value>, OpError> {
    let given = match given {
        PromptArgs::Named(map) => map.clone(),
        PromptArgs::Words(words) => named(declared, words)?,
    };
    if let Some(unknown) = given
        .keys()
        .find(|k| !declared.iter().any(|a| a.name == **k))
    {
        return Err(bad(unknown, "the prompt declares no such argument"));
    }
    let mut sent = Map::new();
    for arg in declared {
        let Some(value) = given.get(&arg.name).or(arg.default.as_ref()) else {
            if arg.required {
                return Err(bad(&arg.name, "it is required"));
            }
            continue;
        };
        let kind = field_type(arg);
        let kinds: Vec<String> = arg
            .target_kind
            .iter()
            .flat_map(|k| k.split('|').map(|k| k.trim().to_string()))
            .collect();
        let form = FieldForm::new(kind, arg.enum_values.clone())
            .targeting_any(&kinds)
            .bounded(arg.minimum, arg.maximum);
        let value = typed(kind, value);
        form.admit_json(&value).map_err(|r| {
            bad(
                &arg.name,
                r.suggestion
                    .unwrap_or_else(|| format!("not a {}", arg.field_type)),
            )
        })?;
        let value = match (&value, kind) {
            (Value::String(s), FieldType::Reference) => {
                Value::String(resolved(view, arg, &kinds, s)?)
            }
            (Value::Array(items), FieldType::ReferenceList) => Value::Array(
                items
                    .iter()
                    .filter_map(Value::as_str)
                    .map(|s| resolved(view, arg, &kinds, s).map(Value::String))
                    .collect::<Result<_, _>>()?,
            ),
            _ => value,
        };
        sent.insert(arg.name.clone(), value);
    }
    Ok(sent)
}
