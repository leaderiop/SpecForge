//! The write journal (ADR 0289): a write records what it is about to replace under `.specforge/write.lock`, and the
//! next writer puts back what an interrupted one left half done.

use serde_json::json;
use specforge_ops::authoring::edit::{self, Edit};
use specforge_ops::view::ProjectView;
use specforge_package::Sha256;
use specforge_project::testing::GsdWrites;
use specforge_test_macros::test as specforge_test;
use std::sync::Arc;

use crate::plans::{ONE, TWO, compiled, project};

fn digest(text: &str) -> String {
    format!("sha256:{}", Sha256::of(text.as_bytes()))
}

#[ignore = "K5 T7"]
#[specforge_test(
    behavior = "roll_back_interrupted_write",
    verify = "an interrupted write is rolled back by the next writer"
)]
fn an_interrupted_write_is_rolled_back_by_the_next() {
    let p = project();
    // A write was killed after its first file: `03-01.spec` holds the text the write gave it, `03-02.spec` was not
    // reached.
    let written = ONE.replace("}\n", "  result pass\n}\n");
    p.write("03-01.spec", &written);
    p.write(
        ".specforge/write-journal.json",
        json!({"format": 1, "doing": "cannot apply the edits", "files": [
            {"path": "03-01.spec", "before": ONE, "after": digest(&written)},
            {"path": "03-02.spec", "before": TWO, "after": digest("never written\n")},
        ]})
        .to_string(),
    );
    let runtime = Arc::new(GsdWrites::new().runtime());
    let compile = compiled(&p, &runtime);
    // The next write changes a file the journal does not name.
    edit::run(
        &ProjectView::of(&compile),
        &[Edit::Retitle {
            entity_id: "v1.0".into(),
            title: "Uno".into(),
        }],
        false,
        specforge_ops::testing::caller(),
    )
    .unwrap();
    assert_eq!(p.read("03-01.spec"), ONE, "the file at `after` is put back");
    assert_eq!(
        p.read("03-02.spec"),
        TWO,
        "the file at `before` is left alone"
    );
    assert!(p.read("m.spec").contains("\"Uno\""));
    assert!(!p.root().join(".specforge/write-journal.json").exists());
}
