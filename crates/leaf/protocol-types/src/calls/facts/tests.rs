//! The fact payload's builders.

use super::*;

fn place(line: usize) -> FactLocation {
    FactLocation {
        file: "src/a.rs".into(),
        start_line: line,
        start_col: 5,
        end_line: line,
        end_col: 8,
    }
}

#[test]
fn fact_name_appends_the_id_once_and_records_each_place() {
    let fact = Fact::new("item", "rust:a::f")
        .name("names", "TR1", place(1))
        .name("names", "TR2", place(2))
        .name("names", "TR1", place(3));

    assert_eq!(fact.fields["names"], serde_json::json!(["TR1", "TR2"]));
    assert_eq!(fact.named_at["names"]["TR1"], [place(1), place(3)]);
    assert_eq!(fact.named_at["names"]["TR2"], [place(2)]);
    assert!(fact.at.is_empty());
    assert_eq!(fact.at(place(9)).at, [place(9)]);
}
