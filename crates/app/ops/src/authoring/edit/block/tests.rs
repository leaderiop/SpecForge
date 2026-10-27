use super::*;
use crate::testing::Viewed;
use specforge_project::testing::Assembled;

const SPEC: &str = "spec \"p\" {\n  version \"0.1.0\"\n}\n";

#[specforge_test_macros::test(
    behavior = "validate_typed_edit",
    verify = "a block an edit leaves unreadable is refused edit_unreadable; one unreadable before it, block_unreadable"
)]
fn a_block_the_edit_leaves_unreadable_is_the_edits_defect() {
    let fixture = Assembled::new();
    let view = fixture.view();
    let site = Site {
        view: &view,
        scopes: view.graph().scopes(),
        key: "main.spec",
        id: "p",
        kind: specforge_common::structural::SPEC,
        ordinal: 0,
        config: FormatConfig::default(),
    };
    let mut block = Block::open(&site, SPEC).unwrap();
    let close = block.close();
    block.lines.insert(close, "  ??? )(".to_string());
    assert_eq!(
        block.finish(&site).unwrap_err().error.code,
        "edit_unreadable"
    );
    assert_eq!(
        canonical(&site, &block.text()).unwrap_err().error.code,
        "block_unreadable"
    );
}
