#[allow(dead_code)]
mod e2e_fixtures;

mod analyze;
mod cli;
mod collect;
mod config_schema;
mod contracts;
mod coverage_gate;
mod determinism;
mod docs_truth;
mod e2e_cross_extension;
mod e2e_edges;
mod e2e_mcp;
mod e2e_multi_entity;
mod e2e_query;
mod e2e_schema;
mod e2e_trace;
mod e2e_verify;
mod explain;
mod export;
mod export_version;
mod extension_authoring;
mod extensions;
mod field_types;
#[allow(deprecated)]
mod format;
mod format_corpus;
mod init;
#[allow(deprecated)]
mod migrate;
mod pipeline;
mod product_commands;
mod query;
mod registry;
mod schema_cache;
mod stats;
mod trace;
mod watch;
mod watch_reload;
