//! The export each operation is called by in the call tests, and how a test performs one operation with an empty
//! input.

use serde_json::json;
use specforge_extension_sdk::prelude::*;
use specforge_extension_sdk::{MigrationInput, ScanRequest};
use specforge_protocol_types::DeclaredCategory;
use specforge_wasm::{CallError, ExtensionCalls, Operation};

use crate::calls::{command_input, validator_context};

/// The export each operation is called by in these tests.
pub fn export_of(operation: Operation) -> &'static str {
    match operation {
        Operation::Handshake => "__handshake",
        Operation::Describe => "__describe",
        Operation::Command => "cmd__raw",
        Operation::McpTool => "mcp__tool",
        Operation::McpResource => "mcp__resource",
        Operation::Pass => "__pass_audit",
        Operation::Collect => "collect__x",
        Operation::Validate => "validate__x",
        Operation::Scan => "scan__x",
        Operation::Migrate => "migrate__x",
        Operation::Prompt => "prompt__x",
        Operation::Render => "render__",
        Operation::Hook => "hook__x",
    }
}

/// Perform `operation` on `extension`, its answer dropped.
pub fn perform(
    calls: &ExtensionCalls,
    operation: Operation,
    extension: &str,
) -> Result<(), CallError> {
    let export = export_of(operation);
    match operation {
        Operation::Handshake => calls.handshake(extension).map(drop),
        Operation::Describe => calls
            .describe(extension, DeclaredCategory::Entities)
            .map(drop),
        Operation::Command => calls
            .run_command(extension, export, &command_input())
            .map(drop),
        Operation::McpTool => calls.call_mcp_tool(extension, export, &json!({})).map(drop),
        Operation::McpResource => calls
            .read_mcp_resource(extension, export, "u://r")
            .map(drop),
        Operation::Pass => {
            let input = ExtensionCalls::encode(&PassInput::default());
            calls.run_pass(extension, "audit", &input).map(drop)
        }
        Operation::Collect => calls
            .collect(extension, export, &CollectInput::default())
            .map(drop),
        Operation::Validate => calls
            .validate(extension, export, &validator_context())
            .map(drop),
        Operation::Scan => calls
            .scan(
                extension,
                export,
                &ScanRequest {
                    file_path: "a.rs".into(),
                    content: String::new(),
                },
            )
            .map(drop),
        Operation::Migrate => calls
            .migrate(extension, export, &MigrationInput::default())
            .map(drop),
        Operation::Prompt => crate::prompt_wire::render(calls, extension, export),
        Operation::Render => calls.render(extension, &Default::default()).map(drop),
        Operation::Hook => calls
            .run_hook(extension, export, &Default::default(), 5_000)
            .map(drop),
    }
}
