//! Each enumerated argument a core tool takes is advertised from its option
//! table (ADR 0027): the input schema's `enum` is every name the table
//! accepts (listed names, then aliases), its `default` the table's, and the
//! description names each choice.

use serde_json::Value;
use specforge_ops::export::AGENT_FORMAT;
use specforge_ops::model::{
    DEPS, GROUP_BY, MODEL_FIELDS, MODEL_FORMAT, OUTLINE_FIELDS, OUTLINE_FORMAT,
};
use specforge_ops::options::OptionTable;

/// What a table says an input-schema property must hold.
struct Advertised {
    tool: &'static str,
    argument: &'static str,
    accepted: Vec<&'static str>,
    names: Vec<&'static str>,
    default: Option<&'static str>,
}

fn advertised<T: Copy + PartialEq>(
    tool: &'static str,
    argument: &'static str,
    table: &OptionTable<T>,
) -> Advertised {
    Advertised {
        tool,
        argument,
        accepted: table.accepted().collect(),
        names: table.names().collect(),
        default: table.default_name(),
    }
}

/// Every enumerated argument of a core tool, with the table it reads.
fn enumerated() -> Vec<Advertised> {
    vec![
        advertised("specforge.export", "format", &AGENT_FORMAT),
        advertised("specforge.model", "format", &MODEL_FORMAT),
        advertised("specforge.model", "group_by", &GROUP_BY),
        advertised("specforge.model", "fields", &MODEL_FIELDS),
        advertised("specforge.outline_extensions", "format", &OUTLINE_FORMAT),
        advertised("specforge.outline_extensions", "fields", &OUTLINE_FIELDS),
        advertised("specforge.outline_extensions", "deps", &DEPS),
    ]
}

/// The input schema property `argument` of core tool `tool`.
fn property(tool: &str, argument: &str) -> Value {
    let spec = specforge_mcp::tools::CORE_TOOLS
        .iter()
        .find(|spec| spec.name == tool)
        .unwrap_or_else(|| panic!("no core tool {tool}"));
    (spec.schema)()["properties"][argument].clone()
}

#[specforge_test_macros::test(
    behavior = "name_enumerated_options_once",
    verify = "each enumerated MCP argument advertises the table's names and default"
)]
fn each_enumerated_argument_advertises_its_table() {
    for expected in enumerated() {
        let at = format!("{}.{}", expected.tool, expected.argument);
        let property = property(expected.tool, expected.argument);
        assert_eq!(property["type"], "string", "{at}");
        let listed: Vec<&str> = property["enum"]
            .as_array()
            .unwrap_or_else(|| panic!("{at} lists no enum: {property}"))
            .iter()
            .map(|name| name.as_str().unwrap())
            .collect();
        assert_eq!(listed, expected.accepted, "{at}");
        assert_eq!(
            property.get("default").and_then(Value::as_str),
            expected.default,
            "{at}: one default, the table's, on every surface"
        );
        let description = property["description"].as_str().unwrap_or_default();
        for name in &expected.names {
            assert!(description.contains(name), "{at}: {description}");
        }
    }
}
