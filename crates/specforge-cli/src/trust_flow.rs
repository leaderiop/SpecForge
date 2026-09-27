//! Trust flow lives in `specforge-registry` (single security
//! implementation shared with the MCP server); re-exported for the CLI.
pub use specforge_registry::client::trust_flow::check_and_pin;
