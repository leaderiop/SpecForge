use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Locate the project root by walking from `start` upward to the filesystem root.
///
/// At each directory level, checks for `specforge.json` (preferred) then `specforge.spec`.
/// The first directory containing either file wins (closest-wins semantics).
/// Symlinks are resolved before comparison to avoid infinite loops.
///
/// Returns `None` if neither file is found in any ancestor.
pub fn find_project_root(start: &Path) -> Option<PathBuf> {
    let mut current = start.canonicalize().ok()?;

    loop {
        if current.join("specforge.json").exists() || current.join("specforge.spec").exists() {
            return Some(current);
        }
        if !current.pop() {
            return None;
        }
    }
}

/// What one `specforge.json` `extensions` entry enables: the one reading
/// of an entry that the runtime loading extensions, the environment
/// reading their declarations, the freshness inputs and the extension
/// operations share.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExtensionEntry<'a> {
    /// A builtin or an installed extension, by its name: `name`, or
    /// `name@version` as older `specforge add`s wrote it (`@acme/foo@1.2.0`
    /// names `@acme/foo`).
    Named(&'a str),
    /// A component loaded from a `.wasm` file: `path.wasm`, or
    /// `name=path.wasm` where `name` must be the name the component
    /// declares. A relative path is relative to the project root. The
    /// extension is the one the component declares (its handshake `name`).
    File {
        /// The name written before `=`, if any.
        name: Option<&'a str>,
        /// The path as written.
        path: &'a str,
    },
}

impl<'a> ExtensionEntry<'a> {
    /// Read an entry (surrounding whitespace ignored): one ending in
    /// `.wasm` names a file, any other an extension.
    pub fn parse(entry: &'a str) -> Self {
        let entry = entry.trim();
        if entry.ends_with(".wasm") {
            return match entry.split_once('=') {
                Some((name, path)) => ExtensionEntry::File {
                    name: Some(name.trim()),
                    path: path.trim(),
                },
                None => ExtensionEntry::File {
                    name: None,
                    path: entry,
                },
            };
        }
        match entry.rfind('@') {
            Some(at) if at > 0 && !entry[at + 1..].contains('/') => {
                ExtensionEntry::Named(&entry[..at])
            }
            _ => ExtensionEntry::Named(entry),
        }
    }

    /// The extension's name as far as the entry alone says: a named
    /// entry's name, a file entry's name before `=`; `None` for a bare
    /// file (only its component says).
    pub fn name(&self) -> Option<&'a str> {
        match *self {
            ExtensionEntry::Named(name) => Some(name),
            ExtensionEntry::File { name, .. } => name,
        }
    }

    /// The file a file entry loads, a relative path resolved against
    /// `root` (the project root).
    pub fn file(&self, root: &Path) -> Option<PathBuf> {
        match *self {
            ExtensionEntry::Named(_) => None,
            ExtensionEntry::File { path, .. } => Some(root.join(path)),
        }
    }
}

/// The extension a `specforge.json` `extensions` entry names, as far as
/// the entry alone says ([`ExtensionEntry`]): `name`, or `name@version`
/// (`@acme/foo@1.2.0` names `@acme/foo`); for a `.wasm` file entry the
/// name before `=`, else the path as written.
pub fn extension_entry_name(entry: &str) -> &str {
    match ExtensionEntry::parse(entry) {
        ExtensionEntry::Named(name)
        | ExtensionEntry::File {
            name: Some(name), ..
        } => name,
        ExtensionEntry::File { name: None, path } => path,
    }
}

/// Parsed project configuration from specforge.json.
#[derive(Debug, Clone, Default)]
pub struct ProjectConfig {
    pub name: Option<String>,
    pub version: Option<String>,
    pub spec_root: Option<String>,
    pub extensions: Vec<String>,
    /// Path substrings excluded from `.spec` discovery (C4-04): matched
    /// against the workspace-relative path by the shared discovery walker.
    pub exclude: Vec<String>,
    pub inference: InferenceConfig,
    pub raw: Option<serde_json::Value>,
}

/// Project-level inference hints that override/append to extension defaults.
#[derive(Debug, Clone, Default)]
pub struct InferenceConfig {
    pub global: Option<String>,
    pub kinds: HashMap<String, String>,
    pub density_threshold: Option<f64>,
}

/// Load project configuration from specforge.json in the given directory.
/// Returns a default config if the file doesn't exist.
pub fn load_project_config(project_root: &Path) -> ProjectConfig {
    let config_path = project_root.join("specforge.json");
    let content = match std::fs::read_to_string(&config_path) {
        Ok(c) => c,
        Err(_) => return ProjectConfig::default(),
    };
    let value: serde_json::Value = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(_) => return ProjectConfig::default(),
    };

    let name = value
        .get("name")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let version = value
        .get("version")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let spec_root = value
        .get("spec_root")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let extensions = value
        .get("extensions")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();
    let exclude = value
        .get("exclude")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();

    let inference = parse_inference_config(&value);

    ProjectConfig {
        name,
        version,
        spec_root,
        extensions,
        exclude,
        inference,
        raw: Some(value),
    }
}

fn parse_inference_config(value: &serde_json::Value) -> InferenceConfig {
    let obj = match value.get("inference").and_then(|v| v.as_object()) {
        Some(o) => o,
        None => return InferenceConfig::default(),
    };

    let global = obj
        .get("global")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let mut kinds = HashMap::new();
    for (key, val) in obj {
        if key == "global" {
            continue;
        }
        if let Some(s) = val.as_str() {
            kinds.insert(key.clone(), s.to_string());
        }
    }

    let density_threshold = obj.get("density_threshold").and_then(|v| v.as_f64());

    InferenceConfig {
        global,
        kinds,
        density_threshold,
    }
}

/// Why `name` can't name a project, or `Ok`: it must be non-empty, at most
/// 214 characters, free of whitespace, and not start with `.` or `-`.
pub fn validate_project_name(name: &str) -> Result<(), &'static str> {
    if name.is_empty() {
        return Err("name must not be empty");
    }
    if name.len() > 214 {
        return Err("name must not exceed 214 characters");
    }
    if name.starts_with('.') || name.starts_with('-') {
        return Err("name must not start with '.' or '-'");
    }
    if name.contains(|c: char| c.is_whitespace()) {
        return Err("name must not contain whitespace");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_entry_names_an_extension_or_a_wasm_file() {
        use ExtensionEntry::{File, Named};
        let cases = [
            ("@specforge/product", Named("@specforge/product")),
            (" @acme/foo@1.2.0 ", Named("@acme/foo")),
            ("greet", Named("greet")),
            (
                "greet.wasm",
                File {
                    name: None,
                    path: "greet.wasm",
                },
            ),
            (
                "./ext/greet.wasm",
                File {
                    name: None,
                    path: "./ext/greet.wasm",
                },
            ),
            (
                "/a@b/greet.wasm",
                File {
                    name: None,
                    path: "/a@b/greet.wasm",
                },
            ),
            (
                "@sdk/greet = ext/greet.wasm",
                File {
                    name: Some("@sdk/greet"),
                    path: "ext/greet.wasm",
                },
            ),
        ];
        for (entry, expected) in cases {
            assert_eq!(ExtensionEntry::parse(entry), expected, "{entry}");
        }
    }

    #[test]
    fn a_file_entry_resolves_against_the_root_and_names_what_it_writes() {
        let root = Path::new("/p");
        let bare = ExtensionEntry::parse("ext/greet.wasm");
        assert_eq!(bare.file(root), Some(PathBuf::from("/p/ext/greet.wasm")));
        assert_eq!(bare.name(), None);
        assert_eq!(extension_entry_name("ext/greet.wasm"), "ext/greet.wasm");

        let named = ExtensionEntry::parse("@sdk/greet=/abs/greet.wasm");
        assert_eq!(named.file(root), Some(PathBuf::from("/abs/greet.wasm")));
        assert_eq!(named.name(), Some("@sdk/greet"));
        assert_eq!(
            extension_entry_name("@sdk/greet=/abs/greet.wasm"),
            "@sdk/greet"
        );

        assert_eq!(ExtensionEntry::parse("@acme/foo@1.0.0").file(root), None);
    }
}
