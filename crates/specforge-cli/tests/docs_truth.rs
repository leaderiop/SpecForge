//! C2-06: entity-kind counts in the docs are derived claims — they rot
//! silently when an extension adds a kind. Derive the number from the
//! builtin manifests and pin the docs to it.

use std::collections::BTreeSet;

fn repo_root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repo root")
}

fn manifest_domain_kinds() -> BTreeSet<String> {
    let mut kinds = BTreeSet::new();
    for entry in std::fs::read_dir(repo_root().join("extensions")).expect("extensions dir") {
        let path = entry.unwrap().path().join("src/describe_entities.json");
        // Registry-language extensions (e.g. typescript) have no Rust
        // describe fixtures; only the four builtins ship them.
        if !path.exists() {
            continue;
        }
        let raw =
            std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        let doc: serde_json::Value = serde_json::from_str(&raw).expect("valid describe JSON");
        for item in doc["items"].as_array().expect("items array") {
            let keyword = item["keyword"]
                .as_str()
                .or_else(|| item["name"].as_str())
                .expect("kind keyword");
            kinds.insert(keyword.to_string());
        }
    }
    kinds
}

/// The canonical repository URL is defined once, as the workspace's Cargo
/// `repository` (ADR 0004 D6-b); the VS Code manifest can't inherit it, so
/// it is pinned to it here.
#[test]
fn vscode_manifest_names_the_workspace_repository() {
    let raw = std::fs::read_to_string(repo_root().join("integrations/vscode/package.json"))
        .expect("integrations/vscode/package.json");
    let manifest: serde_json::Value = serde_json::from_str(&raw).expect("valid package.json");
    assert_eq!(
        manifest["repository"]["url"].as_str(),
        Some(env!("CARGO_PKG_REPOSITORY"))
    );
}

#[test]
fn entity_model_doc_count_matches_builtin_manifests() {
    let kinds = manifest_domain_kinds();
    let count = kinds.len();
    assert_eq!(
        count, 22,
        "builtin manifest kinds drifted; update docs/entity-model.md"
    );

    let doc = std::fs::read_to_string(repo_root().join("docs/entity-model.md"))
        .expect("docs/entity-model.md");
    let claimed = format!("{count}, declared by the four builtin extensions");
    assert!(
        doc.contains(&claimed),
        "entity-model.md must claim '{claimed}' — the manifest-derived count changed"
    );
    let total = 2 + count;
    assert!(
        doc.contains(&format!("= {total} entity kinds")),
        "entity-model.md total must equal 2 structural + {count} domain = {total}"
    );
}
