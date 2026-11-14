//! A JSON value against a field's form (ADR 0287): what a compute pass answers and the harness ledger holds is
//! admitted by the form of the field it is for, as a written value is by [`FieldForm::admit`].

use serde_json::Value;
use specforge_protocol_types::{Decimal, DecimalError, FieldType};

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

    fn admitted(&self, value: &Value) -> Result<(), Refusal> {
        let wrong = || Refusal::with(&format!("give a {}", self.field_type().as_str()));
        match (self.field_type(), value) {
            (FieldType::Integer, Value::Number(n)) => {
                let n = n.as_i64().ok_or_else(wrong)?;
                self.admit(ValueShape::Integer, &n.to_string())
            }
            (FieldType::Number, Value::Number(n)) => {
                let value = Decimal::from_json(n).map_err(|e| Refusal::with(&e.to_string()))?;
                self.in_decimal_bounds(value)
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
            return match entries
                .iter()
                .find_map(|(name, v)| not_field_json(v).map(|why| (name, why)))
            {
                None => Ok(()),
                Some((name, NotFieldJson::Number(error))) => {
                    Err(Refusal::with(&format!("'{name}': {error}")))
                }
                Some((name, NotFieldJson::Shape)) => Err(Refusal::with(&format!(
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

/// Why a value is no field value as every output writes one.
enum NotFieldJson {
    /// A fraction no [`Decimal`] holds: more than 15 significant digits or out of range (ADR 0361).
    Number(DecimalError),
    /// A null, or a list holding a list or a null.
    Shape,
}

/// Why `value` is no field value as every output writes one (`FieldJson`'s shape), `None` when it is one: a string,
/// a whole number within `i64` or `u64` (an `integer`, as `.spec` writes one), a fraction a [`Decimal`] holds, a
/// boolean, an array of strings, numbers, booleans and objects, or an object of field values.
fn not_field_json(value: &Value) -> Option<NotFieldJson> {
    let scalar = |v: &Value| match v {
        Value::String(_) | Value::Bool(_) => None,
        Value::Number(n) if n.is_i64() || n.is_u64() => None,
        Value::Number(n) => Decimal::from_json(n).err().map(NotFieldJson::Number),
        _ => Some(NotFieldJson::Shape),
    };
    match value {
        Value::Array(items) => items.iter().find_map(|item| match item {
            Value::Object(entries) => entries.values().find_map(not_field_json),
            other => scalar(other),
        }),
        Value::Object(entries) => entries.values().find_map(not_field_json),
        other => scalar(other),
    }
}
