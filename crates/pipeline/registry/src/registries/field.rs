use std::collections::HashMap;

use specforge_graph::FieldForm;
use specforge_graph::field_form::BlockMembers;
use specforge_protocol_types::{FieldDescriptor, FieldType, FileBase, ProofRole};

/// A declared field type this host does not read. The registry build
/// reports it as W019 and registers nothing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownFieldType(pub String);

/// What the registry refuses in a field's declaration and drops, so the field keeps the rest (ADR 0285). The
/// registry build reports each: [`Self::UnknownMemberType`] as W019, [`Self::Refused`] as W021.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FieldProblem {
    /// A member whose type this host does not read: the member is dropped.
    UnknownMemberType { member: String, field_type: String },
    /// An attribute the declaration states wrongly (members on a scalar, a bound on a string, both target
    /// fields, a bad base …): the attribute is dropped. `field` is the field or member path (`requires.phase`),
    /// `what` completes "field 'f' on kind 'k' …".
    Refused { field: String, what: String },
}

/// One registered field of one kind: the descriptor its extension declared,
/// the kind it is registered on and the extension that declared the field:
/// the kind's own extension, its shared field's or the enhancement's.
///
/// [`FieldRegistryEntry::new`] is the only way to make one, so an entry's
/// type, enum values and proof role are always its declaration's, read once.
/// There is no second copy to disagree with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldRegistryEntry {
    kind_name: String,
    source_extension: String,
    form: FieldForm,
    proof_role: Option<ProofRole>,
    declared: FieldDescriptor,
    problems: Vec<FieldProblem>,
}

impl FieldRegistryEntry {
    /// `declared`, registered on `kind_name`, declared by `source_extension`.
    ///
    /// The declared type must be a name the host reads (canonical or an
    /// accepted older spelling), else `Err` (W019 at the registry build). The
    /// entry's descriptor names the type canonically (`"boolean"` becomes
    /// `"bool"`), so every reader of `declared()` sees the one name. A
    /// `proof_role` that is not `bound` or `claim` is read as none; the
    /// registry build reports it (W021). The members of a block field are read the same way, recursively; what is
    /// refused in the declaration is dropped from it and listed in [`Self::problems`].
    pub fn new(
        kind_name: &str,
        source_extension: &str,
        mut declared: FieldDescriptor,
    ) -> Result<Self, UnknownFieldType> {
        let field_type = FieldType::parse(&declared.field_type)
            .ok_or_else(|| UnknownFieldType(declared.field_type.clone()))?;
        declared.field_type = field_type.as_str().to_string();
        let proof_role = declared.proof_role.as_deref().and_then(ProofRole::parse);
        let mut problems = Vec::new();
        let path = declared.name.clone();
        let form = read(&mut declared, field_type, &path, false, &mut problems);
        Ok(Self {
            kind_name: kind_name.to_string(),
            source_extension: source_extension.to_string(),
            form,
            proof_role,
            declared,
            problems,
        })
    }

    /// What the registry refused in the declaration, in declared order.
    pub fn problems(&self) -> &[FieldProblem] {
        &self.problems
    }

    /// The descriptor and form at a member path: `requires.phase` is the member `phase` of the field `requires`; a
    /// path of one name is the field itself.
    pub fn at_path(&self, path: &str) -> Option<(&FieldDescriptor, &FieldForm)> {
        let mut parts = path.split('.');
        if parts.next()? != self.declared.name {
            return None;
        }
        let (mut declared, mut form) = (&self.declared, &self.form);
        for part in parts {
            declared = declared.members.iter().find(|m| m.name == part)?;
            form = form.members()?.get(part)?;
        }
        Some((declared, form))
    }

    /// The kind the field is registered on.
    pub fn kind_name(&self) -> &str {
        &self.kind_name
    }

    /// The field's name, as declared.
    pub fn name(&self) -> &str {
        &self.declared.name
    }

    /// The extension that declared the field.
    pub fn source_extension(&self) -> &str {
        &self.source_extension
    }

    /// How the host reads the field's value.
    pub fn field_type(&self) -> FieldType {
        self.form.field_type()
    }

    /// The values an enum field accepts, as declared; empty for any other
    /// type (and for an enum that declares none).
    pub fn enum_values(&self) -> &[String] {
        self.form.enum_values()
    }

    /// The field's declared type read once (ADR 0064): which stored
    /// values it takes, how a written value is stored as it and the JSON
    /// written for it.
    pub fn form(&self) -> &FieldForm {
        &self.form
    }

    /// The type as messages and hover name it: the type's name, followed by
    /// an enum's declared values: `enum (low, medium, high)`.
    pub fn type_label(&self) -> String {
        match self.enum_values() {
            [] => self.field_type().as_str().to_string(),
            values => format!("{} ({})", self.field_type().as_str(), values.join(", ")),
        }
    }

    /// What the prove pass reads the field as, when it declares a known role.
    pub fn proof_role(&self) -> Option<ProofRole> {
        self.proof_role
    }

    /// The field as its extension declared it (its type named canonically).
    pub fn declared(&self) -> &FieldDescriptor {
        &self.declared
    }
}

#[derive(Debug, Default)]
pub struct FieldRegistry {
    /// Two-level map: kind_name -> field_name -> entry.
    /// Using nested `HashMap<String, _>` instead of `HashMap<(String, String), _>`
    /// so lookups can accept `&str` directly (via `Borrow<str>` on `String`),
    /// avoiding per-call `to_string()` allocations.
    entries: HashMap<String, HashMap<String, FieldRegistryEntry>>,
    /// Total number of registered fields (cached for O(1) len).
    count: usize,
}

impl FieldRegistry {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
            count: 0,
        }
    }

    pub fn len(&self) -> usize {
        self.count
    }

    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// The descriptor and form at `path` on `kind`: a field's name, or a member path `field.member`.
    pub fn get_path(&self, kind_name: &str, path: &str) -> Option<(&FieldDescriptor, &FieldForm)> {
        let field = path.split('.').next()?;
        self.get(kind_name, field)?.at_path(path)
    }

    /// Look up a field entry by kind and field name.
    /// Zero-allocation: accepts `&str` without converting to `String`.
    pub fn get(&self, kind_name: &str, field_name: &str) -> Option<&FieldRegistryEntry> {
        self.entries.get(kind_name)?.get(field_name)
    }

    /// Check if a field is registered for the given kind.
    /// Zero-allocation: accepts `&str` without converting to `String`.
    pub fn contains(&self, kind_name: &str, field_name: &str) -> bool {
        self.entries
            .get(kind_name)
            .is_some_and(|fields| fields.contains_key(field_name))
    }

    pub fn register(&mut self, entry: FieldRegistryEntry) {
        let kind_map = self
            .entries
            .entry(entry.kind_name().to_string())
            .or_default();
        if !kind_map.contains_key(entry.name()) {
            self.count += 1;
        }
        kind_map.insert(entry.name().to_string(), entry);
    }

    /// The fields that own the relation an unregistered `name` on `kind` would
    /// write from the other side (ADR 0150): every reference field of another
    /// kind that targets `kind` and names `name` its `inverse_of`; then, when
    /// `value_kind` is another kind, every reference field of `value_kind` that
    /// targets `kind`. Each once, the `inverse_of` matches first; each group
    /// by (kind, field). Empty when `kind` registers `name`; never a field of
    /// `kind` itself (a self-relation's other side cannot be told from a new
    /// one).
    pub fn owning_fields(
        &self,
        kind: &str,
        name: &str,
        value_kind: Option<&str>,
    ) -> Vec<&FieldRegistryEntry> {
        if self.contains(kind, name) {
            return Vec::new();
        }
        let into_kind = || {
            self.iter().filter(move |(owner, _, entry)| {
                *owner != kind
                    && entry.field_type().is_reference()
                    && entry.declared().target_kind.as_deref() == Some(kind)
            })
        };
        let by_owner = |a: &&FieldRegistryEntry, b: &&FieldRegistryEntry| {
            (a.kind_name(), a.name()).cmp(&(b.kind_name(), b.name()))
        };
        let mut owning: Vec<&FieldRegistryEntry> = into_kind()
            .filter(|(_, _, entry)| entry.declared().inverse_of.as_deref() == Some(name))
            .map(|(_, _, entry)| entry)
            .collect();
        owning.sort_by(by_owner);
        if let Some(value_kind) = value_kind.filter(|value_kind| *value_kind != kind) {
            let mut by_value: Vec<&FieldRegistryEntry> = into_kind()
                .filter(|(owner, _, entry)| {
                    *owner == value_kind && !owning.iter().any(|seen| std::ptr::eq(*seen, *entry))
                })
                .map(|(_, _, entry)| entry)
                .collect();
            by_value.sort_by(by_owner);
            owning.extend(by_value);
        }
        owning
    }

    pub fn fields_for_kind(&self, kind_name: &str) -> Vec<&FieldRegistryEntry> {
        self.entries
            .get(kind_name)
            .map(|fields| fields.values().collect())
            .unwrap_or_default()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&str, &str, &FieldRegistryEntry)> {
        self.entries.iter().flat_map(|(kind, fields)| {
            fields
                .iter()
                .map(move |(field, entry)| (kind.as_str(), field.as_str(), entry))
        })
    }
}

/// Whether `name` is one identifier: a member's name never holds a `.` or `[`, so a member path parses back.
fn is_identifier(name: &str) -> bool {
    let mut chars = name.chars();
    chars
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

fn refuse(problems: &mut Vec<FieldProblem>, field: &str, what: String) {
    problems.push(FieldProblem::Refused {
        field: field.to_string(),
        what,
    });
}

/// The form of `declared`, a field of type `field_type` named `path` (a member path for a member), with whatever
/// the declaration states wrongly dropped from `declared` and listed in `problems` (ADR 0285).
fn read(
    declared: &mut FieldDescriptor,
    field_type: FieldType,
    path: &str,
    is_member: bool,
    problems: &mut Vec<FieldProblem>,
) -> FieldForm {
    let ty = field_type.as_str();
    if !declared.members.is_empty() && !field_type.is_block() {
        refuse(
            problems,
            path,
            format!(
                "declares members, but its type is '{ty}': only a block or block_list has members"
            ),
        );
        declared.members.clear();
    }
    if field_type == FieldType::BlockList && declared.members.is_empty() {
        refuse(
            problems,
            path,
            "is a block_list that declares no members".into(),
        );
    }
    let mut kept: Vec<FieldDescriptor> = Vec::new();
    let mut forms = Vec::new();
    for mut member in std::mem::take(&mut declared.members) {
        let member_path = format!("{path}.{}", member.name);
        if !is_identifier(&member.name) {
            refuse(
                problems,
                path,
                format!(
                    "declares a member '{}' whose name is not one identifier",
                    member.name
                ),
            );
            continue;
        }
        if kept.iter().any(|m| m.name == member.name) {
            refuse(
                problems,
                path,
                format!("declares the member '{}' twice", member.name),
            );
            continue;
        }
        let Some(member_type) = FieldType::parse(&member.field_type) else {
            problems.push(FieldProblem::UnknownMemberType {
                member: member_path,
                field_type: member.field_type.clone(),
            });
            continue;
        };
        member.field_type = member_type.as_str().to_string();
        if member.derived_from.take().is_some() {
            refuse(
                problems,
                &member_path,
                "declares derived_from: a block member is no type syntax".into(),
            );
        }
        let form = read(&mut member, member_type, &member_path, true, problems);
        forms.push((member.name.clone(), form));
        kept.push(member);
    }
    declared.members = kept;

    if declared.target_kind.is_some() && !declared.target_kinds.is_empty() {
        refuse(
            problems,
            path,
            "declares both target_kind and target_kinds".into(),
        );
        declared.target_kinds.clear();
    }
    if !declared.target_kinds.is_empty() && !field_type.is_reference() {
        refuse(
            problems,
            path,
            format!("declares target_kinds, but its type is '{ty}'"),
        );
        declared.target_kinds.clear();
    }
    if declared.target_kinds.len() == 1 {
        refuse(
            problems,
            path,
            "declares target_kinds of one kind: write target_kind".into(),
        );
        declared.target_kinds.clear();
    }
    if (declared.minimum.is_some() || declared.maximum.is_some())
        && field_type != FieldType::Integer
    {
        refuse(
            problems,
            path,
            format!(
                "declares a minimum or maximum, but its type is '{ty}': only an integer has bounds"
            ),
        );
        (declared.minimum, declared.maximum) = (None, None);
    }
    if let (Some(minimum), Some(maximum)) = (declared.minimum, declared.maximum)
        && minimum > maximum
    {
        refuse(
            problems,
            path,
            format!("declares minimum {minimum} above maximum {maximum}"),
        );
        (declared.minimum, declared.maximum) = (None, None);
    }
    if let Some(base) = declared.file_base.clone() {
        if !declared.file_reference {
            refuse(
                problems,
                path,
                "declares file_base without file_reference".into(),
            );
            declared.file_base = None;
        } else if FileBase::parse(&base).is_none() {
            refuse(
                problems,
                path,
                format!("declares file_base '{base}': expected 'spec' or 'project'"),
            );
            declared.file_base = None;
        }
    }
    if matches!(field_type, FieldType::Enum | FieldType::EnumList)
        && declared.enum_values.is_empty()
    {
        refuse(
            problems,
            path,
            format!("is an {ty} that declares no values"),
        );
    } else if field_type == FieldType::Enum
        && let Some(default) = declared.default_value.as_deref()
        && !declared.enum_values.iter().any(|v| v == default)
    {
        refuse(
            problems,
            path,
            format!(
                "declares default_value '{default}', which is not one of its values: {}",
                declared.enum_values.join(", ")
            ),
        );
    }
    if matches!(
        field_type,
        FieldType::Block | FieldType::BlockList | FieldType::EnumList
    ) && declared.default_value.is_some()
    {
        refuse(
            problems,
            path,
            format!("declares a default_value, but its type is '{ty}'"),
        );
        declared.default_value = None;
    }
    if is_member && declared.edge.is_some() && !field_type.is_reference() {
        refuse(
            problems,
            path,
            format!("declares an edge, but its type is '{ty}': only a reference member has edges"),
        );
        declared.edge = None;
    }

    let mut form = FieldForm::new(field_type, declared.enum_values.clone())
        .bounded(declared.minimum, declared.maximum)
        .requiring(is_member && declared.required);
    form = match &declared.target_kind {
        Some(kind) => form.targeting(kind),
        None => form.targeting_any(&declared.target_kinds),
    };
    if declared.file_reference
        && let Some(base) = declared.file_base.as_deref().and_then(FileBase::parse)
    {
        form = form.based(base);
    }
    if field_type.is_block() && !forms.is_empty() {
        form = form.with_members(BlockMembers::new(forms));
    }
    form
}
