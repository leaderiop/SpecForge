//! Schema-reflection conformance: every advertised tool parameter must be
//! read by the handler, and every parameter a handler reads must be
//! advertised. Drift between the input schemas and the handlers is how
//! agents end up calling hidden params or sending ignored ones; this test
//! fails on either direction.
//!
//! Each core tool reads its arguments into one `Args` struct that derives
//! `Arguments` (ADR 0033), which lists the arguments the handler can read
//! without calling it.

use specforge_test::prelude::*;
use std::collections::{BTreeMap, BTreeSet};

#[specforge_test(
    behavior = "list_mcp_tools",
    verify = "each core tool's input schema advertises exactly the arguments its handler reads"
)]
fn each_core_tool_schema_advertises_exactly_what_its_handler_reads() {
    let mut drift = Vec::new();
    for tool in specforge_mcp::tools::CORE_TOOLS {
        let read: BTreeSet<&str> = tool.reads().into_iter().collect();
        let schema = tool.input_schema();
        let advertised: BTreeSet<&str> = schema
            .get("properties")
            .and_then(|p| p.as_object())
            .map(|o| o.keys().map(String::as_str).collect())
            .unwrap_or_default();
        let hidden: Vec<&&str> = read.difference(&advertised).collect();
        let ignored: Vec<&&str> = advertised.difference(&read).collect();
        if !hidden.is_empty() || !ignored.is_empty() {
            drift.push(format!(
                "{}: read but not advertised {hidden:?}; advertised but never read {ignored:?}",
                tool.name
            ));
        }
        // A required argument is one the handler cannot do without.
        for required in schema["required"].as_array().into_iter().flatten() {
            let required = required.as_str().unwrap_or_default();
            if !read.contains(required) {
                drift.push(format!("{}: requires {required}, never read", tool.name));
            }
        }
    }
    assert!(drift.is_empty(), "schema drift: {drift:#?}");
}

#[specforge_test(
    behavior = "serve_mcp_prompt",
    verify = "each core prompt lists exactly the arguments its handler reads"
)]
fn each_core_prompt_lists_exactly_the_arguments_its_handler_reads() {
    let mut served = crate::support::TestProject::new().serve_components();
    let mut drift = Vec::new();
    for prompt in specforge_mcp::prompts::CORE_PROMPTS {
        let declared = (prompt.arguments)();
        let listed: Vec<String> = prompt
            .descriptor()
            .arguments
            .unwrap_or_default()
            .into_iter()
            .map(|a| a.name)
            .collect();
        let read: Vec<&str> = declared.iter().map(|argument| argument.name).collect();
        if listed != read {
            drift.push(format!("{}: lists {listed:?}, reads {read:?}", prompt.name));
        }
        if declared
            .iter()
            .any(|argument| argument.description.is_empty())
        {
            drift.push(format!("{}: an argument has no description", prompt.name));
        }
        // A listed required argument is one the prompt cannot render
        // without: a request with only the others is refused naming it.
        let required: Vec<&str> = declared
            .iter()
            .filter(|argument| argument.required)
            .map(|argument| argument.name)
            .collect();
        for missing in &required {
            let others: serde_json::Map<String, serde_json::Value> = required
                .iter()
                .filter(|name| *name != missing)
                .map(|name| (name.to_string(), serde_json::json!("x")))
                .collect();
            let reply = crate::support::get_prompt(
                &mut served,
                prompt.name,
                serde_json::Value::Object(others),
            );
            let error = &reply["error"];
            if error["code"] != -32602
                || error["message"] != format!("Missing required parameter: {missing}")
                || error["data"]["argument"] != *missing
            {
                drift.push(format!("{}: without {missing}: {reply}", prompt.name));
            }
        }
    }
    assert!(drift.is_empty(), "prompt argument drift: {drift:#?}");
    assert!(
        specforge_mcp::prompts::CORE_PROMPTS
            .iter()
            .all(|p| !(p.arguments)().is_empty()),
        "every prompt declares its arguments"
    );
}

#[test]
fn the_field_tracer_sees_each_args_struct() {
    let names = |tool: &str| -> BTreeSet<&'static str> {
        specforge_mcp::tools::core_tool(tool)
            .unwrap()
            .arguments()
            .iter()
            .map(|argument| argument.name)
            .collect()
    };
    let expected: BTreeSet<&str> =
        ["entity_id", "depth", "kinds", "format", "include_coverage"].into();
    assert_eq!(names("specforge.query"), expected);
    assert!(
        names("specforge.stats").is_empty(),
        "stats takes no arguments"
    );
}

/// Advertised property names per tool, extracted from the core tool table.
fn advertised_properties() -> BTreeMap<String, Vec<String>> {
    let mut out = BTreeMap::new();
    for tool in crate::support::core_tools() {
        let props = tool
            .input_schema
            .get("properties")
            .and_then(|p| p.as_object())
            .map(|o| o.keys().cloned().collect())
            .unwrap_or_default();
        out.insert(tool.name, props);
    }
    out
}

#[test]
fn trace_advertises_plan() {
    let tools = advertised_properties();
    let trace = tools.get("specforge.trace").expect("trace tool advertised");
    assert!(
        trace.iter().any(|p| p == "plan"),
        "trace reads plan for gap analysis; it must be advertised"
    );
}

#[test]
fn format_advertises_every_argument_it_reads() {
    let tools = advertised_properties();
    let format = tools
        .get("specforge.format")
        .expect("format tool advertised");
    for arg in ["path", "paths", "check", "diff", "write"] {
        assert!(format.iter().any(|p| p == arg), "format reads {arg}");
    }
}

#[test]
fn operation_tools_advertise_path() {
    let tools = advertised_properties();
    for name in [
        "specforge.rename",
        "specforge.add_extension",
        "specforge.remove_extension",
        "specforge.migrate",
        "specforge.collect",
        "specforge.validate",
        "specforge.format",
    ] {
        let props = tools
            .get(name)
            .unwrap_or_else(|| panic!("{name} advertised"));
        assert!(
            props.iter().any(|p| p == "path"),
            "{name} resolves its root via project_root_of (reads 'path') but does not advertise it"
        );
    }
}

#[test]
fn query_advertises_format_and_coverage() {
    let tools = advertised_properties();
    let query = tools.get("specforge.query").expect("query advertised");
    for expected in ["format", "include_coverage"] {
        assert!(
            query.iter().any(|p| p == expected),
            "specforge.query reads '{expected}' (query.rs) but does not advertise it"
        );
    }
}

#[test]
fn validate_advertises_severity_filter_and_cache() {
    let tools = advertised_properties();
    let validate = tools
        .get("specforge.validate")
        .expect("validate advertised");
    for expected in ["severity_filter", "use_cached"] {
        assert!(
            validate.iter().any(|p| p == expected),
            "specforge.validate reads '{expected}' (validate.rs) but does not advertise it"
        );
    }
}

#[test]
fn search_advertises_field_value_references() {
    let tools = advertised_properties();
    let search = tools.get("specforge.search").expect("search advertised");
    for expected in ["field", "value", "references"] {
        assert!(
            search.iter().any(|p| p == expected),
            "specforge.search reads '{expected}' (search.rs) but does not advertise it"
        );
    }
}
