//! The input kind table against the wire.

use super::*;

/// One input of each kind, every optional key set.
fn each() -> Vec<Input> {
    vec![
        Input::Files(FileRead {
            name: "sources".into(),
            field: None,
            glob: Some("src/**".into()),
            root: Some("project".into()),
            text: true,
        }),
        Input::Tests,
        Input::State,
        Input::Graph,
        Input::SlashCommands,
        Input::git().status().when("x.y"),
        Input::setting("@a/b", "x.y"),
        Input::CheckOutcomes,
    ]
}

#[test]
fn the_kind_table_spells_each_inputs_wire_tag_and_keys() {
    assert_eq!(
        InputKind::Files.wire().1,
        crate::declaration::struct_fields::<FileRead>()
    );
    let inputs = each();
    assert_eq!(
        inputs.iter().map(Input::kind).collect::<Vec<_>>(),
        InputKind::ALL
    );
    for input in inputs {
        let value = serde_json::to_value(&input).unwrap();
        assert_eq!(value["input"], input.tag());
        for key in value.as_object().unwrap().keys() {
            assert!(input_keys().contains(&key.as_str()), "{key}");
        }
    }
}
