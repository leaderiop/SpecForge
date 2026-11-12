use super::*;

#[test]
fn every_name_is_one_row_and_no_group_is_a_tool_name() {
    for (i, tool) in ENGINE_TOOLS.iter().enumerate() {
        assert!(
            ENGINE_TOOLS[i + 1..].iter().all(|t| t.name != tool.name),
            "{} is two rows",
            tool.name
        );
        assert_eq!(engine_tool(tool.name), Some(tool));
    }
    for group in EngineGroup::ALL {
        assert_eq!(engine_tool(group.as_str()), None, "{group:?}");
        assert_eq!(EngineGroup::parse(group.as_str()), Some(group));
    }
}

#[test]
fn the_groups_hold_16_7_3_2_11_rows_and_graph_leaves_out_setup() {
    let count = |g: EngineGroup| ENGINE_TOOLS.iter().filter(|t| t.group == g).count();
    let counts: Vec<usize> = EngineGroup::ALL.into_iter().map(count).collect();
    assert_eq!(counts, [16, 7, 3, 2, 11]);
    assert_eq!(ENGINE_TOOLS.len(), 39);
    assert_eq!(ENGINE_TOOLS.iter().filter(|t| t.group.graph()).count(), 28);
    assert!(!EngineGroup::Setup.graph());
}

#[test]
fn a_class_parses_to_a_tool_a_group_or_a_command() {
    assert_eq!(
        parse_class("specforge:trace"),
        Some(EngineClass::Tool(engine_tool("trace").unwrap()))
    );
    assert_eq!(
        parse_class("specforge:reads"),
        Some(EngineClass::Group(EngineGroup::Reads))
    );
    assert_eq!(
        parse_class("specforge:gsd.record_verdict"),
        Some(EngineClass::Command {
            short: "gsd",
            id: "record_verdict"
        })
    );
    for none in [
        "specforge:",
        "specforge:nope",
        "specforge:.x",
        "specforge:x.",
        "read",
        "mcp:x/y",
    ] {
        assert_eq!(parse_class(none), None, "{none}");
    }
}

#[test]
fn by_cli_picks_the_twin_with_the_most_words() {
    let name = |words: &[&str]| by_cli(words).map(|t| t.name);
    assert_eq!(
        name(&["query", "X", "--inputs", "gsd/E501"]),
        Some("verdict_inputs")
    );
    assert_eq!(
        name(&["query", "X", "--inputs=gsd/E501"]),
        Some("verdict_inputs")
    );
    assert_eq!(name(&["query", "X"]), Some("query"));
    assert_eq!(
        name(&["anchors", "--file", "src/a.rs"]),
        Some("find_spec_for_source")
    );
    assert_eq!(
        name(&["anchors", "behavior.x"]),
        Some("find_implementation")
    );
    assert_eq!(name(&["settings", "set", "k", "v"]), Some("settings_set"));
    assert_eq!(name(&["settings"]), Some("settings"));
    assert_eq!(name(&["link", "a", "b"]), Some("edit"));
    assert_eq!(name(&["review"]), None);
    assert_eq!(name(&[]), None);
}

const TRACE: Option<&EngineTool> = by_mcp_name("specforge.trace");

#[test]
fn by_mcp_name_finds_each_row_in_a_const() {
    assert_eq!(TRACE.map(|t| t.name), Some("trace"));
    for tool in ENGINE_TOOLS {
        assert_eq!(by_mcp_name(&tool.mcp_name()), Some(tool));
    }
    for none in ["specforge.", "specforge.nope", "trace", "x.trace", ""] {
        assert_eq!(by_mcp_name(none), None, "{none}");
    }
}

#[test]
fn every_use_when_is_one_sentence() {
    for tool in ENGINE_TOOLS {
        let u = tool.use_when;
        assert!(
            u.starts_with(|c: char| c.is_ascii_uppercase()),
            "{}: {u}",
            tool.name
        );
        assert!(
            u.ends_with('.') && !u[..u.len() - 1].contains(". "),
            "{}: {u}",
            tool.name
        );
        assert!(
            !u.contains('\n') && u.chars().count() <= 120,
            "{}: {u}",
            tool.name
        );
        // The remedy-wording scan leaves this file out for its CLI twins; the prose names no flag either.
        assert!(!u.contains("--"), "{}: {u}", tool.name);
    }
}
