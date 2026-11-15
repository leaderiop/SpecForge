//! A model doc's field tables (`spec/<area>.md`, one table under each `## <kind>` heading), read so a doc cannot drift
//! from the declaration: each row's field, type and flags, in the notation of `spec/README.md`.

/// The rows of the field table under `## <kind>` in the model doc `md`: field, type, flags, meaning.
pub fn table(md: &str, kind: &str) -> Vec<Vec<String>> {
    let table = gsd_kit::markdown::tables(md)
        .into_iter()
        .find(|t| t.heading == kind)
        .unwrap_or_else(|| panic!("no table under `## {kind}`"));
    assert_eq!(
        table.header,
        ["Field", "Type", "Flags", "Meaning"],
        "{kind}"
    );
    table.rows
}

/// A declared field (or block member) as the Type column writes it: `str`, `strs`, `int`, `int{1..10}`, `bool`,
/// `→ kind`, `→* kind`, `→* a | b`, `enum{a, b}`, and a block or block list `{member type [R], …}`. A `ts` column is
/// read as `str` (`assert_declared`).
pub fn written_type(field: &serde_json::Value) -> String {
    let target = || field["target_kind"].as_str().expect("a target kind");
    match field["field_type"].as_str().expect("a field type") {
        "string" => "str".to_string(),
        "string_list" => "strs".to_string(),
        "bool" => "bool".to_string(),
        "integer" => match (field["minimum"].as_i64(), field["maximum"].as_i64()) {
            (Some(lo), Some(hi)) => format!("int{{{lo}..{hi}}}"),
            (None, None) => "int".to_string(),
            _ => panic!("the model docs write no half-bounded int yet: {field}"),
        },
        "reference" => format!("→ {}", target()),
        "reference_list" => match field["target_kinds"].as_array() {
            Some(kinds) => {
                let kinds: Vec<&str> = kinds.iter().map(|k| k.as_str().expect("a kind")).collect();
                format!("→* {}", kinds.join(" | "))
            }
            None => format!("→* {}", target()),
        },
        "enum" => {
            let words: Vec<&str> = field["enum_values"]
                .as_array()
                .expect("enum values")
                .iter()
                .map(|w| w.as_str().expect("a word"))
                .collect();
            format!("enum{{{}}}", words.join(", "))
        }
        "block" | "block_list" => {
            let members: Vec<String> = field["members"]
                .as_array()
                .expect("members")
                .iter()
                .map(|m| {
                    let flags = written_flags(m);
                    let name = m["name"].as_str().expect("a name");
                    let written = format!("{name} {}", written_type(m));
                    if flags.is_empty() {
                        written
                    } else {
                        format!("{written} {flags}")
                    }
                })
                .collect();
            format!("{{{}}}", members.join(", "))
        }
        other => panic!("the model docs write no `{other}` field yet: give it its notation here"),
    }
}

/// The Flags column: `R` for a required field, `C` for a computed one, empty for neither.
pub fn written_flags(field: &serde_json::Value) -> String {
    match (field["required"] == true, field["writer"] == "computed") {
        (true, true) => panic!("a field is required or computed, never both: {field}"),
        (true, false) => "R".to_string(),
        (false, true) => "C".to_string(),
        (false, false) => String::new(),
    }
}

/// Holds every kind of `kinds` to its table in `md`: the declared fields in order, each with its Type and Flags.
pub fn assert_declared(md: &str, kinds: &[&str]) {
    for kind in kinds {
        let rows = table(md, kind);
        let names: Vec<String> = rows.iter().map(|row| row[0].clone()).collect();
        assert_eq!(super::fields_of(kind), names, "{kind}");
        for row in &rows {
            let field = super::field_of(kind, &row[0]);
            // A `ts` is declared a string: its RFC 3339 form is the extension's check, not the kernel's type.
            let written = if row[1] == "ts" { "str" } else { &row[1] };
            assert_eq!(written_type(&field), written, "{kind}.{}: Type", row[0]);
            assert_eq!(written_flags(&field), row[2], "{kind}.{}: Flags", row[0]);
        }
    }
}
