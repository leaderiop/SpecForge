//! What the nothing-served tests probe: every core entry, with the kind of entry it is.

/// What an entry kind a probe calls.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Entry {
    Tool,
    Prompt,
    Resource,
}

/// Every core tool but init, prompt and resource, with nothing served: its
/// kind, name or URI, arguments as JSON text.
pub const NOTHING_SERVED_PROBES: &[(Entry, &str, &str)] = &[
    (Entry::Tool, "specforge.query", r#"{"entity_id":"x"}"#),
    (Entry::Tool, "specforge.validate", "{}"),
    (Entry::Tool, "specforge.analyze", "{}"),
    (Entry::Tool, "specforge.export", "{}"),
    (Entry::Tool, "specforge.trace", r#"{"entity_id":"x"}"#),
    (Entry::Tool, "specforge.search", r#"{"query":"x"}"#),
    (Entry::Tool, "specforge.explain", r#"{"code":"E003"}"#),
    (Entry::Tool, "specforge.schema", "{}"),
    (Entry::Tool, "specforge.model", "{}"),
    (Entry::Tool, "specforge.outline_extensions", "{}"),
    (Entry::Tool, "specforge.coverage", "{}"),
    (Entry::Tool, "specforge.stats", "{}"),
    (Entry::Tool, "specforge.list", "{}"),
    (Entry::Tool, "specforge.inspect", r#"{"entity_id":"x"}"#),
    (
        Entry::Tool,
        "specforge.find_definition",
        r#"{"entity_id":"x"}"#,
    ),
    (
        Entry::Tool,
        "specforge.find_references",
        r#"{"entity_id":"x"}"#,
    ),
    (Entry::Tool, "specforge.outline", r#"{"file":"a.spec"}"#),
    (Entry::Tool, "specforge.suggest_fixes", "{}"),
    (Entry::Tool, "specforge.format", r#"{"check":true}"#),
    (
        Entry::Tool,
        "specforge.rename",
        r#"{"entity_id":"x","new_name":"y","dry_run":true}"#,
    ),
    (Entry::Tool, "specforge.edit", r#"{"edits":[]}"#),
    (
        Entry::Tool,
        "specforge.add_extension",
        r#"{"specifiers":["@specforge/software"],"dry_run":true}"#,
    ),
    (
        Entry::Tool,
        "specforge.remove_extension",
        r#"{"name":"@specforge/software","dry_run":true}"#,
    ),
    (Entry::Tool, "specforge.migrate", r#"{"dry_run":true}"#),
    (Entry::Tool, "specforge.extensions", "{}"),
    (Entry::Tool, "specforge.providers", "{}"),
    (Entry::Tool, "specforge.settings", "{}"),
    (
        Entry::Tool,
        "specforge.settings_set",
        r#"{"extension":"@specforge/software","name":"x","unset":true,"dry_run":true}"#,
    ),
    (
        Entry::Tool,
        "specforge.verdict_inputs",
        r#"{"entity":"x","rule":"nothing"}"#,
    ),
    (Entry::Tool, "specforge.doctor", "{}"),
    (Entry::Tool, "specforge.collect", "{}"),
    (Entry::Tool, "specforge.render", r#"{"format":"brief"}"#),
    (Entry::Tool, "specforge.infer_progress", "{}"),
    (Entry::Tool, "specforge.infer_gaps", "{}"),
    (Entry::Tool, "specforge.prompt", r#"{"name":"review"}"#),
    (
        Entry::Tool,
        "specforge.infer_session",
        r#"{"action":"start"}"#,
    ),
    (
        Entry::Tool,
        "specforge.find_implementation",
        r#"{"entity_id":"x"}"#,
    ),
    (
        Entry::Tool,
        "specforge.find_spec_for_source",
        r#"{"file_path":"src/lib.rs"}"#,
    ),
    (
        Entry::Prompt,
        "specforge://prompts/context",
        r#"{"entity_id":"x"}"#,
    ),
    (Entry::Prompt, "specforge://prompts/review", "{}"),
    (
        Entry::Prompt,
        "specforge://prompts/trace",
        r#"{"entity_id":"x"}"#,
    ),
    (Entry::Prompt, "specforge://prompts/explore", "{}"),
    (Entry::Prompt, "specforge://prompts/infer", "{}"),
    (Entry::Resource, "specforge://graph", "{}"),
    (Entry::Resource, "specforge://schema", "{}"),
    (Entry::Resource, "specforge://context", "{}"),
    (Entry::Resource, "specforge://context/x", "{}"),
    (Entry::Resource, "specforge://brief", "{}"),
    (Entry::Resource, "specforge://diagnostics", "{}"),
    (Entry::Resource, "specforge://graph/x", "{}"),
    (Entry::Resource, "specforge://entities/feature", "{}"),
];
