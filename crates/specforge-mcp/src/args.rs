//! Typed tool arguments: each core tool reads its `arguments` into one
//! `Args` struct, and [`fields`] recovers that struct's field names, so a
//! test can hold every input schema to exactly what its handler reads.

use serde::Deserialize;
use serde::de::{self, DeserializeOwned, Deserializer, Visitor};
use serde_json::Value;
use specforge_ops::options::OptionTable;

use crate::tool::{ErrorCode, McpError, ToolOutcome};

/// `arguments` (an object; the dispatcher refuses any other) read as `A`.
/// A failure is invalid input, an `isError` result (ADR 0004 D4-a): a
/// missing required argument says `Missing required parameter: <name>`
/// and names it; anything else serde rejects says why.
pub fn parse<A: DeserializeOwned>(arguments: Value) -> Result<A, ToolOutcome> {
    parse_args(arguments).map_err(ToolOutcome::Refused)
}

/// [`parse`], the refusal as the `McpError` itself (boxed, as
/// [`ToolOutcome::Refused`] holds it): what a prompt answers with, a
/// JSON-RPC error carrying it (prompts have no `isError`).
pub fn parse_args<A: DeserializeOwned>(arguments: Value) -> Result<A, Box<McpError>> {
    serde_json::from_value(arguments).map_err(|error| Box::new(refusal(&error)))
}

/// Why serde refused the arguments, as the tool reports it.
fn refusal(error: &serde_json::Error) -> McpError {
    let message = error.to_string();
    match message
        .strip_prefix("missing field `")
        .and_then(|rest| rest.split('`').next())
    {
        Some(field) => McpError::new(
            ErrorCode::InvalidInput,
            format!("Missing required parameter: {field}"),
        )
        .with_argument(field),
        None => McpError::new(
            ErrorCode::InvalidInput,
            format!("Invalid arguments: {message}"),
        ),
    }
}

/// The field names `A` reads, as serde's derive hands them to
/// `deserialize_struct`.
pub fn fields<A: DeserializeOwned>() -> &'static [&'static str] {
    match A::deserialize(FieldTracer) {
        Err(Traced(Some(fields))) => fields,
        _ => &[],
    }
}

/// The fields `A` cannot be read without, in field order. serde names the
/// first missing field of a struct; the probe gives it an empty string and
/// reads again, until the struct reads (or refuses for another reason).
/// Every argument of a prompt is a string (MCP sends them as strings), so
/// an empty string is a value each required field accepts.
pub fn required<A: DeserializeOwned>() -> Vec<&'static str> {
    let fields = fields::<A>();
    let mut probe = serde_json::Map::new();
    let mut required = Vec::new();
    while let Err(error) = serde_json::from_value::<A>(Value::Object(probe.clone())) {
        let message = error.to_string();
        let Some(missing) = message
            .strip_prefix("missing field `")
            .and_then(|rest| rest.split('`').next())
            .and_then(|name| fields.iter().find(|field| **field == name))
        else {
            break;
        };
        if probe.contains_key(*missing) {
            break;
        }
        probe.insert((*missing).to_string(), Value::from(""));
        required.push(*missing);
    }
    let order = |name: &&str| fields.iter().position(|field| field == name);
    required.sort_by_key(order);
    required
}

/// A count: a non-negative integer, or a string holding one (MCP sends
/// prompt arguments as strings). Anything else is refused.
pub fn count<'de, D: Deserializer<'de>>(deserializer: D) -> Result<usize, D::Error> {
    match Value::deserialize(deserializer)? {
        Value::Number(number) => number
            .as_u64()
            .and_then(|n| usize::try_from(n).ok())
            .ok_or_else(|| {
                de::Error::custom(format!(
                    "invalid count {number}, expected a non-negative integer"
                ))
            }),
        Value::String(text) => text.trim().parse::<usize>().map_err(|_| {
            de::Error::custom(format!(
                "invalid count \"{text}\", expected a non-negative integer"
            ))
        }),
        other => Err(de::Error::custom(format!(
            "invalid type: {}, expected a non-negative integer",
            kind_of(&other)
        ))),
    }
}

/// [`count`], absent or `null` read as none.
pub fn some_count<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Option<usize>, D::Error> {
    match Value::deserialize(deserializer)? {
        Value::Null => Ok(None),
        value => count(value).map(Some).map_err(de::Error::custom),
    }
}

/// Entity ids: a list of strings, or one comma-separated string (MCP sends
/// prompt arguments as strings); blank entries are dropped. Anything else
/// is refused.
pub fn id_list<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<String>, D::Error> {
    match Value::deserialize(deserializer)? {
        Value::Null => Ok(Vec::new()),
        Value::String(ids) => Ok(ids
            .split(',')
            .map(str::trim)
            .filter(|id| !id.is_empty())
            .map(String::from)
            .collect()),
        Value::Array(items) => items
            .into_iter()
            .map(|item| match item {
                Value::String(id) => Ok(id),
                other => Err(de::Error::custom(format!(
                    "invalid type: {}, expected an entity id string",
                    kind_of(&other)
                ))),
            })
            .collect(),
        other => Err(de::Error::custom(format!(
            "invalid type: {}, expected a list of entity ids or a comma-separated string",
            kind_of(&other)
        ))),
    }
}

/// How serde names a JSON value's type in its messages.
fn kind_of(value: &Value) -> String {
    match value {
        Value::Null => "null".into(),
        Value::Bool(b) => format!("boolean `{b}`"),
        Value::Number(n) if n.is_f64() => format!("floating point `{n}`"),
        Value::Number(n) => format!("integer `{n}`"),
        Value::String(s) => format!("string \"{s}\""),
        Value::Array(_) => "sequence".into(),
        Value::Object(_) => "map".into(),
    }
}

/// A deserializer that reads nothing: it records the field names a struct
/// asks for and stops.
struct FieldTracer;

#[derive(Debug)]
struct Traced(Option<&'static [&'static str]>);

impl std::fmt::Display for Traced {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("field tracer")
    }
}

impl std::error::Error for Traced {}

impl de::Error for Traced {
    fn custom<T: std::fmt::Display>(_: T) -> Self {
        Traced(None)
    }
}

impl<'de> Deserializer<'de> for FieldTracer {
    type Error = Traced;

    fn deserialize_any<V: Visitor<'de>>(self, _: V) -> Result<V::Value, Traced> {
        Err(Traced(None))
    }

    fn deserialize_struct<V: Visitor<'de>>(
        self,
        _: &'static str,
        fields: &'static [&'static str],
        _: V,
    ) -> Result<V::Value, Traced> {
        Err(Traced(Some(fields)))
    }

    serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        bytes byte_buf option unit unit_struct newtype_struct seq tuple
        tuple_struct map enum identifier ignored_any
    }
}

/// An optional argument of the wrong type reads as absent, as the handlers
/// have always read one.
pub fn lenient<'de, D: Deserializer<'de>, T: DeserializeOwned>(
    deserializer: D,
) -> Result<Option<T>, D::Error> {
    Ok(serde_json::from_value(Value::deserialize(deserializer)?).ok())
}

/// An enumerated argument's input schema (ADR 0027): `type` string, `enum`
/// every name the table accepts (listed names, then aliases, so a
/// validating client may send an alias), `default` the table's (none for a
/// filter), `description` followed by each choice and its help
/// (`Output format: markdown; mermaid: ER diagram; …`).
pub fn choice_schema<T: Copy + PartialEq>(table: &OptionTable<T>, description: &str) -> Value {
    let mut schema = required_choice_schema(table, description);
    if let Some(default) = table.default_name() {
        schema["default"] = Value::from(default);
    }
    schema
}

/// [`choice_schema`] of a required argument: no `default`.
pub fn required_choice_schema<T: Copy + PartialEq>(
    table: &OptionTable<T>,
    description: &str,
) -> Value {
    let choices: Vec<String> = table
        .choices
        .iter()
        .map(|choice| {
            let mut text = choice.name.to_string();
            if !choice.aliases.is_empty() {
                text.push_str(&format!(" (also {})", choice.aliases.join(", ")));
            }
            if !choice.help.is_empty() {
                text.push_str(": ");
                text.push_str(choice.help);
            }
            text
        })
        .collect();
    serde_json::json!({
        "type": "string",
        "enum": table.accepted().collect::<Vec<_>>(),
        "description": format!("{description}: {}", choices.join("; ")),
    })
}

/// The input schema of a name list that is not an option table (severity
/// and lint profiles, ADR 0018: their typed `CheckError` refusals stay):
/// `type` string, `enum` the names.
pub fn names_schema(names: &[&str], description: &str) -> Value {
    serde_json::json!({
        "type": "string",
        "enum": names,
        "description": description,
    })
}

/// An enumerated argument as a handler reads it: absent is the table's
/// default; an unknown name is the table's refusal, `invalid_input` on
/// `key` (ADR 0027).
///
/// # Panics
/// When the table has no default: a filter is read with
/// [`optional_choice`].
pub fn choice<T: Copy + PartialEq>(
    table: &OptionTable<T>,
    key: &str,
    name: Option<&str>,
) -> Result<T, ToolOutcome> {
    table
        .parse_or_default(name)
        .map_err(|error| refused(error, key))
}

/// [`choice`] of a table without a default (a filter): absent is none.
pub fn optional_choice<T: Copy + PartialEq>(
    table: &OptionTable<T>,
    key: &str,
    name: Option<&str>,
) -> Result<Option<T>, ToolOutcome> {
    table
        .parse_optional(name)
        .map_err(|error| refused(error, key))
}

/// A table's refusal as the tool's `invalid_input` result on `key`.
fn refused(error: specforge_ops::OpError, key: &str) -> ToolOutcome {
    crate::operations::op_error(error).with_argument(key).into()
}

/// A list of strings, its other items skipped; anything but a list reads
/// as none.
pub fn strings<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<String>, D::Error> {
    Ok(some_strings(deserializer)?.unwrap_or_default())
}

/// [`strings`], keeping whether a list was given at all.
pub fn some_strings<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<Vec<String>>, D::Error> {
    let value = Value::deserialize(deserializer)?;
    Ok(value.as_array().map(|items| {
        items
            .iter()
            .filter_map(|item| item.as_str().map(String::from))
            .collect()
    }))
}

/// The arguments of a tool that takes none.
#[derive(Debug, Default, Deserialize)]
pub struct NoArgs {}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[derive(Debug, Deserialize)]
    #[allow(dead_code, reason = "read by the probe only")]
    struct Probe {
        first: String,
        #[serde(default)]
        defaulted: Option<String>,
        second: String,
    }

    #[test]
    fn required_names_each_field_the_struct_cannot_read_without() {
        assert_eq!(required::<Probe>(), ["first", "second"]);
        assert!(required::<NoArgs>().is_empty());
    }

    #[derive(Debug, Deserialize)]
    struct Counted {
        #[serde(deserialize_with = "count")]
        n: usize,
    }

    fn counted(n: Value) -> Result<usize, Box<McpError>> {
        parse_args::<Counted>(json!({ "n": n })).map(|c| c.n)
    }

    #[test]
    fn a_count_is_an_integer_or_a_string_holding_one() {
        assert_eq!(counted(json!(2)).unwrap(), 2);
        assert_eq!(counted(json!("2")).unwrap(), 2);
        for refused in [
            json!("two"),
            json!(-1),
            json!("-1"),
            json!(1.5),
            json!(true),
        ] {
            let error = counted(refused.clone()).unwrap_err();
            assert_eq!(error.code, ErrorCode::InvalidInput, "{refused}");
            assert!(
                error.message.starts_with("Invalid arguments: "),
                "{refused}: {}",
                error.message
            );
        }
    }

    #[derive(Debug, Deserialize)]
    struct MaybeCounted {
        #[serde(default, deserialize_with = "some_count")]
        n: Option<usize>,
    }

    #[test]
    fn an_absent_count_is_none() {
        let read = |args: Value| parse_args::<MaybeCounted>(args).map(|c| c.n);
        assert_eq!(read(json!({})).unwrap(), None);
        assert_eq!(read(json!({ "n": null })).unwrap(), None);
        assert_eq!(read(json!({ "n": "3" })).unwrap(), Some(3));
        assert!(read(json!({ "n": "three" })).is_err());
    }

    #[derive(Debug, Deserialize)]
    struct Ids {
        #[serde(default, deserialize_with = "id_list")]
        ids: Vec<String>,
    }

    #[test]
    fn an_id_list_is_a_list_or_a_comma_separated_string() {
        let read = |args: Value| parse_args::<Ids>(args).map(|i| i.ids);
        assert_eq!(read(json!({ "ids": ["a", "b"] })).unwrap(), ["a", "b"]);
        assert_eq!(read(json!({ "ids": "a, b,,c " })).unwrap(), ["a", "b", "c"]);
        assert!(read(json!({})).unwrap().is_empty());
        assert!(read(json!({ "ids": 4 })).is_err());
        assert!(read(json!({ "ids": ["a", 4] })).is_err());
    }

    #[test]
    fn a_missing_required_field_is_named() {
        let error = parse_args::<Probe>(json!({ "second": "x" })).unwrap_err();
        assert_eq!(error.message, "Missing required parameter: first");
        assert_eq!(error.argument.as_deref(), Some("first"));
        let error = parse_args::<Probe>(json!({ "first": 42, "second": "x" })).unwrap_err();
        assert_eq!(
            error.message,
            "Invalid arguments: invalid type: integer `42`, expected a string"
        );
    }
}
