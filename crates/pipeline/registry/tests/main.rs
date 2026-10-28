#![allow(
    clippy::unwrap_used,
    reason = "a test's unwrap is its assertion: a None or an Err fails the test"
)]

mod build;
mod checks;
mod command_routes;
mod declarations;
mod plain_disk;
mod prompts;
mod registry_build;
mod rules;
mod support;
mod surface;
