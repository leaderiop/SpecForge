use extism_pdk::*;
use serde::Deserialize;

#[host_fn]
extern "ExtismHost" {
    fn host_emit_diagnostic(input: Vec<u8>) -> Vec<u8>;
    fn host_read_file(input: Vec<u8>) -> Vec<u8>;
    fn host_query_graph(input: Vec<u8>) -> Vec<u8>;
}

static HANDSHAKE_JSON: &[u8] = include_bytes!("handshake.json");
static DESCRIBE_ENTITIES: &[u8] = include_bytes!("describe_entities.json");
static DESCRIBE_EDGES: &[u8] = include_bytes!("describe_edges.json");
static DESCRIBE_FIELDS: &[u8] = include_bytes!("describe_fields.json");
static DESCRIBE_SHARED_FIELDS: &[u8] = include_bytes!("describe_shared_fields.json");
static DESCRIBE_ENHANCEMENTS: &[u8] = include_bytes!("describe_enhancements.json");
static DESCRIBE_VALIDATION_RULES: &[u8] = include_bytes!("describe_validation_rules.json");
static DESCRIBE_SURFACES: &[u8] = include_bytes!("describe_surfaces.json");
static DESCRIBE_PASSES: &[u8] = include_bytes!("describe_passes.json");
static DESCRIBE_FEATURE_FLAGS: &[u8] = include_bytes!("describe_feature_flags.json");

#[derive(Deserialize)]
struct DescribeRequest {
    category: String,
}

#[plugin_fn]
pub fn __handshake(_input: Vec<u8>) -> FnResult<Vec<u8>> {
    Ok(HANDSHAKE_JSON.to_vec())
}

#[plugin_fn]
pub fn __describe(input: Vec<u8>) -> FnResult<Vec<u8>> {
    let request: DescribeRequest = serde_json::from_slice(&input)?;
    let response = match request.category.as_str() {
        "entities" => DESCRIBE_ENTITIES,
        "edges" => DESCRIBE_EDGES,
        "fields" => DESCRIBE_FIELDS,
        "shared_fields" => DESCRIBE_SHARED_FIELDS,
        "enhancements" => DESCRIBE_ENHANCEMENTS,
        "validation_rules" => DESCRIBE_VALIDATION_RULES,
        "surfaces" => DESCRIBE_SURFACES,
        "passes" => DESCRIBE_PASSES,
        "feature_flags" => DESCRIBE_FEATURE_FLAGS,
        _ => return Err(WithReturnCode::new(
            anyhow::anyhow!("unsupported category: {}", request.category),
            1,
        )),
    };
    Ok(response.to_vec())
}
