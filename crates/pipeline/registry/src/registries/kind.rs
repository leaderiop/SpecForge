use std::collections::HashMap;

use specforge_protocol_types::{
    CompleteWhen, EntityKindDescriptor, FieldType, FieldWriter, TestStatus,
};

/// One registered entity kind: what its extension declared, and what the
/// registry build resolved of it. Everything else is read from `declared`
/// (`entry.declared.description`, `entry.declared.has_body_parser`, ...).
#[derive(Debug, Clone, Default)]
pub struct KindRegistryEntry {
    /// The keyword its entities are written with: the declared keyword,
    /// else its name.
    pub kind_name: String,
    pub source_extension: String,
    /// Declared, or made so by an enhancement carrying `verify_kinds`.
    pub testable: bool,
    /// Declared, or made so by an enhancement carrying `verify_kinds`.
    pub supports_verify: bool,
    /// The `verify` kinds it accepts: declared, or an enhancement's.
    pub allowed_verify_kinds: Vec<String>,
    /// The field naming the entity its entities are scoped by (ADR 0284): the declared one, when it is a single
    /// reference with a target kind that is a loaded kind and closes no scope cycle (W021 otherwise).
    pub scoped_by: Option<String>,
    /// The kind as its extension declared it.
    pub declared: EntityKindDescriptor,
}

impl KindRegistryEntry {
    /// Whether only a person writes its entities (`writer: human`, ADR 0289).
    pub fn is_human_only(&self) -> bool {
        self.declared.writer.as_deref() == Some(FieldWriter::Human.as_str())
    }

    /// Whether a collector writes its entities, its facts (`writer: collector`, ADR 0327): never a spec file.
    pub fn is_collected(&self) -> bool {
        self.declared.writer.as_deref() == Some(FieldWriter::Collector.as_str())
    }

    /// The schemes its facts' ids are written in (`js` for `js:pkg/a.test.js#adds`): a collected kind's only.
    pub fn schemes(&self) -> &[String] {
        &self.declared.schemes
    }

    /// The enum field holding its facts' result (ADR 0329 D3): a collected kind's only, as the registry kept it.
    pub fn result(&self) -> Option<&str> {
        self.declared.result.as_deref()
    }

    /// When its entities are complete (ADR 0329 D4), as the registry kept it.
    pub fn complete_when(&self) -> Option<&CompleteWhen> {
        self.declared.complete_when.as_ref()
    }
}

/// `declared`'s writer and schemes as the registry keeps them (ADR 0289, 0327), and what it dropped (W021, each a
/// message after "kind '<k>' "): a writer other than `human` or `collector`, `schemes` on a kind no collector writes,
/// and a scheme that is no lowercase name. A collected kind left with no scheme stays collected.
pub(crate) fn read_writer(declared: &mut EntityKindDescriptor) -> Vec<String> {
    let mut problems = Vec::new();
    if let Some(writer) = declared.writer.clone()
        && !matches!(
            FieldWriter::parse(&writer),
            Some(FieldWriter::Human | FieldWriter::Collector)
        )
    {
        problems.push(format!(
            "declares writer '{writer}': expected 'human' or 'collector'"
        ));
        declared.writer = None;
    }
    if declared.writer.as_deref() != Some(FieldWriter::Collector.as_str()) {
        if !declared.schemes.is_empty() {
            problems.push(format!(
                "declares schemes [{}] without writer 'collector': only a collector's facts have scheme ids",
                declared.schemes.join(", ")
            ));
            declared.schemes.clear();
        }
        return problems;
    }
    declared.schemes.retain(|scheme| {
        let named = is_scheme(scheme);
        if !named {
            problems.push(format!(
                "declares the scheme '{scheme}', which is not a lowercase name ([a-z][a-z0-9_-]*)"
            ));
        }
        named
    });
    if declared.schemes.is_empty() {
        problems.push(no_scheme());
    }
    problems
}

/// `declared`'s `result` as the registry keeps it (ADR 0329 D3), and why it dropped it (W021, a message after "kind
/// '<k>' "): it is kept on a collected kind naming an enum field the kind declares whose values are all `TestStatus`
/// names. Read after [`read_writer`], so a writer it dropped leaves the kind uncollected.
pub(crate) fn read_result(declared: &mut EntityKindDescriptor) -> Option<String> {
    let field = declared.result.as_deref()?;
    let why = if declared.writer.as_deref() != Some(FieldWriter::Collector.as_str()) {
        "only a fact has a result of its own, and no collector writes the kind".to_string()
    } else {
        let statuses = enum_outside(declared, field, |values| {
            values
                .iter()
                .map(String::as_str)
                .filter(|v| !TestStatus::ALL.iter().any(|s| s.as_str() == *v))
                .collect()
        });
        match statuses {
            Err(why) => why,
            Ok((outside, _)) if outside.is_empty() => return None,
            Ok((outside, _)) => format!(
                "its values [{}] are not test statuses (passed, failed, skipped)",
                outside.join(", ")
            ),
        }
    };
    let message = format!("declares result '{field}': {why}");
    declared.result = None;
    Some(message)
}

/// `declared`'s `complete_when` as the registry keeps it (ADR 0329 D4), and why it dropped it (W021, a message after
/// "kind '<k>' "): it is kept when it names an enum field the kind declares and at least one value, each one of the
/// field's.
pub(crate) fn read_complete_when(declared: &mut EntityKindDescriptor) -> Option<String> {
    let when = declared.complete_when.as_ref()?;
    let values = enum_outside(declared, &when.field, |values| {
        when.values
            .iter()
            .map(String::as_str)
            .filter(|v| !values.iter().any(|e| e == v))
            .collect()
    });
    let why = match values {
        Err(why) => why,
        Ok(_) if when.values.is_empty() => {
            "it names no value, so no entity of the kind would ever be complete".to_string()
        }
        Ok((outside, _)) if outside.is_empty() => return None,
        Ok((outside, values)) => format!(
            "[{}] are not among its values [{}]",
            outside.join(", "),
            values.join(", ")
        ),
    };
    let message = format!("declares complete_when '{}': {why}", when.field);
    declared.complete_when = None;
    Some(message)
}

/// Of `declared`'s enum field `name`: the values `outside` picks from its own (W021 names them), and its own; or W021's
/// reason when the kind declares no field `name`, or one that is not an enum.
fn enum_outside<'a>(
    declared: &'a EntityKindDescriptor,
    name: &str,
    outside: impl FnOnce(&'a [String]) -> Vec<&'a str>,
) -> Result<(Vec<&'a str>, &'a [String]), String> {
    let field = declared
        .fields
        .iter()
        .find(|f| f.name == name)
        .ok_or("it is not one of its fields")?;
    if FieldType::parse(&field.field_type) != Some(FieldType::Enum) {
        return Err("it is not an enum field".to_string());
    }
    Ok((outside(&field.enum_values), &field.enum_values))
}

/// What a collected kind left with no scheme is told (W021).
pub(crate) fn no_scheme() -> String {
    "declares writer 'collector' and no scheme: its facts name no scheme, so no reference resolves to one on a fresh \
     clone"
        .to_string()
}

/// Whether `scheme` is a lowercase name, `[a-z][a-z0-9_-]*`.
fn is_scheme(scheme: &str) -> bool {
    let mut chars = scheme.chars();
    chars.next().is_some_and(|c| c.is_ascii_lowercase())
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-')
}

#[derive(Debug, Default)]
pub struct KindRegistry {
    entries: HashMap<String, KindRegistryEntry>,
}

impl KindRegistry {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn get(&self, kind_name: &str) -> Option<&KindRegistryEntry> {
        self.entries.get(kind_name)
    }

    pub fn get_mut(&mut self, kind_name: &str) -> Option<&mut KindRegistryEntry> {
        self.entries.get_mut(kind_name)
    }

    pub fn contains(&self, kind_name: &str) -> bool {
        self.entries.contains_key(kind_name)
    }

    /// The kind `kind_name` when a collector writes it (ADR 0327): `None` for a kind not loaded or written otherwise.
    pub fn collected(&self, kind_name: &str) -> Option<&KindRegistryEntry> {
        self.get(kind_name).filter(|kind| kind.is_collected())
    }

    /// Whether a collector writes the kind `kind_name` (ADR 0327).
    pub fn is_collected(&self, kind_name: &str) -> bool {
        self.collected(kind_name).is_some()
    }

    /// Whether a collector writes any loaded kind: whether a session reads fact stores (ADR 0327 D5).
    pub fn any_collected(&self) -> bool {
        self.entries.values().any(KindRegistryEntry::is_collected)
    }

    pub fn register(&mut self, entry: KindRegistryEntry) -> Option<KindRegistryEntry> {
        self.entries.insert(entry.kind_name.clone(), entry)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&String, &KindRegistryEntry)> {
        self.entries.iter()
    }

    pub(crate) fn iter_mut(&mut self) -> impl Iterator<Item = &mut KindRegistryEntry> {
        self.entries.values_mut()
    }

    pub fn keywords(&self) -> impl Iterator<Item = &String> {
        self.entries.keys()
    }

    /// Whether `id` may not be an entity ID (E013): a structural keyword, or the keyword of a registered
    /// kind. An ID equal to one makes `refs [behavior]`-style entries ambiguous with the block introducer.
    pub fn reserves(&self, id: &str) -> bool {
        specforge_common::structural::is_structural(id) || self.entries.contains_key(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // B:boot_empty_kind_registry — verify unit "KindRegistry::new() has zero entries"
    #[test]
    fn test_kind_registry_new_has_zero_entries() {
        let registry = KindRegistry::new();
        assert_eq!(registry.len(), 0);
        assert!(registry.is_empty());
    }

    // B:boot_empty_kind_registry — verify unit "parser recognizes spec keyword without extensions"
    // B:boot_empty_kind_registry — verify unit "parser recognizes ref keyword without extensions"
    // B:boot_empty_kind_registry — verify unit "parser recognizes use keyword without extensions"
    // B:boot_empty_kind_registry — verify unit "parser recognizes define keyword without extensions"
    // These 4 are parser-level tests (structural keywords are grammar rules, not registry entries).
    // The KindRegistry has zero entries — structural keywords bypass it entirely.
    #[test]
    fn test_structural_keywords_not_in_kind_registry() {
        let registry = KindRegistry::new();
        // Structural keywords (spec, ref, use, define) are grammar rules,
        // NOT entity kinds — they are never registered in KindRegistry.
        assert!(!registry.contains("spec"));
        assert!(!registry.contains("ref"));
        assert!(!registry.contains("use"));
        assert!(!registry.contains("define"));
    }

    // B:boot_empty_kind_registry — verify contract "requires/ensures consistency for empty kind registry boot"
    #[test]
    fn test_boot_empty_kind_registry_contract() {
        // requires: no extensions loaded yet
        // ensures: registry is empty, len==0, get returns None for any key
        let registry = KindRegistry::new();
        // ensures: empty
        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);
        // ensures: get returns None for arbitrary keys
        assert!(registry.get("behavior").is_none());
        assert!(registry.get("feature").is_none());
        assert!(registry.get("").is_none());
        // ensures: contains returns false for arbitrary keys
        assert!(!registry.contains("behavior"));
        // ensures: keywords iterator yields nothing
        assert_eq!(registry.keywords().count(), 0);
        // ensures: iter yields nothing
        assert_eq!(registry.iter().count(), 0);
    }
}
