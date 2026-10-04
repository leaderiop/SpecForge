//! @specforge/product — the product planning vocabulary and its queries.
//!
//! The vocabulary's describe payloads are the protocol envelopes in
//! `describe_*.json`, served through the SDK's `raw_category`; the handshake
//! is derived by the SDK from the extension metadata and contribution flags.
//! The CLI commands (`specforge product <command>`, auto-promoted to the MCP
//! tools `specforge.product.<id>`) are declared with their handlers in
//! [`commands`], over the queries in [`queries`]: the SDK derives their
//! `surfaces` payload and routes their `cmd__product_*` exports.

mod commands;
mod queries;
#[cfg(test)]
mod tests;

use specforge_extension_sdk::prelude::*;

static DESCRIBE_ENTITIES: &[u8] = include_bytes!("describe_entities.json");
static DESCRIBE_EDGES: &[u8] = include_bytes!("describe_edges.json");
static DESCRIBE_FIELDS: &[u8] = include_bytes!("describe_fields.json");
static DESCRIBE_SHARED_FIELDS: &[u8] = include_bytes!("describe_shared_fields.json");
static DESCRIBE_ENHANCEMENTS: &[u8] = include_bytes!("describe_enhancements.json");
static DESCRIBE_VALIDATION_RULES: &[u8] = include_bytes!("describe_validation_rules.json");
static DESCRIBE_PASSES: &[u8] = include_bytes!("describe_passes.json");
static DESCRIBE_FEATURE_FLAGS: &[u8] = include_bytes!("describe_feature_flags.json");

#[specforge_extension_sdk::extension(name = "@specforge/product", version = "1.0.0")]
struct Product;

impl Contributions for Product {
    fn contribute(c: &mut ContributionsBuilder) {
        // `specforge init` writes this as the starter spec of a project that
        // enables product.
        // Diagrams (`model`, `outline`) draw the extension in this colour.
        c.theme_color("#2ecc71");
        c.starter_template(include_str!("starter.spec"));

        for (category, bytes) in [
            ("entities", DESCRIBE_ENTITIES),
            ("edges", DESCRIBE_EDGES),
            ("fields", DESCRIBE_FIELDS),
            ("shared_fields", DESCRIBE_SHARED_FIELDS),
            ("enhancements", DESCRIBE_ENHANCEMENTS),
            ("validation_rules", DESCRIBE_VALIDATION_RULES),
            ("passes", DESCRIBE_PASSES),
            ("feature_flags", DESCRIBE_FEATURE_FLAGS),
        ] {
            let envelope: serde_json::Value = serde_json::from_slice(bytes)
                .unwrap_or_else(|e| panic!("product describe '{category}' is not valid JSON: {e}"));
            c.raw_category(category, envelope["items"].clone());
        }
        commands::declare(c);
    }
}

specforge_extension_sdk::component_guest!(build = specforge_extension_build);
