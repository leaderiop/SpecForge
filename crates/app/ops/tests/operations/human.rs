//! A human-only kind, field or member is refused in every agent-facing write (ADR 0289); `override` of the
//! `@t/core` fixture is one.

use std::sync::Arc;

use serde_json::json;
use specforge_ops::view::ProjectView;
use specforge_project::testing::{CORE, GsdWrites};
use specforge_test_macros::test as specforge_test;

use crate::plans::{project, refused};

const OVERRIDE: &str = "override O1 \"O\" {\n  phase 03\n}\n";

#[ignore = "K5 T10"]
#[specforge_test(
    behavior = "refuse_human_only_write",
    verify = "a human-only value is refused in an agents write"
)]
fn a_human_only_value_is_refused_in_an_agents_write() {
    let p = project();
    // From a command's plan.
    let create = json!([{"op": "create", "kind": "override", "id": "O1", "title": "O",
        "file": "o.spec", "revision": "absent", "fields": {"phase": "03"}}]);
    assert_eq!(
        refused(&p, CORE, "core", "record", create).as_deref(),
        Some("human_only")
    );
    assert!(!p.root().join("o.spec").exists());
    // From a typed edit, which MCP `specforge.edit` and the CLI without a terminal both are.
    let compile = p.compile_in(Arc::new(GsdWrites::new().runtime()));
    let edit = crate::edit::create("override", "O1", "o.spec", &[]);
    let error = specforge_ops::authoring::edit::run(
        &ProjectView::of(&compile),
        &[edit],
        true,
        &specforge_ops::authoring::apply::testing::origin(),
        specforge_ops::testing::caller(),
    )
    .unwrap_err();
    assert_eq!(error.error.code, "human_only");
    // By hand it compiles clean.
    let by_hand = project().file("o.spec", OVERRIDE);
    let errors: Vec<_> = by_hand
        .compile_in(Arc::new(GsdWrites::new().runtime()))
        .diagnostics()
        .into_iter()
        .filter(|d| d.code == "E080")
        .collect();
    assert!(errors.is_empty(), "{errors:?}");
}
