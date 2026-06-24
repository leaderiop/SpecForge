/// E2E pipeline tests: parse → resolve → graph → validate → emit
/// These tests exercise the full compilation pipeline from spec source files
/// to emitted output, verifying roundtrip integrity.

use specforge_test::prelude::*;
use tempfile::TempDir;
use std::fs;

/// Helper: write spec files to a temp dir and run the simple compilation pipeline.
fn compile_specs(files: &[(&str, &str)]) -> specforge_emitter::compile::CompilationContext {
    let dir = TempDir::new().unwrap();
    for (name, content) in files {
        let path = dir.path().join(name);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(&path, content).unwrap();
    }
    specforge_emitter::compile_simple(dir.path())
}

/// Helper: compile with the builtin runtime for the given extensions (full pipeline).
fn compile_with_builtins(
    extensions: &[&str],
    files: &[(&str, &str)],
) -> specforge_emitter::compile::CompilationContext {
    let dir = TempDir::new().unwrap();
    let ext_json = extensions
        .iter()
        .map(|e| format!("\"{e}\""))
        .collect::<Vec<_>>()
        .join(", ");
    fs::write(
        dir.path().join("specforge.json"),
        format!(
            r#"{{ "name": "t", "version": "0.1.0", "spec_root": "spec", "extensions": [{ext_json}] }}"#
        ),
    )
    .unwrap();
    for (name, content) in files {
        let path = dir.path().join("spec").join(name);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(&path, content).unwrap();
    }
    let runtime = specforge_emitter::builtins::runtime_for_extensions(
        &extensions.iter().map(|s| s.to_string()).collect::<Vec<_>>(),
    );
    specforge_emitter::compile_with_runtime(dir.path(), Some(&runtime))
}

// B:compile_pipeline — verify unit "port method body syntax does not surface parse errors"
#[test]
#[specforge_test(behavior = "compile_pipeline", verify = "extension-owned body syntax (port methods) does not surface E001")]
fn port_method_body_does_not_surface_parse_errors() {
    let ctx = compile_with_builtins(
        &["@specforge/software"],
        &[(
            "main.spec",
            r#"spec "t" { version "0.1.0" }
type Task "T" { id string }
port TaskRepository "Repo" {
  direction outbound
  category  "persistence/task"
  method create(input: Task) -> Result<Task, never>
  method findById(id: string) -> Result<Task, never>
  verify integration "contract satisfied"
}
"#,
        )],
    );

    // Port method signatures are extension-owned body syntax. The core grammar
    // cannot parse them, but they must NOT surface as E001 parse errors to the
    // user — the `port` entity itself parses fine (direction, category, verify).
    let e001: Vec<_> = ctx.diagnostics.iter().filter(|d| d.code == "E001").collect();
    assert!(
        e001.is_empty(),
        "port method body must not produce E001 parse errors, got: {:?}",
        e001
    );
}

// B:compile_pipeline — verify unit "single entity roundtrip: parse→graph→json"
#[test]
#[specforge_test(behavior = "compile_pipeline", verify = "single entity roundtrip produces valid graph")]
fn single_entity_roundtrip() {
    let ctx = compile_specs(&[(
        "core.spec",
        r#"behavior login "User Login" {
    status planned
    contract "The system MUST authenticate users"
}
"#,
    )]);

    // Graph should contain exactly one node
    assert_eq!(ctx.graph.nodes().len(), 1, "expected 1 node, got {}", ctx.graph.nodes().len());
    let node = &ctx.graph.nodes()[0];
    assert_eq!(node.id.raw.as_str(), "login");
    assert_eq!(node.kind.raw.as_str(), "behavior");
    assert_eq!(node.title.as_deref(), Some("User Login"));

    // Emit as JSON — should produce valid JSON
    let json = specforge_emitter::emit_json(&ctx.graph);
    let parsed: serde_json::Value = serde_json::from_str(&json).expect("emitted JSON must be valid");
    let nodes = parsed["nodes"].as_array().expect("must have nodes array");
    assert_eq!(nodes.len(), 1);
    assert_eq!(nodes[0]["id"].as_str().unwrap(), "login");
}

// B:compile_pipeline — verify unit "multi-file resolution with imports"
#[test]
#[specforge_test(behavior = "compile_pipeline", verify = "multi-file resolution with imports")]
fn multi_file_with_imports() {
    let ctx = compile_specs(&[
        ("types.spec", r#"type user_id "User ID" {
    format "UUID"
}
"#),
        ("behaviors.spec", r#"use "types"

behavior login "User Login" {
    status planned
    types [user_id]
}
"#),
    ]);

    // Should have 2 nodes
    assert_eq!(ctx.graph.nodes().len(), 2, "expected 2 nodes");
    // Should have resolved the import (no E025 file-not-found errors)
    let file_errors: Vec<_> = ctx.diagnostics.iter().filter(|d| d.code == "E025").collect();
    assert!(file_errors.is_empty(), "should not have file errors: {:?}", file_errors);
}

// B:compile_pipeline — verify unit "cross-entity references produce edges"
#[test]
#[specforge_test(behavior = "compile_pipeline", verify = "cross-entity references produce edges")]
fn cross_entity_references_produce_edges() {
    let ctx = compile_specs(&[(
        "core.spec",
        r#"invariant data_integrity "Data Integrity" {
    contract "Data MUST be consistent"
}

behavior save_record "Save Record" {
    status planned
    invariants [data_integrity]
}
"#,
    )]);

    assert_eq!(ctx.graph.nodes().len(), 2);
    assert!(!ctx.graph.edges().is_empty(), "should have at least one edge from behavior to invariant");

    // Check edge connects the right nodes
    let edge = &ctx.graph.edges()[0];
    assert_eq!(edge.source.as_str(), "save_record");
    assert_eq!(edge.target.as_str(), "data_integrity");
}

// B:compile_pipeline — verify unit "validation diagnostics surface through pipeline"
#[test]
#[specforge_test(behavior = "compile_pipeline", verify = "validation diagnostics surface through pipeline")]
fn validation_diagnostics_surface() {
    let ctx = compile_specs(&[(
        "core.spec",
        r#"behavior broken "Broken" {
    status planned
    invariants [nonexistent_invariant]
}
"#,
    )]);

    // Should have a diagnostic about the unresolved reference
    let has_ref_diag = ctx.diagnostics.iter().any(|d| {
        d.message.contains("nonexistent_invariant") || d.code.starts_with("W")
    });
    assert!(has_ref_diag, "expected diagnostic about unresolved reference, got: {:?}", ctx.diagnostics);
}

// B:compile_pipeline — verify unit "empty project produces empty graph"
#[test]
#[specforge_test(behavior = "compile_pipeline", verify = "empty project produces empty graph")]
fn empty_project_produces_empty_graph() {
    let dir = TempDir::new().unwrap();
    let ctx = specforge_emitter::compile_simple(dir.path());

    assert!(ctx.graph.nodes().is_empty(), "empty project should have no nodes");
    assert!(ctx.graph.edges().is_empty(), "empty project should have no edges");
}

// (Removed: surfaces_flow_through_compilation_context — tested manifest.json surface loading
// which is no longer supported. Surface wiring is tested in MCP surface_wiring tests.)

// B:compile_pipeline — verify unit "all emit formats work on pipeline output"
#[test]
#[specforge_test(behavior = "compile_pipeline", verify = "all emit formats work on pipeline output")]
fn all_emit_formats_work() {
    let ctx = compile_specs(&[(
        "core.spec",
        r#"behavior auth "Authentication" {
    status planned
    contract "Users MUST be authenticated"

    verify unit "credentials are validated"
}

invariant security "Security Invariant" {
    contract "All endpoints MUST be authenticated"
}
"#,
    )]);

    // JSON format
    let json = specforge_emitter::emit_json(&ctx.graph);
    assert!(serde_json::from_str::<serde_json::Value>(&json).is_ok(), "JSON must be valid");

    // Brief format
    let brief = specforge_emitter::emit_brief(&ctx.graph);
    assert!(!brief.is_empty(), "brief must not be empty");
    assert!(brief.contains("auth"), "brief must mention entity");

    // Context format
    let context = specforge_emitter::emit_context(&ctx.graph);
    assert!(!context.is_empty(), "context must not be empty");

    // DOT format
    let dot = specforge_emitter::emit_dot(&ctx.graph);
    assert!(dot.contains("digraph"), "DOT must contain digraph");

    // Stats
    let stats = specforge_emitter::compute_stats(&ctx.graph);
    assert!(stats.total_entities >= 2, "should have at least 2 entities");
}
