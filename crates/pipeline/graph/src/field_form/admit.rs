//! A JSON value against a field's form (ADR 0287): what a compute pass answers and the harness ledger holds is
//! admitted by the form of the field it is for, as a written value is by [`FieldForm::admit`].

use serde_json::Value;
use specforge_protocol_types::FieldType;

use super::{FieldForm, FieldJson, Refusal, ValueShape};

impl FieldForm {
    /// `value` as this field takes it, or why not. A scalar is the JSON of its type (`string`, `enum`, `identifier`,
    /// `reference` and `date` a string; `integer` an integer and `number` a number, within their bounds; `bool` a
    /// boolean), a list an array of strings, a `block` an object and a `block_list` an array of objects, each entry
    /// admitted by its member's form; a member the block does not declare is refused. The same rules `admit` and
    /// `refuse_item` apply to a written value.
    pub fn admit_json(&self, value: &Value) -> Result<FieldJson, Refusal> {
        self.admitted(value)?;
        Ok(FieldJson::of(value.clone()))
    }

    /// `Ok` when the number `x` lies within the declared bounds (a `number` setting's).
    fn in_number_bounds(&self, x: f64) -> Result<(), Refusal> {
        let (minimum, maximum) = self.bounds;
        if minimum.is_some_and(|m| x < m as f64) || maximum.is_some_and(|m| x > m as f64) {
            return Err(Refusal::with(&match (minimum, maximum) {
                (Some(low), Some(high)) => format!("write a number from {low} to {high}"),
                (Some(low), None) => format!("write a number of at least {low}"),
                (None, Some(high)) => format!("write a number of at most {high}"),
                (None, None) => unreachable!("an unbounded number is in bounds"),
            }));
        }
        Ok(())
    }

    fn admitted(&self, value: &Value) -> Result<(), Refusal> {
        let wrong = || Refusal::with(&format!("give a {}", self.field_type().as_str()));
        match (self.field_type(), value) {
            (FieldType::Integer, Value::Number(n)) => {
                let n = n.as_i64().ok_or_else(wrong)?;
                self.admit(ValueShape::Integer, &n.to_string())
            }
            (FieldType::Number, Value::Number(n)) => {
                let x = n.as_f64().ok_or_else(wrong)?;
                self.in_number_bounds(x)
            }
            (FieldType::Bool, Value::Bool(_)) => Ok(()),
            (FieldType::Date, Value::String(s)) => self.admit(ValueShape::Date, s),
            (
                FieldType::String | FieldType::Enum | FieldType::Identifier | FieldType::Reference,
                Value::String(s),
            ) => self.admit(ValueShape::String, s),
            (
                FieldType::StringList | FieldType::ReferenceList | FieldType::EnumList,
                Value::Array(items),
            ) => {
                let items: Vec<String> = items
                    .iter()
                    .map(|item| item.as_str().map(str::to_string).ok_or_else(wrong))
                    .collect::<Result<_, _>>()?;
                match self.refuse_item(&items) {
                    Some((_, refusal)) => Err(refusal),
                    None => Ok(()),
                }
            }
            (FieldType::Block, Value::Object(entries)) => self.admit_entries(entries),
            (FieldType::BlockList, Value::Array(items)) => {
                items.iter().try_for_each(|item| match item {
                    Value::Object(entries) => self.admit_entries(entries),
                    _ => Err(Refusal::with("give each item as an object")),
                })
            }
            _ => Err(wrong()),
        }
    }

    fn admit_entries(&self, entries: &serde_json::Map<String, Value>) -> Result<(), Refusal> {
        let Some(members) = self.members().filter(|m| !m.is_empty()) else {
            // An open block: named entries, each a field value.
            return match entries.iter().find(|(_, v)| !is_field_json(v)) {
                None => Ok(()),
                Some((name, _)) => Err(Refusal::with(&format!(
                    "give '{name}' a string, a number, a boolean, a list or an object"
                ))),
            };
        };
        for (name, member) in entries {
            members
                .get(name)
                .ok_or_else(|| Refusal::with(&format!("'{name}' is no member of this block")))?
                .admitted(member)?;
        }
        if let Some(missing) = members
            .iter()
            .find(|(name, form)| form.required() && !entries.contains_key(*name))
        {
            return Err(Refusal::with(&format!("give the member '{}'", missing.0)));
        }
        Ok(())
    }
}

/// Whether `value` is a field value as every output writes one (`FieldJson`'s shape): a string, an integer, a
/// boolean, an array of strings, integers, booleans and objects, or an object of field values.
fn is_field_json(value: &Value) -> bool {
    let scalar = |v: &Value| match v {
        Value::String(_) | Value::Bool(_) => true,
        Value::Number(n) => n.is_i64() || n.is_u64(),
        _ => false,
    };
    match value {
        Value::Array(items) => items.iter().all(|item| match item {
            Value::Object(entries) => entries.values().all(is_field_json),
            other => scalar(other),
        }),
        Value::Object(entries) => entries.values().all(is_field_json),
        other => scalar(other),
    }
}
