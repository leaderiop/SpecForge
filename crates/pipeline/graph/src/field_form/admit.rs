//! A JSON value against a field's form (ADR 0287): what a compute pass answers and the harness ledger holds is
//! admitted by the form of the field it is for, as a written value is by [`FieldForm::admit`].

use serde_json::Value;
use specforge_protocol_types::FieldType;

use super::{FieldForm, FieldJson, Refusal, ValueShape};

impl FieldForm {
    /// `value` as this field takes it, or why not. A scalar is the JSON of its type (`string`, `enum`, `identifier`,
    /// `reference` and `date` a string; `integer` an integer within its bounds; `bool` a boolean), a list an array
    /// of strings, a `block` an object and a `block_list` an array of objects, each entry admitted by its member's
    /// form; a member the block does not declare is refused. The same rules `admit` and `refuse_item` apply to a
    /// written value.
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
        let Some(members) = self.members() else {
            return Ok(());
        };
        for (name, member) in entries {
            members
                .get(name)
                .ok_or_else(|| Refusal::with(&format!("'{name}' is no member of this block")))?
                .admitted(member)?;
        }
        Ok(())
    }
}
