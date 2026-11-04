//! One document tree for JSON, YAML and TOML: each parses to a `serde_json::Value`, so one pointer and one schema read
//! them all, and a YAML file means what its JSON twin does. The text must be UTF-8 (a byte order mark is dropped). TOML
//! datetimes become their RFC 3339 strings, and a YAML key that is no string is written as its JSON.
//!
//! Bounded, since a file is untrusted: nesting stops at [`MAX_DEPTH`], and YAML aliases at [`MAX_NODES`] nodes, so a
//! billion laughs costs a few kilobytes.

use std::collections::HashMap;

use serde_json::{Map, Number, Value};
use yaml_rust2::Yaml;
use yaml_rust2::parser::{Event, MarkedEventReceiver, Parser};
use yaml_rust2::scanner::{Marker, TScalarStyle};

/// The deepest a document nests.
pub const MAX_DEPTH: usize = 128;
/// The most nodes a YAML document holds once its aliases are read: a 1 MiB file holds fewer.
pub const MAX_NODES: usize = 1 << 20;

/// A format the tree reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Json,
    Yaml,
    Toml,
}

impl Format {
    /// `json`, `yaml`, `toml`.
    pub fn named(name: &str) -> Option<Format> {
        match name {
            "json" => Some(Format::Json),
            "yaml" => Some(Format::Yaml),
            "toml" => Some(Format::Toml),
            _ => None,
        }
    }

    /// The format of `path`, by its extension.
    pub fn of_path(path: &str) -> Option<Format> {
        let extension = path.rsplit_once('.')?.1.to_ascii_lowercase();
        match extension.as_str() {
            "json" => Some(Format::Json),
            "yaml" | "yml" => Some(Format::Yaml),
            "toml" => Some(Format::Toml),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Format::Json => "JSON",
            Format::Yaml => "YAML",
            Format::Toml => "TOML",
        }
    }
}

/// Why a document does not parse, and where, when the parser said (both 1-based).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    pub message: String,
    pub line: Option<usize>,
    pub column: Option<usize>,
}

impl ParseError {
    fn new(message: impl Into<String>) -> Self {
        ParseError {
            message: message.into(),
            line: None,
            column: None,
        }
    }

    fn at(message: impl Into<String>, line: usize, column: usize) -> Self {
        ParseError {
            message: message.into(),
            line: Some(line),
            column: Some(column),
        }
    }

    /// The error at byte `offset` of `text`.
    fn at_offset(message: impl Into<String>, text: &str, offset: usize) -> Self {
        let (line, column) = line_col(text, offset);
        ParseError::at(message, line, column)
    }
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match (self.line, self.column) {
            (Some(line), Some(column)) => write!(f, "{} at {line}:{column}", self.message),
            _ => f.write_str(&self.message),
        }
    }
}

/// The 1-based line and column (in characters) of byte `offset` of `text`.
fn line_col(text: &str, offset: usize) -> (usize, usize) {
    let mut offset = offset.min(text.len());
    while !text.is_char_boundary(offset) {
        offset -= 1;
    }
    let before = &text[..offset];
    let line = before.matches('\n').count() + 1;
    let start = before.rfind('\n').map_or(0, |at| at + 1);
    (line, before[start..].chars().count() + 1)
}

/// `bytes` read as `format`.
pub fn parse(format: Format, bytes: &[u8]) -> Result<Value, ParseError> {
    let text = match std::str::from_utf8(bytes) {
        Ok(text) => text,
        Err(error) => {
            let valid = std::str::from_utf8(&bytes[..error.valid_up_to()]).unwrap_or_default();
            let (line, column) = line_col(valid, valid.len());
            return Err(ParseError::at("is not UTF-8", line, column));
        }
    };
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let value = match format {
        Format::Json => serde_json::from_str(text).map_err(|error| {
            let message = error.to_string();
            let message = message.split(" at line ").next().unwrap_or(&message);
            ParseError::at(message, error.line(), error.column())
        })?,
        Format::Yaml => yaml(text)?,
        Format::Toml => {
            let table: toml::Table = toml::from_str(text).map_err(|error| {
                let message = error.message().to_string();
                match error.span() {
                    Some(span) => ParseError::at_offset(message, text, span.start),
                    None => ParseError::new(message),
                }
            })?;
            from_toml(toml::Value::Table(table))
        }
    };
    if depth(&value) > MAX_DEPTH {
        return Err(ParseError::new(format!(
            "nests deeper than {MAX_DEPTH} levels"
        )));
    }
    Ok(value)
}

/// How deep `value` nests, without recursing.
fn depth(value: &Value) -> usize {
    let mut deepest = 0;
    let mut pending = vec![(value, 1)];
    while let Some((value, at)) = pending.pop() {
        deepest = deepest.max(at);
        match value {
            Value::Array(items) => pending.extend(items.iter().map(|item| (item, at + 1))),
            Value::Object(members) => pending.extend(members.values().map(|item| (item, at + 1))),
            _ => {}
        }
    }
    deepest
}

fn from_toml(value: toml::Value) -> Value {
    match value {
        toml::Value::String(text) => Value::String(text),
        toml::Value::Integer(number) => Value::from(number),
        toml::Value::Float(number) => Number::from_f64(number).map_or(Value::Null, Value::Number),
        toml::Value::Boolean(flag) => Value::Bool(flag),
        toml::Value::Datetime(when) => Value::String(when.to_string()),
        toml::Value::Array(items) => Value::Array(items.into_iter().map(from_toml).collect()),
        toml::Value::Table(table) => Value::Object(
            table
                .into_iter()
                .map(|(key, value)| (key, from_toml(value)))
                .collect(),
        ),
    }
}

/// The first YAML document of `text`; an empty stream is `null`.
fn yaml(text: &str) -> Result<Value, ParseError> {
    let mut builder = Builder::default();
    let loaded = Parser::new_from_str(text).load(&mut builder, false);
    if let Err(error) = loaded {
        let mark = error.marker();
        return Err(ParseError::at(error.info(), mark.line(), mark.col() + 1));
    }
    if let Some(error) = builder.error {
        return Err(error);
    }
    Ok(builder.done.unwrap_or(Value::Null))
}

/// A collection being built: a sequence, or a mapping with the key read but not yet its value; with its anchor and
/// how many nodes it holds so far.
enum Open {
    Sequence(Vec<Value>, usize, usize),
    Mapping(Map<String, Value>, Option<String>, usize, usize),
}

/// Builds a tree from the parser's events, each alias expanded by copy and counted.
#[derive(Default)]
struct Builder {
    open: Vec<Open>,
    /// Each anchor's node and how many nodes it holds.
    anchors: HashMap<usize, (Value, usize)>,
    nodes: usize,
    done: Option<Value>,
    error: Option<ParseError>,
}

fn at_mark(message: impl Into<String>, mark: Marker) -> ParseError {
    ParseError::at(message, mark.line(), mark.col() + 1)
}

impl Builder {
    fn fail(&mut self, error: ParseError) {
        self.error.get_or_insert(error);
    }

    /// Count `fresh` more nodes: false, and the tree fails, past [`MAX_NODES`].
    fn count(&mut self, fresh: usize, mark: Marker) -> bool {
        self.nodes = self.nodes.saturating_add(fresh);
        if self.nodes > MAX_NODES {
            let why = format!("holds more than {MAX_NODES} nodes once its aliases are read");
            self.fail(at_mark(why, mark));
        }
        self.error.is_none()
    }

    /// Add `value`, which holds `size` nodes (anchored as `anchor` when not 0), to what is open.
    fn add(&mut self, value: Value, size: usize, anchor: usize) {
        if anchor != 0 {
            self.anchors.insert(anchor, (value.clone(), size));
        }
        match self.open.last_mut() {
            None => {
                self.done.get_or_insert(value);
            }
            Some(Open::Sequence(items, _, held)) => {
                *held += size;
                items.push(value);
            }
            Some(Open::Mapping(_, key @ None, _, held)) => {
                *held += size;
                *key = Some(key_of(value));
            }
            Some(Open::Mapping(members, key @ Some(_), _, held)) => {
                *held += size;
                let key = key.take().expect("a key is held");
                members.insert(key, value);
            }
        }
    }

    fn open(&mut self, collection: Open, mark: Marker) {
        if self.open.len() >= MAX_DEPTH {
            return self.fail(at_mark(
                format!("nests deeper than {MAX_DEPTH} levels"),
                mark,
            ));
        }
        if self.count(1, mark) {
            self.open.push(collection);
        }
    }
}

/// A key as the tree writes it: a string as it is, anything else as its JSON.
fn key_of(value: Value) -> String {
    match value {
        Value::String(text) => text,
        other => other.to_string(),
    }
}

/// A plain scalar as YAML 1.2's core schema reads it; a quoted one is a string.
fn scalar(text: String, style: TScalarStyle) -> Value {
    if style != TScalarStyle::Plain {
        return Value::String(text);
    }
    match Yaml::from_str(&text) {
        Yaml::Null => Value::Null,
        Yaml::Boolean(flag) => Value::Bool(flag),
        Yaml::Integer(number) => Value::from(number),
        Yaml::Real(real) => real
            .parse::<f64>()
            .ok()
            .and_then(Number::from_f64)
            .map_or(Value::String(text), Value::Number),
        _ => Value::String(text),
    }
}

impl MarkedEventReceiver for Builder {
    fn on_event(&mut self, event: Event, mark: Marker) {
        if self.error.is_some() || self.done.is_some() && self.open.is_empty() {
            return;
        }
        match event {
            Event::Scalar(text, style, anchor, _) => {
                if self.count(1, mark) {
                    self.add(scalar(text, style), 1, anchor);
                }
            }
            Event::Alias(anchor) => match self.anchors.get(&anchor).map(|(_, size)| *size) {
                // Counted before it is copied: past the cap nothing is.
                Some(size) if self.count(size, mark) => {
                    let value = self.anchors[&anchor].0.clone();
                    self.add(value, size, 0);
                }
                Some(_) => {}
                None => self.fail(at_mark("names an anchor not defined before it", mark)),
            },
            Event::SequenceStart(anchor, _) => {
                self.open(Open::Sequence(Vec::new(), anchor, 1), mark)
            }
            Event::MappingStart(anchor, _) => {
                self.open(Open::Mapping(Map::new(), None, anchor, 1), mark)
            }
            Event::SequenceEnd | Event::MappingEnd => {
                let (value, anchor, size) = match self.open.pop() {
                    Some(Open::Sequence(items, anchor, size)) => {
                        (Value::Array(items), anchor, size)
                    }
                    Some(Open::Mapping(members, _, anchor, size)) => {
                        (Value::Object(members), anchor, size)
                    }
                    None => return,
                };
                self.add(value, size, anchor);
            }
            _ => {}
        }
    }
}
