use specforge_registry::{
    register_surface_contributions, CommandArg, CommandArgType, CommandContribution,
    McpResourceContribution, McpToolContribution, SurfaceContributions, SurfaceType, ManifestV2,
};

fn make_surfaces(
    commands: Vec<CommandContribution>,
    tools: Vec<McpToolContribution>,
    resources: Vec<McpResourceContribution>,
) -> SurfaceContributions {
    SurfaceContributions {
        commands,
        mcp_tools: tools,
        mcp_resources: resources,
    }
}

fn make_command(id: &str, export: &str) -> CommandContribution {
    CommandContribution {
        id: id.to_string(),
        title: id.to_string(),
        description: format!("{} command", id),
        category: None,
        export: export.to_string(),
        args: vec![],
        sandbox: None,
    }
}

fn make_tool(name: &str, export: &str) -> McpToolContribution {
    McpToolContribution {
        name: name.to_string(),
        description: format!("{} tool", name),
        category: None,
        export: export.to_string(),
        input_schema: serde_json::json!({"type": "object"}),
        output_schema: None,
        sandbox: None,
    }
}

fn make_resource(name: &str, export: &str) -> McpResourceContribution {
    McpResourceContribution {
        uri_template: format!("spec://{}", name),
        name: name.to_string(),
        description: None,
        export: export.to_string(),
        mime_type: "application/json".to_string(),
        sandbox: None,
    }
}

// B:surface_contributions_types — verify unit "SurfaceContributions round-trip JSON serialization"
#[test]
fn test_surface_contributions_round_trip_json() {
    let surfaces = make_surfaces(
        vec![make_command("analyze", "cmd__analyze")],
        vec![make_tool("search", "mcp__search")],
        vec![make_resource("graph", "mcp__graph")],
    );

    let json = serde_json::to_string(&surfaces).unwrap();
    let parsed: SurfaceContributions = serde_json::from_str(&json).unwrap();
    assert_eq!(surfaces, parsed);
}

// B:surface_contributions_types — verify unit "ManifestV2 with surfaces field parses"
#[test]
fn test_manifest_with_surfaces_parses() {
    let manifest: ManifestV2 = serde_json::from_str(r#"{
        "name": "@ext/test",
        "version": "1.0.0",
        "manifestVersion": 2,
        "wasmPath": "test.wasm",
        "surfaces": {
            "commands": [{
                "id": "analyze",
                "title": "Analyze",
                "description": "Run analysis",
                "export": "cmd__analyze"
            }],
            "mcpTools": [{
                "name": "search",
                "description": "Search entities",
                "export": "mcp__search",
                "inputSchema": {"type": "object"}
            }]
        }
    }"#).unwrap();

    let surfaces = manifest.surfaces.unwrap();
    assert_eq!(surfaces.commands.len(), 1);
    assert_eq!(surfaces.commands[0].id, "analyze");
    assert_eq!(surfaces.mcp_tools.len(), 1);
    assert_eq!(surfaces.mcp_tools[0].name, "search");
}

// B:surface_contributions_types — verify unit "ManifestV2 without surfaces defaults to None"
#[test]
fn test_manifest_without_surfaces_defaults_none() {
    let manifest: ManifestV2 = serde_json::from_str(r#"{
        "name": "@ext/test",
        "version": "1.0.0",
        "manifestVersion": 2,
        "wasmPath": "test.wasm"
    }"#).unwrap();
    assert!(manifest.surfaces.is_none());
}

// B:surface_contributions_types — verify unit "all 5 CommandArgType variants deserialize"
#[test]
fn test_all_command_arg_type_variants_deserialize() {
    let json = r#"[
        {"name": "path", "argType": "string"},
        {"name": "file", "argType": "path"},
        {"name": "verbose", "argType": "bool"},
        {"name": "format", "argType": {"enum": {"values": ["json", "text"]}}},
        {"name": "count", "argType": "integer"}
    ]"#;
    let args: Vec<CommandArg> = serde_json::from_str(json).unwrap();
    assert_eq!(args.len(), 5);
    assert_eq!(args[0].arg_type, CommandArgType::StringArg);
    assert_eq!(args[1].arg_type, CommandArgType::PathArg);
    assert_eq!(args[2].arg_type, CommandArgType::BoolArg);
    assert!(matches!(args[3].arg_type, CommandArgType::EnumArg { .. }));
    assert_eq!(args[4].arg_type, CommandArgType::IntegerArg);
}

// B:register_surface_contributions — verify unit "commands, tools, resources collected"
#[test]
fn test_register_surface_contributions_collects_all() {
    let surfaces = make_surfaces(
        vec![make_command("analyze", "cmd__analyze")],
        vec![make_tool("search", "mcp__search")],
        vec![make_resource("graph", "mcp__graph")],
    );

    let manifests = vec![("@ext/test".to_string(), Some(surfaces))];
    let (entries, diags) = register_surface_contributions(&manifests);
    assert!(diags.is_empty());
    assert_eq!(entries.len(), 3);

    assert!(entries.iter().any(|e| e.surface_type == SurfaceType::Command && e.contribution_name == "analyze"));
    assert!(entries.iter().any(|e| e.surface_type == SurfaceType::McpTool && e.contribution_name == "search"));
    assert!(entries.iter().any(|e| e.surface_type == SurfaceType::McpResource && e.contribution_name == "graph"));
}

// B:register_surface_contributions — verify unit "duplicate command ID → E039"
#[test]
fn test_register_duplicate_command_id_e039() {
    let s1 = make_surfaces(vec![make_command("analyze", "cmd__analyze")], vec![], vec![]);
    let s2 = make_surfaces(vec![make_command("analyze", "cmd__analyze_v2")], vec![], vec![]);

    let manifests = vec![
        ("@ext/a".to_string(), Some(s1)),
        ("@ext/b".to_string(), Some(s2)),
    ];
    let (_, diags) = register_surface_contributions(&manifests);
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].code, "E039");
    assert!(diags[0].message.contains("analyze"));
}

// B:register_surface_contributions — verify unit "duplicate MCP tool name → E039"
#[test]
fn test_register_duplicate_mcp_tool_e039() {
    let s1 = make_surfaces(vec![], vec![make_tool("search", "mcp__search")], vec![]);
    let s2 = make_surfaces(vec![], vec![make_tool("search", "mcp__search_v2")], vec![]);

    let manifests = vec![
        ("@ext/a".to_string(), Some(s1)),
        ("@ext/b".to_string(), Some(s2)),
    ];
    let (_, diags) = register_surface_contributions(&manifests);
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].code, "E039");
    assert!(diags[0].message.contains("search"));
}

// B:register_surface_contributions — verify unit "no duplicates → clean registration"
#[test]
fn test_register_no_duplicates_clean() {
    let s1 = make_surfaces(
        vec![make_command("analyze", "cmd__analyze")],
        vec![make_tool("search", "mcp__search")],
        vec![],
    );
    let s2 = make_surfaces(
        vec![make_command("report", "cmd__report")],
        vec![],
        vec![make_resource("graph", "mcp__graph")],
    );

    let manifests = vec![
        ("@ext/a".to_string(), Some(s1)),
        ("@ext/b".to_string(), Some(s2)),
    ];
    let (entries, diags) = register_surface_contributions(&manifests);
    assert!(diags.is_empty());
    assert_eq!(entries.len(), 4);
}

// B:register_surface_contributions — verify contract
#[test]
fn test_register_surface_contributions_contract() {
    // requires: manifests with surface contributions
    // ensures: all contributions registered with correct types
    let surfaces = make_surfaces(
        vec![make_command("run", "cmd__run")],
        vec![make_tool("query", "mcp__query")],
        vec![make_resource("spec", "mcp__spec")],
    );
    let manifests = vec![("@ext/a".to_string(), Some(surfaces))];
    let (entries, diags) = register_surface_contributions(&manifests);
    assert!(diags.is_empty());
    assert_eq!(entries.len(), 3);
    assert!(entries.iter().all(|e| e.enabled));
    assert!(entries.iter().all(|e| e.extension_name == "@ext/a"));

    // ensures: None surfaces are skipped
    let manifests2 = vec![("@ext/b".to_string(), None)];
    let (entries2, diags2) = register_surface_contributions(&manifests2);
    assert!(entries2.is_empty());
    assert!(diags2.is_empty());

    // ensures: duplicates produce E039
    let s1 = make_surfaces(vec![make_command("x", "cmd__x")], vec![], vec![]);
    let s2 = make_surfaces(vec![make_command("x", "cmd__x2")], vec![], vec![]);
    let manifests3 = vec![
        ("@ext/a".to_string(), Some(s1)),
        ("@ext/b".to_string(), Some(s2)),
    ];
    let (_, diags3) = register_surface_contributions(&manifests3);
    assert!(diags3.iter().all(|d| d.code == "E039"));
}
