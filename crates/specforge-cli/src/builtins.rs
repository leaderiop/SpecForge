//! Builtin extensions ship inside the binary: enabling one is an entry in
//! `specforge.json`'s `extensions` array, not a download or a lock entry.

use specforge_component::builtins::BUILTIN_EXTENSIONS;
use std::path::Path;

/// The builtin named by `specifier` (`@specforge/product`, optionally with an
/// `@version` suffix, which is ignored — builtins track the binary's version).
pub fn builtin_name(specifier: &str) -> Option<&'static str> {
    let specifier = specifier.trim();
    let name = match specifier.rfind('@') {
        Some(at) if at > 0 => &specifier[..at],
        _ => specifier,
    };
    BUILTIN_EXTENSIONS
        .iter()
        .map(|(builtin, _)| *builtin)
        .find(|builtin| *builtin == name)
}

/// Builtins that `name` requires (its non-optional peer dependencies that
/// are themselves builtins), read from its handshake.
pub fn required_peers(name: &str) -> Vec<&'static str> {
    let runtime = specforge_component::ComponentRuntime::new();
    if specforge_component::builtins::load_builtins_for(&runtime, &[name.to_string()]).is_err() {
        return Vec::new();
    }
    let host = specforge_wasm::protocol::ProtocolHost::new(&runtime);
    let Ok(handshake) = host.handshake(name) else {
        return Vec::new();
    };
    handshake
        .peer_dependencies
        .iter()
        .filter(|peer| !peer.optional)
        .filter_map(|peer| builtin_name(&peer.name))
        .collect()
}

/// Builtins enabled in the project's `specforge.json`, in declaration order.
pub fn enabled(project: &Path) -> Vec<&'static str> {
    specforge_common::load_project_config(project)
        .extensions
        .iter()
        .filter_map(|entry| builtin_name(entry))
        .collect()
}

/// Add `name` to `specforge.json`. `Ok(false)` when it was already enabled.
pub fn enable(project: &Path, name: &str) -> Result<bool, String> {
    edit_extensions(project, |extensions| {
        if extensions
            .iter()
            .any(|e| e.as_str().and_then(builtin_name) == Some(name))
        {
            return false;
        }
        extensions.push(serde_json::Value::from(name));
        true
    })
}

/// Remove `name` from `specforge.json`. `Ok(false)` when it was not enabled.
pub fn disable(project: &Path, name: &str) -> Result<bool, String> {
    edit_extensions(project, |extensions| {
        let before = extensions.len();
        extensions.retain(|e| e.as_str().and_then(builtin_name) != Some(name));
        extensions.len() != before
    })
}

fn edit_extensions(
    project: &Path,
    edit: impl FnOnce(&mut Vec<serde_json::Value>) -> bool,
) -> Result<bool, String> {
    let config_path = project.join("specforge.json");
    let content = std::fs::read_to_string(&config_path).map_err(|_| {
        format!(
            "no specforge.json in {} — run `specforge init` first",
            project.display()
        )
    })?;
    let mut json: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| format!("{} is not valid JSON: {e}", config_path.display()))?;
    let object = json
        .as_object_mut()
        .ok_or_else(|| format!("{} must be a JSON object", config_path.display()))?;
    let extensions = object
        .entry("extensions")
        .or_insert_with(|| serde_json::json!([]))
        .as_array_mut()
        .ok_or_else(|| "\"extensions\" in specforge.json must be an array".to_string())?;

    let changed = edit(extensions);
    if changed {
        let pretty = serde_json::to_string_pretty(&json).map_err(|e| e.to_string())?;
        std::fs::write(&config_path, pretty + "\n")
            .map_err(|e| format!("failed to write {}: {e}", config_path.display()))?;
    }
    Ok(changed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_name_accepts_bare_and_versioned_names() {
        assert_eq!(
            builtin_name("@specforge/product"),
            Some("@specforge/product")
        );
        assert_eq!(
            builtin_name("@specforge/formal@latest"),
            Some("@specforge/formal")
        );
        assert_eq!(builtin_name("@acme/thing"), None);
        assert_eq!(builtin_name("./local.wasm"), None);
    }

    #[test]
    fn enable_and_disable_edit_specforge_json_idempotently() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("specforge.json"),
            r#"{"name":"p","extensions":["@specforge/software"]}"#,
        )
        .unwrap();

        assert_eq!(enable(dir.path(), "@specforge/product"), Ok(true));
        assert_eq!(enable(dir.path(), "@specforge/product"), Ok(false));
        assert_eq!(
            enabled(dir.path()),
            ["@specforge/software", "@specforge/product"]
        );

        assert_eq!(disable(dir.path(), "@specforge/software"), Ok(true));
        assert_eq!(disable(dir.path(), "@specforge/software"), Ok(false));
        assert_eq!(enabled(dir.path()), ["@specforge/product"]);
    }

    #[test]
    fn enable_without_a_project_says_to_init() {
        let dir = tempfile::tempdir().unwrap();
        let err = enable(dir.path(), "@specforge/product").unwrap_err();
        assert!(err.contains("specforge init"), "{err}");
    }
}
