//! Typed tool arguments: each core tool reads its `arguments` into one
//! `Args` struct, and [`fields`] recovers that struct's field names, so a
//! test can hold every input schema to exactly what its handler reads.

use serde::Deserialize;
use serde::de::{self, DeserializeOwned, Deserializer, Visitor};
use serde_json::Value;

use crate::tool::ToolOutcome;

/// `arguments` read as `A`. A missing required argument is refused with
/// `Missing required parameter: <name>`; anything else serde rejects, with
/// its reason. Arguments that are not an object read as none.
pub fn parse<A: DeserializeOwned>(arguments: Value) -> Result<A, ToolOutcome> {
    let arguments = match arguments {
        object @ Value::Object(_) => object,
        _ => Value::Object(Default::default()),
    };
    serde_json::from_value(arguments).map_err(|error| ToolOutcome::invalid_params(reason(&error)))
}

/// Why serde refused the arguments, as the tool reports it.
fn reason(error: &serde_json::Error) -> String {
    let message = error.to_string();
    match message
        .strip_prefix("missing field `")
        .and_then(|rest| rest.split('`').next())
    {
        Some(field) => format!("Missing required parameter: {field}"),
        None => format!("Invalid arguments: {message}"),
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
