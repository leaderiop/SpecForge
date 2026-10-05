//! The registry build of the builtins, pinned: a canonical digest of what
//! `Environment::load` derives from every builtin together, from each
//! builtin alone and from the SDK greet fixture alone. The digest reads the
//! registries as they are typed today; a refactor of those types rewrites
//! the digest, never the pinned snapshots.

use std::collections::BTreeMap;
use std::path::Path;

use serde_json::{Value, json};
use specforge_common::Diagnostic;
use specforge_component::ComponentRuntime;
use specforge_component::builtins::BUILTIN_EXTENSIONS;
use specforge_project::Environment;
use specforge_protocol_types::{SurfaceDescriptor, SurfaceSandboxOverride};
use tempfile::TempDir;

fn runtime() -> ComponentRuntime {
    let runtime = ComponentRuntime::new();
    specforge_component::builtins::load_builtins(&runtime).expect("builtins load");
    let greet =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/greet-extension/greet.wasm");
    let greet = std::fs::read(greet).expect("vendored greet component blob");
    runtime
        .load_module_bytes("@sdk/greet", &greet)
        .expect("greet loads");
    runtime
}

fn load(runtime: &ComponentRuntime, extensions: &[&str]) -> Environment {
    let dir = TempDir::new().unwrap();
    std::fs::write(
        dir.path().join("specforge.json"),
        json!({ "name": "p", "version": "0.1.0", "extensions": extensions }).to_string(),
    )
    .unwrap();
    Environment::load(dir.path(), Some(runtime))
}

fn diagnostics<'a>(diagnostics: impl IntoIterator<Item = &'a Diagnostic>) -> Value {
    diagnostics
        .into_iter()
        .map(|d| json!({ "code": d.code, "message": d.message }))
        .collect()
}

fn sorted(mut values: Vec<Value>) -> Value {
    values.sort_by_key(|v| v.to_string());
    Value::Array(values)
}

fn surfaces(s: &SurfaceDescriptor) -> Value {
    let sandbox = |s: Option<&SurfaceSandboxOverride>| {
        s.map(|s| json!({ "fs_read": s.fs_read, "fs_write": s.fs_write, "network": s.network }))
    };
    json!({
        "commands": s.commands.iter().map(|c| json!({
            "id": c.id,
            "title": c.title,
            "description": c.description,
            "category": c.category,
            "export": c.export,
            "args": c.args.iter().map(|a| json!({
                "name": a.name,
                "arg_type": serde_json::to_value(&a.arg_type).unwrap(),
                "required": a.required,
                "default_value": a.default_value,
                "description": a.description,
            })).collect::<Vec<_>>(),
            "sandbox": sandbox(c.sandbox.as_ref()),
        })).collect::<Vec<_>>(),
        "mcp_tools": s.mcp_tools.iter().map(|t| json!({
            "name": t.name,
            "description": t.description,
            "category": t.category,
            "export": t.export,
            "input_schema": t.input_schema,
            "output_schema": t.output_schema,
            "sandbox": sandbox(t.sandbox.as_ref()),
        })).collect::<Vec<_>>(),
        "mcp_resources": s.mcp_resources.iter().map(|r| json!({
            "uri_template": r.uri_template,
            "name": r.name,
            "description": r.description,
            "export": r.export,
            "mime_type": r.mime_type,
            "sandbox": sandbox(r.sandbox.as_ref()),
        })).collect::<Vec<_>>(),
    })
}

fn digest(env: &Environment) -> Value {
    let r = &env.registries;
    let kinds: BTreeMap<&str, Value> = r
        .kinds
        .iter()
        .map(|(name, k)| {
            (
                name.as_str(),
                json!({
                    "kind_name": k.kind_name,
                    "description": k.description,
                    "source_extension": k.source_extension,
                    "testable": k.testable,
                    "singleton": k.singleton,
                    "supports_verify": k.supports_verify,
                    "allowed_verify_kinds": k.allowed_verify_kinds,
                    "has_body_parser": k.has_body_parser,
                    "semantic_token": k.semantic_token,
                    "lsp_icon": k.lsp_icon,
                    "dot_shape": k.dot_shape,
                    "dot_color": k.dot_color,
                    "dot_fillcolor": k.dot_fillcolor,
                    "open_fields": k.open_fields,
                    "contract_target": k.contract_target,
                    "declares_types": k.declares_types,
                    "lifecycle_field": k.lifecycle_field,
                }),
            )
        })
        .collect();
    let fields: BTreeMap<String, Value> = r
        .fields
        .iter()
        .map(|(kind, field, f)| {
            (
                format!("{kind}.{field}"),
                json!({
                    "kind_name": f.kind_name,
                    "field_name": f.field_name,
                    "description": f.description,
                    "field_type": format!("{:?}", f.field_type),
                    "source_extension": f.source_extension,
                    "edge": f.edge,
                    "target_kind": f.target_kind,
                    "file_reference": f.file_reference,
                    "required": f.required,
                    "inverse_of": f.inverse_of,
                    "normative": f.normative,
                    "exempts_obligations": f.exempts_obligations,
                    "headline": f.headline,
                    "derived_from": f.derived_from,
                    "proof_role": f.proof_role.map(|p| format!("{p:?}")),
                }),
            )
        })
        .collect();
    let edges: BTreeMap<&str, Value> = r
        .edges
        .iter()
        .map(|(label, e)| {
            (
                label.as_str(),
                json!({
                    "label": e.label,
                    "description": e.description,
                    "source_kind": e.source_kind,
                    "target_kind": e.target_kind,
                    "source_extension": e.source_extension,
                    "edge_style": e.edge_style,
                    "edge_color": e.edge_color,
                    "edge_arrowhead": e.edge_arrowhead,
                }),
            )
        })
        .collect();
    let rules: Vec<Value> = r
        .rules
        .iter()
        .map(|(rule, owner)| {
            json!({
                "code": rule.code,
                "severity": format!("{:?}", rule.severity),
                "message_template": rule.message_template,
                "check": rule.check.as_str(),
                "target_kind": rule.target_kind,
                "edge_type": rule.edge_type,
                "edge_peer_kind": rule.edge_peer_kind,
                "field": rule.field,
                "constraint": rule.constraint.as_ref().map(|c| json!({
                    "kind": c.kind.as_ref().map(|k| k.as_str()),
                    "pattern": c.pattern,
                    "values": c.values,
                })),
                "wasm_function": rule.wasm_function,
                "owner": owner,
            })
        })
        .collect();
    let bidirectional_pairs: Vec<Value> = r
        .bidirectional_pairs
        .iter()
        .map(|(a, b)| {
            let (a, b) = if a <= b { (a, b) } else { (b, a) };
            json!([a, b])
        })
        .collect();
    let absent_reference_targets: BTreeMap<String, &String> = r
        .absent_reference_targets
        .iter()
        .map(|((kind, field), target)| (format!("{kind}.{field}"), target))
        .collect();
    // An extension's routing name: its declared short name, else its name's
    // last segment.
    let ext_short: BTreeMap<&str, String> = r
        .declarations()
        .iter()
        .map(|d| (d.name(), d.short().into_owned()))
        .collect();
    // The load's diagnostics, then the declarations' own, then the setup's
    // (providers, I002): what `Environment::diagnostics()` reports before
    // the registry build's.
    let load_diagnostics = env
        .load_diagnostics
        .iter()
        .chain(&r.declaration_diagnostics)
        .chain(&env.setup_diagnostics);
    json!({
        "kinds": kinds,
        "fields": fields,
        "edges": edges,
        "rules": rules,
        "body_parser_kinds": sorted(r.body_parser_kinds.iter().map(|k| json!(k)).collect()),
        "single_reference_fields": sorted(
            r.single_reference_fields.iter().map(|(k, f)| json!([k, f])).collect()
        ),
        "bidirectional_pairs": sorted(bidirectional_pairs),
        "absent_reference_targets": absent_reference_targets,
        "surfaces": r.surfaces.iter().map(|s| json!({
            "surface_type": format!("{:?}", s.surface_type),
            "name": s.contribution_name,
            "extension": s.extension_name,
            "export": s.export_name,
        })).collect::<Vec<_>>(),
        "manifest_surfaces": r.declarations().iter()
            .filter(|d| d.surfaces != SurfaceDescriptor::default())
            .map(|d| json!({ "extension": d.name(), "surfaces": surfaces(&d.surfaces) }))
            .collect::<Vec<_>>(),
        "extension_info": r.extension_info().collect::<Vec<_>>(),
        "check_passes": r.check_passes()
            .map(|p| json!({ "extension": p.extension, "name": p.pass.name }))
            .collect::<Vec<_>>(),
        "ext_short": ext_short,
        "load_diagnostics": diagnostics(load_diagnostics),
        "registry_diagnostics": diagnostics(&r.registry_diagnostics),
        "surface_diagnostics": diagnostics(&r.surface_diagnostics),
    })
}

#[specforge_test_macros::test(
    behavior = "build_registries_from_declarations",
    verify = "the registry build of the builtins matches its pinned snapshot"
)]
fn registry_build_of_the_builtins_matches_its_snapshot() {
    let runtime = runtime();
    let all: Vec<&str> = BUILTIN_EXTENSIONS.iter().map(|(name, _)| *name).collect();
    insta::assert_json_snapshot!("all_builtins", digest(&load(&runtime, &all)));
    for name in &all {
        let dir = name.rsplit('/').next().unwrap();
        insta::assert_json_snapshot!(format!("alone_{dir}"), digest(&load(&runtime, &[name])));
    }
    insta::assert_json_snapshot!("alone_greet", digest(&load(&runtime, &["@sdk/greet"])));
}
