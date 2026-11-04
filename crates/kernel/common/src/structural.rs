//! The kinds the grammar itself parses, not an extension: a project's root
//! container, an external reference, an import, a definition block.

/// The project's root container.
pub const SPEC: &str = "spec";
/// An external reference (`ref gh.issue:1 "..."`).
pub const REF: &str = "ref";
/// An import.
pub const USE: &str = "use";
/// A definition block (not supported: reported as W143).
pub const DEFINE: &str = "define";

/// File defaults (ADR 0149): a structural form, parsed by the generic entity
/// block, never a node.
pub const DEFAULTS: &str = "defaults";

/// Every structural keyword: never an extension's kind, always reserved as
/// an ID (E013).
pub const KEYWORDS: [&str; 5] = [SPEC, REF, USE, DEFINE, DEFAULTS];

/// The structural keywords that become graph nodes; `use` is an import and
/// `define` is never a node (W143).
pub const NODE_KINDS: [&str; 2] = [REF, SPEC];

/// `keyword` is one of [`KEYWORDS`].
pub fn is_structural(keyword: &str) -> bool {
    KEYWORDS.contains(&keyword)
}

/// Every core command of the `specforge` binary, by its name on the command line: an extension whose short name is
/// one of them has no CLI route (E114), since the binary matches the core command first. The CLI's tests keep it
/// equal to the clap tree.
pub const CORE_COMMANDS: &[&str] = &[
    "init",
    "check",
    "export",
    "schema",
    "model",
    "outline",
    "query",
    "trace",
    "format",
    "stats",
    "explore",
    "review",
    "infer-guide",
    "prompt",
    "add",
    "remove",
    "extensions",
    "publish",
    "search",
    "analyze",
    "watch",
    "new",
    "set",
    "unset",
    "link",
    "unlink",
    "delete",
    "update",
    "login",
    "logout",
    "providers",
    "settings",
    "grants",
    "collect",
    "harness",
    "hook",
    "install",
    "uninstall",
    "doctor",
    "mcp",
    "migrate",
    "completions",
    "explain",
    "infer-status",
    "extension",
    "files",
    "run",
];

/// `name` is one of [`CORE_COMMANDS`].
pub fn is_core_command(name: &str) -> bool {
    CORE_COMMANDS.contains(&name)
}

/// The kind a block's body is read as (ADR 0224): the kind a `defaults <kind>` header names (ADR 0149 D1), else the
/// block's own keyword. The formatter (each value's form) and the editor (completion, hover, tokens) ask it; the
/// graph's defaults module reads a defaults block by its name, the same answer.
pub fn body_kind<'a>(keyword: &'a str, name: Option<&'a str>) -> &'a str {
    match name {
        Some(name) if keyword == DEFAULTS => name,
        _ => keyword,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_structural_keywords_are_the_grammar_forms_and_defaults() {
        assert_eq!(KEYWORDS, ["spec", "ref", "use", "define", "defaults"]);
        assert!(KEYWORDS.iter().all(|k| is_structural(k)));
        assert!(!is_structural("behavior"));
    }

    #[test]
    fn a_defaults_body_is_read_as_the_kind_it_names() {
        assert_eq!(body_kind(DEFAULTS, Some("behavior")), "behavior");
        assert_eq!(body_kind("behavior", Some("b_one")), "behavior");
        assert_eq!(body_kind(DEFAULTS, None), DEFAULTS);
    }
}
