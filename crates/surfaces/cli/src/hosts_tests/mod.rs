//! `specforge install`, `uninstall` and `hook` over the GSD-shaped host fixture, served in process (ADR 0292). The
//! two helpers [`install`] and [`hook`] are the one place a test calls the command line; the tickets that add the
//! commands fill them in, and until then the pins that use them are ignored.

#![allow(
    clippy::unimplemented,
    reason = "a K9 pin's helper stands unimplemented until the ticket that adds what it calls"
)]

mod golden;
mod hook;
mod install;

use std::collections::BTreeMap;

use serde_json::{Value, json};
use specforge_project::testing::{CORE, GSD_HOSTS_EXTENSIONS, TestProject, UI, gsd_hosts_sources};

/// A command's exit code and what it printed.
pub struct Out {
    pub code: i32,
    pub stdout: String,
    pub stderr: String,
}

impl Out {
    /// Stdout as JSON (`--format json`).
    pub fn json(&self) -> Value {
        serde_json::from_str(&self.stdout).unwrap_or_else(|e| panic!("{e}: {}", self.stdout))
    }

    /// The action install took for `path` on any host: `created`, `updated`, `unchanged` or `removed`.
    pub fn action(&self, path: &str) -> Option<String> {
        self.json()["hosts"]
            .as_array()?
            .iter()
            .flat_map(|h| h["files"].as_array().into_iter().flatten())
            .find(|f| f["path"] == path)
            .and_then(|f| f["action"].as_str().map(str::to_string))
    }
}

/// A project of the fixture's extensions with `core` settings for `@t/core` and `ui` for `@t/ui`.
pub fn project(core: Value, ui: Value) -> TestProject {
    let project = TestProject::new()
        .config(|c| {
            c["name"] = "p".into();
            c["settings"] = json!({ CORE: core, UI: ui });
        })
        .enabling(&GSD_HOSTS_EXTENSIONS)
        .installing_enabled();
    for (path, text) in gsd_hosts_sources() {
        project.write(path, text);
    }
    project
}

/// Every file of the project but its sources and the hidden state, by relative path.
pub fn tree(project: &TestProject) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    fn walk(root: &std::path::Path, at: &std::path::Path, out: &mut BTreeMap<String, String>) {
        for entry in std::fs::read_dir(at).unwrap() {
            let path = entry.unwrap().path();
            let relative = path
                .strip_prefix(root)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/");
            if relative.starts_with(".specforge")
                || relative.starts_with(".git")
                || relative == "specforge.json"
            {
                continue;
            }
            if path.is_dir() {
                walk(root, &path, out);
            } else {
                out.insert(relative, std::fs::read_to_string(&path).unwrap_or_default());
            }
        }
    }
    walk(project.root(), project.root(), &mut out);
    out
}

/// `specforge install <hosts…> <flags…> --format json` over `project`.
pub fn install(_project: &TestProject, _hosts: &[&str], _flags: &[&str]) -> Out {
    unimplemented!("K9 T9: specforge install")
}

/// `specforge uninstall <hosts…> <flags…> --format json` over `project`.
pub fn uninstall(_project: &TestProject, _hosts: &[&str], _flags: &[&str]) -> Out {
    unimplemented!("K9 T9: specforge uninstall")
}

/// What `specforge hook` did: its exit code and output, and the exports the guest was asked for.
pub struct Hooked {
    pub out: Out,
    pub exports: Vec<String>,
}

/// `specforge hook t.<id> --host <host> --event <event> …` over `project` with `stdin`.
pub fn hook(
    _project: &TestProject,
    _id: &str,
    _host: &str,
    _args: &[&str],
    _stdin: &Value,
) -> Hooked {
    unimplemented!("K9 T10: specforge hook")
}

/// `@t/ui`'s name, as a setting block key.
pub const UI_EXTENSION: &str = UI;
