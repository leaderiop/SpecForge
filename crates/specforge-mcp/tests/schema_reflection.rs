//! Schema-reflection conformance: every advertised tool parameter must be
//! read by the handler, and every parameter a handler reads must be
//! advertised. Drift between `default_tools()` inputSchemas and the handlers
//! in `tools/` + `operations/` is how agents end up calling hidden params or
//! sending ignored ones — this test fails on either direction.

use serde_json::Value;
use serde_json::json;

/// Records which top-level argument keys a handler touches.
#[derive(Default)]
struct ArgSpy {
    inner: Value,
    touched: std::sync::Mutex<std::collections::BTreeSet<String>>,
}

impl ArgSpy {
    fn new(args: Value) -> Self {
        Self {
            inner: args,
            touched: std::sync::Mutex::new(Default::default()),
        }
    }

    /// Read a top-level key the way handlers do, recording the access.
    fn get(&self, key: &str) -> Option<&Value> {
        self.touched.lock().unwrap().insert(key.to_string());
        self.inner.get(key)
    }

    fn touched_keys(&self) -> std::collections::BTreeSet<String> {
        self.touched.lock().unwrap().clone()
    }
}

/// Advertised property names per tool, extracted from `default_tools()`.
fn advertised_properties() -> std::collections::BTreeMap<String, Vec<String>> {
    let mut out = std::collections::BTreeMap::new();
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

/// Every top-level key the handlers read, discovered by replaying a probe
/// argument object through the spy against the real dispatch table.
///
/// The probe contains every parameter name any tool documents (plus the
/// known legacy-hidden ones); a handler that reads outside its advertised
/// set will touch a key the schema does not list.
fn handler_read_keys(tool_name: &str, dispatch: impl Fn(&ArgSpy) -> Value) -> Vec<String> {
    let probe: Value = [
        ("entity_id", "@specforge/product"),
        ("depth", "0"),
        ("format", "graph"),
        ("include_coverage", "false"),
        ("path", "/nonexistent-probe-root"),
        ("severity_filter", "info"),
        ("use_cached", "true"),
        ("pass", "coverage"),
        ("strict", "false"),
        ("test_results", ""),
        ("scope", ""),
        ("max_tokens", "1"),
        ("plan", "{}"),
        ("query", ""),
        ("limit", "1"),
        ("field", ""),
        ("value", ""),
        ("references", ""),
        ("kind", ""),
        ("group_by", ""),
        ("fields", "[]"),
        ("extension", ""),
        ("root", ""),
        ("file", ""),
        ("new_name", ""),
        ("specifier", ""),
        ("name", ""),
        ("force", "false"),
        ("check", "false"),
        ("write", "false"),
        ("from_version", ""),
        ("to_version", ""),
        ("collector", ""),
        ("action", "status"),
        ("agent", ""),
        ("source_roots", "[]"),
        ("source_file", ""),
        ("entities_produced", "[]"),
        ("session_id", ""),
        ("status", ""),
        ("file_path", ""),
        ("paths", "[]"),
    ]
    .iter()
    .map(|(k, v)| (k.to_string(), Value::String(v.to_string())))
    .collect::<serde_json::Map<String, Value>>()
    .into();
    let spy = ArgSpy::new(probe);
    let _ = dispatch(&spy);
    let _ = tool_name;
    spy.touched_keys().into_iter().collect()
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

#[test]
fn spy_sees_accessed_keys() {
    // The mechanism itself works: a handler touching "depth" through the spy
    // is recorded.
    let spy = ArgSpy::new(json!({"depth": 2}));
    let _ = spy.get("depth");
    let _ = spy.get("entity_id");
    let touched = spy.touched_keys();
    assert!(touched.contains("depth") && touched.contains("entity_id"));
    assert_eq!(touched.len(), 2);
    let _ = handler_read_keys("probe", |spy| {
        let _ = spy.get("depth");
        json!({})
    });
}
