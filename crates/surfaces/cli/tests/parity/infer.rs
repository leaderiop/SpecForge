//! Infer: one progress and gap document on both surfaces.

use super::*;

/// A Rust project half-way through inference: `src/lib.rs` is indexed, `src/net/wire.rs` is not; a
/// `rust-source` store holds their public items, and an anchor covers `alpha` by `behavior alpha`.
fn infer_project(proj: &TestProject) {
    let root = proj.root();
    std::fs::create_dir_all(root.join("spec")).unwrap();
    std::fs::create_dir_all(root.join("src/net")).unwrap();
    std::fs::write(
        root.join("specforge.json"),
        r#"{"name":"inf","spec_root":"spec","extensions":["@specforge/software","@specforge/governance","@code/core"]}"#,
    )
    .unwrap();
    let item = specforge_project::testing::item;
    specforge_project::testing::stored(
        root,
        "rust-source",
        specforge_project::testing::WORKTREE,
        json!([
            item(
                "rust:inf::alpha",
                "src/lib.rs",
                1,
                "function",
                "public",
                "alpha"
            ),
            item(
                "rust:inf::beta",
                "src/lib.rs",
                2,
                "function",
                "public",
                "beta"
            ),
            item(
                "rust:inf::net::Wire",
                "src/net/wire.rs",
                1,
                "struct",
                "public",
                "Wire"
            ),
        ]),
    );
    std::fs::write(
        root.join("specforge-anchors.json"),
        json!({"version": 1, "anchors": [{"entity_id": "alpha", "file": "src/lib.rs", "line": 1,
               "symbol_name": "alpha", "item_kind": "function", "scanner": "rust-source"}]})
        .to_string(),
    )
    .unwrap();
    std::fs::write(root.join("spec/a.spec"), MAIN_SPEC).unwrap();
    std::fs::write(
        root.join("src/lib.rs"),
        "pub fn alpha() {}\npub fn beta() {}\n",
    )
    .unwrap();
    std::fs::write(root.join("src/net/wire.rs"), "pub struct Wire;\n").unwrap();
    std::fs::write(
        root.join("specforge-infer.json"),
        json!({
            "version": 1,
            "source_roots": ["src"],
            "source_index": [{
                "path": "src/lib.rs", "content_hash": "stale",
                "entities_produced": ["alpha"], "analyzed_at": "2026-10-01T00:00:00Z"
            }],
            "sessions": [
                {"session_id": "s-1", "agent": "claude", "status": "completed",
                 "started_at": "2026-10-01T00:00:00Z", "ended_at": "2026-10-01T01:00:00Z"},
                {"session_id": "s-2", "agent": "codex", "status": "active",
                 "started_at": "2026-10-02T00:00:00Z"}
            ]
        })
        .to_string(),
    )
    .unwrap();
}

/// `specforge infer-status --format json` with `flags`.
fn cli_infer_status(project: &TestProject, flags: &[&str]) -> Value {
    let mut argv = vec!["infer-status", "--format", "json"];
    argv.extend(flags);
    let out = cli(project).run(&argv);
    assert_eq!(out.code, 0, "{}{}", out.stdout, out.stderr);
    out.json()
}

#[test]
fn infer_progress_is_equal_on_both_surfaces() {
    let dir = scratch();
    infer_project(&dir);

    let cli_doc = cli_infer_status(&dir, &[]);
    let mcp_doc = mcp_tool(dir.root(), "specforge.infer_progress");

    assert_eq!(
        cli_doc["unanalyzed"],
        json!(["src/net/wire.rs"]),
        "{cli_doc}"
    );
    assert_eq!(cli_doc["stale"], json!(["src/lib.rs"]), "{cli_doc}");
    assert_eq!(
        cli_doc["sessions"],
        json!([
            {"session_id": "s-1", "agent": "claude", "status": "completed",
             "started_at": "2026-10-01T00:00:00Z", "ended_at": "2026-10-01T01:00:00Z"},
            {"session_id": "s-2", "agent": "codex", "status": "active",
             "started_at": "2026-10-02T00:00:00Z"}
        ]),
        "{cli_doc}"
    );
    assert_eq!(cli_doc, mcp_doc);
}

#[specforge_test_macros::test(
    behavior = "provide_infer_status_cli",
    verify = "--format json includes the gaps --gaps and --gaps-detail ask for"
)]
fn infer_status_json_carries_the_gaps_asked_for() {
    let dir = scratch();
    infer_project(&dir);

    let plain = cli_infer_status(&dir, &[]);
    assert!(plain.get("unanalyzed_by_directory").is_none(), "{plain}");
    assert!(plain.get("gap_analysis").is_none(), "{plain}");

    let doc = cli_infer_status(&dir, &["--gaps", "--gaps-detail"]);
    assert_eq!(
        doc["unanalyzed_by_directory"],
        json!([{"directory": "src/net", "count": 1, "files": ["src/net/wire.rs"]}])
    );
    // The gap report is the one specforge.infer_gaps answers with.
    let gaps = &doc["gap_analysis"];
    assert_eq!(gaps, &mcp_tool(dir.root(), "specforge.infer_gaps"));
    let names: Vec<&str> = gaps["by_directory"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|d| d["items"].as_array().unwrap())
        .map(|i| i["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, ["beta", "Wire"], "{gaps}");
    assert_eq!(gaps["covered_items"], 1, "alpha is anchored: {gaps}");
    assert_eq!(gaps["collectors"], json!(["rust-source"]), "{gaps}");
}

/// A store holding only a private item: an item fact the gaps count, with no public one.
#[test]
fn only_private_items_still_count_as_item_facts() {
    let dir = scratch();
    infer_project(&dir);
    specforge_project::testing::stored(
        dir.root(),
        "rust-source",
        specforge_project::testing::WORKTREE,
        json!([specforge_project::testing::item(
            "rust:inf::hidden",
            "src/lib.rs",
            1,
            "function",
            "private",
            "hidden"
        )]),
    );
    let out = cli(&dir).run(&["infer-status", "--gaps-detail"]);
    assert_eq!(out.code, 0, "{}{}", out.stdout, out.stderr);
    assert!(!out.stdout.contains("No item facts"), "{}", out.stdout);
    assert!(out.stdout.contains("  Public items: 0\n"), "{}", out.stdout);
    let gaps = &cli_infer_status(&dir, &["--gaps-detail"])["gap_analysis"];
    assert_eq!(gaps["item_facts"], 1, "{gaps}");
    assert_eq!(gaps, &mcp_tool(dir.root(), "specforge.infer_gaps"));
}
