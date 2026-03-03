use clap::Args;

const SCHEMA: &str = include_str!("../../../../schema/specforge.schema.json");

#[derive(Args)]
pub struct SchemaArgs;

/// Print the JSON Schema for specforge.json to stdout.
pub fn run(_args: SchemaArgs) -> i32 {
    println!("{SCHEMA}");
    0
}
