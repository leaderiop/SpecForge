#[allow(dead_code)]
mod e2e_fixtures;

mod analyze;
mod build_cache;
mod check_surfaces;
mod child_guard;
mod cli;
mod collect;
mod config_schema;
mod contracts;
mod coverage_corpus;
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
mod extension_surfaces;
mod extensions;
#[allow(dead_code)]
mod fake_registry;
mod field_types;
#[allow(deprecated)]
mod format;
mod format_corpus;
mod init;
mod installed_extensions;
mod mcp_add;
#[allow(deprecated)]
mod migrate;
mod navigation_parity;
mod parity;
mod pipeline;
mod product_commands;
mod product_rules;
mod publish;
mod query;
mod read_views;
mod registry;
mod schema_cache;
mod stats;
mod surface_parity;
mod trace;
mod watch;
mod watch_reload;
