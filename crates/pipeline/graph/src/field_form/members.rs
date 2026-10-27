//! The members of a typed block field (ADR 0285): each a field of the block, read as a field is.

use super::FieldForm;

/// A typed `block` or `block_list` field's members, in declared order, each with the form that reads its value.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct BlockMembers(Vec<(String, FieldForm)>);

impl BlockMembers {
    pub fn new(members: Vec<(String, FieldForm)>) -> Self {
        Self(members)
    }

    /// The member `name`, if declared.
    pub fn get(&self, name: &str) -> Option<&FieldForm> {
        self.0
            .iter()
            .find(|(member, _)| member == name)
            .map(|(_, form)| form)
    }

    /// Every member and its form, in declared order.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &FieldForm)> {
        self.0.iter().map(|(name, form)| (name.as_str(), form))
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}
