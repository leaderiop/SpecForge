//! The entity snapshot (ADR 0019): every entity of one built graph as every
//! check after the build reads it. For now it holds the field-text rule,
//! the one string a field value is to a declarative rule, a custom
//! validator and a compiler pass.

use specforge_parser::FieldValue;

/// A field value's text: the one rule every reader after the graph build
/// shares (ADR 0019). Scalars as written; lists of strings or references
/// and mixed lists joined by `", "`; variant lists and type unions by
/// `" | "`; expressions by `", "` in their display form; verify statements'
/// texts by `"; "`; a block's keys by `", "`. Empty values are `""`.
///
/// A joined list cannot be split back when an item itself contains the
/// joiner (`["a, b", "c"]` is `"a, b, c"`).
pub fn field_text(value: &FieldValue) -> String {
    // No `_` arm: a new variant does not compile until it has a text.
    match value {
        FieldValue::String(s) | FieldValue::Identifier(s) | FieldValue::Date(s) => s.clone(),
        FieldValue::Integer(n) => n.to_string(),
        FieldValue::Boolean(b) => b.to_string(),
        FieldValue::StringList(items) => items.join(", "),
        FieldValue::ReferenceList(refs) => refs
            .iter()
            .map(|r| r.id.as_str())
            .collect::<Vec<_>>()
            .join(", "),
        FieldValue::VariantList(members) | FieldValue::TypeUnion(members) => members.join(" | "),
        FieldValue::MixedList(items) => items.iter().map(field_text).collect::<Vec<_>>().join(", "),
        FieldValue::Expression(exprs) => exprs
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(", "),
        FieldValue::VerifyList(statements) => statements
            .iter()
            .map(|s| s.description.as_str())
            .collect::<Vec<_>>()
            .join("; "),
        FieldValue::Block(block) => block
            .entries()
            .iter()
            .map(|e| e.key.as_str())
            .collect::<Vec<_>>()
            .join(", "),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use specforge_test_macros::test as specforge_test;

    /// Every field of `source`'s first entity: key, variant name, text.
    fn texts(source: &str) -> Vec<(String, &'static str, String)> {
        let parsed = specforge_parser::parse(source, "t.spec");
        assert!(parsed.errors.is_empty(), "{:?}", parsed.errors);
        parsed.entities[0]
            .fields
            .entries()
            .iter()
            .map(|e| {
                let variant = match &e.value {
                    FieldValue::String(_) => "String",
                    FieldValue::Identifier(_) => "Identifier",
                    FieldValue::Date(_) => "Date",
                    FieldValue::Integer(_) => "Integer",
                    FieldValue::Boolean(_) => "Boolean",
                    FieldValue::StringList(_) => "StringList",
                    FieldValue::ReferenceList(_) => "ReferenceList",
                    FieldValue::VariantList(_) => "VariantList",
                    FieldValue::TypeUnion(_) => "TypeUnion",
                    FieldValue::MixedList(_) => "MixedList",
                    FieldValue::Expression(_) => "Expression",
                    FieldValue::VerifyList(_) => "VerifyList",
                    FieldValue::Block(_) => "Block",
                };
                (e.key.to_string(), variant, field_text(&e.value))
            })
            .collect()
    }

    #[specforge_test(
        behavior = "snapshot_entities_once",
        verify = "a variant list or type union is its members joined by ' | ', a mixed list or expression group its items joined by ', '"
    )]
    fn field_text_of_every_variant() {
        let source = r#"item x "X" {
  title "a title"
  owner alice
  due 2026-10-06
  count 42
  active true
  labels ["a, b", "c"]
  needs [y, z]
  values [low, high]
  shape string | string[]
  mix [1, true, two]
  metric expr { latency < 10ms, load > 5 }
  ensures {
    done "it is done"
    kept "it is kept"
  }
  verify unit "x works"
  verify "x holds"
}
"#;
        let expected = [
            ("title", "String", "a title"),
            ("owner", "Identifier", "alice"),
            ("due", "Date", "2026-10-06"),
            ("count", "Integer", "42"),
            ("active", "Boolean", "true"),
            ("labels", "StringList", "a, b, c"),
            ("needs", "ReferenceList", "y, z"),
            ("values", "VariantList", "low | high"),
            ("shape", "TypeUnion", "string | string[]"),
            ("mix", "MixedList", "1, true, two"),
            ("metric", "Expression", "latency < 10ms, load > 5"),
            ("ensures", "Block", "done, kept"),
            ("verify", "VerifyList", "x works; x holds"),
        ];
        let actual = texts(source);
        assert_eq!(
            actual,
            expected
                .iter()
                .map(|(k, v, t)| (k.to_string(), *v, t.to_string()))
                .collect::<Vec<_>>()
        );
    }

    #[specforge_test(
        behavior = "snapshot_entities_once",
        verify = "an empty list or block is written, with empty text, never left out or null"
    )]
    fn an_empty_value_has_empty_text() {
        let actual = texts("item x \"X\" {\n  values []\n  tags []\n  requires {\n  }\n}\n");
        assert_eq!(
            actual,
            [
                ("values".to_string(), "VariantList", String::new()),
                ("tags".to_string(), "ReferenceList", String::new()),
                ("requires".to_string(), "Block", String::new()),
            ]
        );
    }
}
