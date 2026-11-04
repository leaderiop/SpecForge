use super::*;

#[test]
fn operation_wire_names_are_snake_case() {
    let names: Vec<String> = Operation::ALL
        .iter()
        .map(|op| {
            serde_json::to_value(op)
                .unwrap()
                .as_str()
                .unwrap()
                .to_string()
        })
        .collect();
    assert_eq!(
        names,
        [
            "handshake",
            "describe",
            "command",
            "mcp_tool",
            "mcp_resource",
            "pass",
            "collect",
            "validate",
            "scan",
            "migrate",
            "prompt",
            "render",
            "hook"
        ]
    );
    for op in Operation::ALL {
        let wire = serde_json::to_string(&op).unwrap();
        assert_eq!(serde_json::from_str::<Operation>(&wire).unwrap(), op);
    }
    assert!(serde_json::from_str::<Operation>("\"McpTool\"").is_err());
}
