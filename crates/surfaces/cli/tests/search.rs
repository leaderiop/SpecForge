//! `specforge search <text>`: what the command searches, over a copy of the read-views fixture (`behavior alpha
//! "Alpha"`).

use std::path::Path;

use crate::support::{Cli, TestProject};

/// A copy of the read-views fixture, which configures no registry.
fn rv1() -> TestProject {
    TestProject::copy_of(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/read_views/rv1"),
    )
}

#[test]
fn search_is_the_registry_search_even_where_an_entity_matches() {
    let project = rv1();
    let run = Cli::on(&project).run(&["search", "alpha"]).exits(1);
    assert!(run.stderr.contains("E063"), "{}", run.stderr);
    assert!(run.stderr.contains("`search` needs one"), "{}", run.stderr);
}
