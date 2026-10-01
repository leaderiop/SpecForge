//! Schema-reflection conformance: every advertised tool parameter must be
//! read by the handler, and every parameter a handler reads must be
//! advertised. Drift between `default_tools()` inputSchemas and the handlers
//! in `tools/` + `operations/` is how agents end up calling hidden params or
//! sending ignored ones; this test fails on either direction.
//!
//! A `serde_json::Value` has no hook that records which keys a handler
//! reads, so the reads come from the handler's source: every
//! `args.get("key")` (or `arguments.get`, or `args["key"]`) in
//! `tools/<tool>.rs`, or in the body of `fn <tool>_op` in
//! `operations/mod.rs`, plus `path` for a call to `project_root_of`. Typed
//! argument structs (plan 04 T4) replace this with a serde field tracer.

use specforge_test::prelude::*;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

fn crate_file(rel: &str) -> Option<String> {
    std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("src").join(rel)).ok()
}

/// The source of the handler for `tool`: its own module under `tools/`, or
/// its `fn <short>_op` in `operations/mod.rs` up to the next function.
fn handler_source(tool: &str) -> Option<String> {
    let short = tool.strip_prefix("specforge.")?;
    if let Some(source) = crate_file(&format!("tools/{short}.rs")) {
        return Some(source);
    }
    let ops = crate_file("operations/mod.rs")?;
    let start = ["\nfn ", "\npub(crate) fn "]
        .iter()
        .find_map(|item| ops.find(&format!("{item}{short}_op(")))?
        + 1;
    let body = &ops[start..];
    let end = ["\nfn ", "\npub fn ", "\npub(crate) fn "]
        .iter()
        .filter_map(|item| body[1..].find(item))
        .min()
        .map_or(body.len(), |i| i + 1);
    Some(body[..end].to_string())
}

/// The argument keys a handler's source reads. A read through a computed
/// key cannot be checked, so it is an error.
fn handler_reads(source: &str) -> Result<BTreeSet<String>, String> {
    // Join method chains split across lines (`args\n    .get(`); any other
    // whitespace run becomes one space, so words stay apart.
    let mut compact = String::new();
    let mut chars = source.chars().peekable();
    while let Some(c) = chars.next() {
        if !c.is_whitespace() {
            compact.push(c);
            continue;
        }
        while chars.next_if(|c| c.is_whitespace()).is_some() {}
        if chars.peek() != Some(&'.') && !compact.ends_with('.') {
            compact.push(' ');
        }
    }
    let mut reads = BTreeSet::new();
    if compact.contains("project_root_of(state, &args)") {
        reads.insert("path".to_string());
    }
    for receiver in ["args", "arguments"] {
        for access in [".get(", "["] {
            let pattern = format!("{receiver}{access}");
            for (at, _) in compact.match_indices(&pattern) {
                let bounded = compact[..at]
                    .chars()
                    .next_back()
                    .is_none_or(|c| !(c.is_alphanumeric() || c == '_' || c == '.'));
                if !bounded {
                    continue;
                }
                let rest = &compact[at + pattern.len()..];
                let Some(literal) = rest.strip_prefix('"') else {
                    let shown: String = rest.chars().take(30).collect();
                    return Err(format!("reads a computed key: {pattern}{shown}"));
                };
                reads.insert(literal.chars().take_while(|c| *c != '"').collect());
            }
        }
    }
    Ok(reads)
}

#[specforge_test(
    behavior = "list_mcp_tools",
    verify = "each core tool's input schema advertises exactly the arguments its handler reads"
)]
fn each_core_tool_schema_advertises_exactly_what_its_handler_reads() {
    let mut drift = Vec::new();
    for tool in specforge_mcp::registry::default_tools() {
        let source = handler_source(&tool.name)
            .unwrap_or_else(|| panic!("{}: no handler source found", tool.name));
        let read = handler_reads(&source).unwrap_or_else(|e| panic!("{}: {e}", tool.name));
        let advertised: BTreeSet<String> = tool
            .input_schema
            .get("properties")
            .and_then(|p| p.as_object())
            .map(|o| o.keys().cloned().collect())
            .unwrap_or_default();
        let hidden: Vec<&String> = read.difference(&advertised).collect();
        let ignored: Vec<&String> = advertised.difference(&read).collect();
        if !hidden.is_empty() || !ignored.is_empty() {
            drift.push(format!(
                "{}: read but not advertised {hidden:?}; advertised but never read {ignored:?}",
                tool.name
            ));
        }
    }
    assert!(drift.is_empty(), "schema drift: {drift:#?}");
}

#[test]
fn handler_reads_sees_literal_reads_across_lines() {
    let source = "let a = args\n    .get(\"depth\");\nlet b = arguments.get(\"kind\");\n\
                  let c = node.fields.get(\"verify\");\nlet d = project_root_of(state, &args);";
    let reads = handler_reads(source).unwrap();
    let expected: BTreeSet<String> = ["depth", "kind", "path"].map(String::from).into();
    assert_eq!(reads, expected);
    assert!(handler_reads("args.get(key)").is_err());
}

/// Advertised property names per tool, extracted from `default_tools()`.
fn advertised_properties() -> BTreeMap<String, Vec<String>> {
    let mut out = BTreeMap::new();
    for tool in specforge_mcp::registry::default_tools() {
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
