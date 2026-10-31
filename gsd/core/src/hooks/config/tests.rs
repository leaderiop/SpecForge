//! Ported from gsd-core v1.19.0 hooks/gsd-config-reload.js, MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD
//!
//! `config_reload` over the settings of a call (no upstream suite covers the hook).

use super::*;

fn pairs(p: &[(&str, &str)]) -> Vec<(String, String)> {
    p.iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

fn facts(change: &str) -> Facts<'static> {
    Facts {
        change: Some(change.into()),
        ..Facts::default()
    }
}

fn said(change: &str, settings: &Snapshot) -> String {
    config_reload(&facts(change), settings)
        .context
        .expect("a context")
}

#[test]
fn a_change_lists_the_keys_per_family() {
    let settings = Snapshot {
        mode: Some("yolo".into()),
        hooks: pairs(&[("community", "true"), ("workflow_guard", "false")]),
        workflow: pairs(&[("use_worktrees", "false")]),
        ..Snapshot::default()
    };
    assert_eq!(
        said("change", &settings),
        "GSD config reloaded (specforge.json updated):\n  mode: yolo\n  hooks: { community=true, workflow_guard=false }\n  workflow: { use_worktrees=false }"
    );
}

#[test]
fn a_deleted_file_says_so() {
    assert_eq!(
        said("unlink", &Snapshot::default()),
        "GSD config (specforge.json) was deleted. Falling back to built-in defaults for this session."
    );
}

#[test]
fn no_notable_key_still_confirms_the_reload() {
    assert_eq!(
        said("add", &Snapshot::default()),
        "GSD config reloaded (specforge.json updated):\n  (no notable keys changed)"
    );
}
