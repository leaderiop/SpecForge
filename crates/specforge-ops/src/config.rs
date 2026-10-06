//! The one writer of `specforge.json`.
//!
//! Every surface that changes the project config goes through here, so the
//! file always comes out in one style (pretty-printed, trailing newline)
//! and an extension entry is matched by its exact name, never a prefix.

use crate::OpError;
use serde_json::Value;
use std::path::Path;

/// The config file's name, at the project root.
pub const CONFIG_FILE: &str = "specforge.json";

/// The extension an `extensions` entry names: `name` or `name@version`
/// (`@acme/foo@1.2.0` names `@acme/foo`). A path entry is returned whole.
pub fn entry_name(entry: &str) -> &str {
    specforge_common::extension_entry_name(entry)
}

/// Write `config` as the project's `specforge.json`: pretty-printed, with
/// a trailing newline.
pub fn write(root: &Path, config: &Value) -> Result<(), OpError> {
    let path = root.join(CONFIG_FILE);
    let text = serde_json::to_string_pretty(config)
        .map_err(|e| OpError::new("config_invalid", e.to_string()))?;
    std::fs::write(&path, text + "\n").map_err(|e| {
        OpError::new(
            "config_write_failed",
            format!("failed to write {}: {e}", path.display()),
        )
    })
}

/// Apply `edit` to the `extensions` array of the project's
/// `specforge.json` (created empty when absent). The file is rewritten
/// only when `edit` returns `true`; the result is that flag.
pub fn edit_extensions(
    root: &Path,
    edit: impl FnOnce(&mut Vec<Value>) -> bool,
) -> Result<bool, OpError> {
    let path = root.join(CONFIG_FILE);
    let content = std::fs::read_to_string(&path).map_err(|_| {
        OpError::new(
            "config_not_found",
            format!("no {CONFIG_FILE} in {}", root.display()),
        )
        .with_suggestion("run `specforge init` first")
    })?;
    let mut config: Value = serde_json::from_str(&content).map_err(|e| {
        OpError::new(
            "config_invalid",
            format!("{} is not valid JSON: {e}", path.display()),
        )
    })?;
    let object = config.as_object_mut().ok_or_else(|| {
        OpError::new(
            "config_invalid",
            format!("{} must be a JSON object", path.display()),
        )
    })?;
    let extensions = object
        .entry("extensions")
        .or_insert_with(|| Value::Array(Vec::new()))
        .as_array_mut()
        .ok_or_else(|| {
            OpError::new(
                "config_invalid",
                format!("\"extensions\" in {} must be an array", path.display()),
            )
        })?;

    let changed = edit(extensions);
    if changed {
        write(root, &config)?;
    }
    Ok(changed)
}

/// Whether `extensions` has an entry naming exactly `name`.
pub fn has_extension(extensions: &[Value], name: &str) -> bool {
    extensions
        .iter()
        .any(|e| e.as_str().is_some_and(|s| entry_name(s) == name))
}

/// Append `entry` (`name` or `name@version`) to the project's extensions
/// unless an entry already names `name`. `Ok(false)` when one did.
pub fn add_extension(root: &Path, name: &str, entry: &str) -> Result<bool, OpError> {
    edit_extensions(root, |extensions| {
        if has_extension(extensions, name) {
            return false;
        }
        extensions.push(Value::from(entry));
        true
    })
}

/// Drop every entry naming `name` from the project's extensions.
/// `Ok(false)` when none did.
pub fn remove_extension(root: &Path, name: &str) -> Result<bool, OpError> {
    edit_extensions(root, |extensions| {
        let before = extensions.len();
        extensions.retain(|e| e.as_str().is_none_or(|s| entry_name(s) != name));
        extensions.len() != before
    })
}

/// Drop the entries written exactly `entry` (surrounding whitespace
/// ignored) from the project's extensions. `Ok(false)` when none was.
pub fn remove_entry(root: &Path, entry: &str) -> Result<bool, OpError> {
    let entry = entry.trim();
    edit_extensions(root, |extensions| {
        let before = extensions.len();
        extensions.retain(|e| e.as_str().is_none_or(|s| s.trim() != entry));
        extensions.len() != before
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use specforge_test_macros::test as specforge_test;

    fn project(config: &str) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(CONFIG_FILE), config).unwrap();
        dir
    }

    fn extensions(root: &Path) -> Vec<String> {
        let text = std::fs::read_to_string(root.join(CONFIG_FILE)).unwrap();
        let config: Value = serde_json::from_str(&text).unwrap();
        config["extensions"]
            .as_array()
            .unwrap()
            .iter()
            .map(|e| e.as_str().unwrap().to_string())
            .collect()
    }

    #[specforge_test(
        behavior = "add_extension_to_existing_project",
        verify = "add extension appends to extensions list"
    )]
    fn add_appends_an_extension_whose_name_prefixes_an_enabled_one() {
        let dir = project(r#"{"name":"p","extensions":["@acme/foobar@1.0.0"]}"#);

        assert_eq!(
            add_extension(dir.path(), "@acme/foo", "@acme/foo@2.0.0"),
            Ok(true)
        );
        assert_eq!(
            extensions(dir.path()),
            ["@acme/foobar@1.0.0", "@acme/foo@2.0.0"]
        );
    }

    #[test]
    fn add_leaves_an_extension_already_enabled_alone() {
        let dir = project(r#"{"name":"p","extensions":["@acme/foo@1.0.0"]}"#);
        let before = std::fs::read(dir.path().join(CONFIG_FILE)).unwrap();

        assert_eq!(
            add_extension(dir.path(), "@acme/foo", "@acme/foo@2.0.0"),
            Ok(false)
        );
        assert_eq!(std::fs::read(dir.path().join(CONFIG_FILE)).unwrap(), before);
    }

    #[test]
    fn remove_drops_only_the_exact_name() {
        let dir =
            project(r#"{"name":"p","extensions":["@acme/foo@1.0.0","@acme/foobar","@acme/foo"]}"#);

        assert_eq!(remove_extension(dir.path(), "@acme/foo"), Ok(true));
        assert_eq!(extensions(dir.path()), ["@acme/foobar"]);
        assert_eq!(remove_extension(dir.path(), "@acme/foo"), Ok(false));
    }

    #[test]
    fn entry_name_strips_only_a_version_suffix() {
        assert_eq!(entry_name("@acme/foo"), "@acme/foo");
        assert_eq!(entry_name("@acme/foo@1.2.0"), "@acme/foo");
        assert_eq!(
            entry_name(" @specforge/formal@latest "),
            "@specforge/formal"
        );
        assert_eq!(entry_name("plain@0.0.0"), "plain");
        assert_eq!(entry_name("./ext/local.wasm"), "./ext/local.wasm");
        assert_eq!(entry_name("./ext@2/local.wasm"), "./ext@2/local.wasm");
    }

    #[test]
    fn edits_write_pretty_json_with_a_trailing_newline_and_keep_other_fields() {
        let dir = project(r#"{"name":"p","version":"0.1.0","spec_root":"spec"}"#);

        assert_eq!(
            add_extension(dir.path(), "@acme/foo", "@acme/foo"),
            Ok(true)
        );
        let text = std::fs::read_to_string(dir.path().join(CONFIG_FILE)).unwrap();
        assert_eq!(
            text,
            "{\n  \"extensions\": [\n    \"@acme/foo\"\n  ],\n  \"name\": \"p\",\n  \"spec_root\": \"spec\",\n  \"version\": \"0.1.0\"\n}\n"
        );
    }

    #[test]
    fn an_edit_without_a_project_says_to_init() {
        let dir = tempfile::tempdir().unwrap();
        let err = add_extension(dir.path(), "@acme/foo", "@acme/foo").unwrap_err();
        assert_eq!(err.code, "config_not_found");
        assert!(
            err.suggestion
                .as_deref()
                .unwrap()
                .contains("specforge init"),
            "{err:?}"
        );
    }

    #[test]
    fn an_edit_refuses_a_config_whose_extensions_are_not_an_array() {
        let dir = project(r#"{"name":"p","extensions":"@acme/foo"}"#);
        let err = add_extension(dir.path(), "@acme/bar", "@acme/bar").unwrap_err();
        assert_eq!(err.code, "config_invalid");
    }
}
