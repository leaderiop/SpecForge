use std::collections::BTreeMap;

use super::*;

#[test]
fn escape_writes_controls_as_code_points() {
    assert_eq!(escape("\u{1b}[31mred"), "\\u{1b}[31mred");
    assert_eq!(escape("a\u{202e}b"), "a\\u{202e}b");
    assert_eq!(escape("one\ntwo\tthree"), "one\\u{a}two\\u{9}three");
    assert_eq!(escape("nul\0"), "nul\\u{0}");
    assert_eq!(escape("del\u{7f} c1\u{85}"), "del\\u{7f} c1\\u{85}");
    for c in [
        '\u{061c}', '\u{200e}', '\u{200f}', '\u{202a}', '\u{202d}', '\u{2066}', '\u{2069}',
    ] {
        assert_eq!(escape(&c.to_string()), format!("\\u{{{:x}}}", c as u32));
    }
}

#[test]
fn escape_leaves_other_text_unchanged() {
    let text = "Tâche — « déjà » 日本語 `code` \\u{1b} 'quoted' $(id)";
    assert_eq!(escape(text), text);
}

#[test]
fn escape_shown_caps_a_value_at_200_characters() {
    let long = "é".repeat(10_000);
    let out = shown(&long);
    assert_eq!(out.chars().count(), SHOWN_MAX + 1);
    assert!(out.ends_with('…'));
    assert_eq!(shown(&"x".repeat(SHOWN_MAX)), "x".repeat(SHOWN_MAX));
    // The cut comes before the escape, so no escape is split.
    let hostile = "\u{1b}".repeat(300);
    assert_eq!(shown(&hostile), format!("{}…", "\\u{1b}".repeat(SHOWN_MAX)));
}

fn hostile() -> Vec<Entry> {
    let at = |line| Source {
        file: ".planning/PROJECT.md".to_string(),
        line,
    };
    vec![
        Entry::at(
            &at(3),
            "## What This Is",
            Outcome::Unmapped,
            "title \u{1b}[31mred\u{1b}[0m\nforged: mapped".to_string(),
        ),
        Entry::at(
            &at(3),
            "## Core",
            Outcome::Redundant,
            "bidi \u{202e}txt.exe".to_string(),
        ),
        Entry::at(&at(9), "", Outcome::Filled, "nul \0 here".to_string()),
        Entry::file(".planning/state.json", Outcome::Runtime, "x".repeat(10_000)),
        Entry::file(
            ".planning/notes\n.txt",
            Outcome::Unmapped,
            "unknown file".to_string(),
        ),
    ]
}

fn filled(entries: Vec<Entry>) -> Report {
    let mut report = Report::default();
    for entry in entries {
        report.push(entry);
    }
    report.setting("specforge settings set @gsd/core mode 'yolo'".to_string());
    report.setting("specforge settings set @gsd/core granularity 'fine'".to_string());
    report.owed("/gsd-verify-work 01".to_string());
    report.authored_after_import("milestone v1.0: its title".to_string());
    report.minted(Minted {
        kind: "requirement".to_string(),
        old: "Out of Scope row 1".to_string(),
        new: "OOS-01".to_string(),
    });
    report
}

#[test]
fn report_is_stable() {
    let counts = BTreeMap::from([("requirement", 1)]);
    let mut reversed = hostile();
    reversed.reverse();
    let (a, b) = (filled(hostile()), filled(reversed));
    assert_eq!(a.text(&counts), b.text(&counts));
    assert_eq!(
        serde_json::to_string(&a.payload(&counts)).expect("JSON"),
        serde_json::to_string(&b.payload(&counts)).expect("JSON")
    );
}

#[test]
fn report_text_is_one_escaped_line_per_entry() {
    let report = filled(hostile());
    let text = report.text(&BTreeMap::new());
    let lines: Vec<&str> = text.lines().collect();
    // Five entries, then the headed sections, then the count line: no echoed string breaks a line.
    assert_eq!(
        &lines[..5],
        [
            ".planning/PROJECT.md:3  ## Core  redundant  bidi \\u{202e}txt.exe",
            ".planning/PROJECT.md:3  ## What This Is  unmapped  title \\u{1b}[31mred\\u{1b}[0m\\u{a}forged: mapped",
            ".planning/PROJECT.md:9  -  filled  nul \\u{0} here",
            ".planning/notes\\u{a}.txt  -  unmapped  unknown file",
            &format!(
                ".planning/state.json  -  runtime  {}…",
                "x".repeat(SHOWN_MAX)
            ),
        ]
    );
    assert!(text.contains("\nsettings: run each line to keep the setting\n  specforge settings set @gsd/core granularity 'fine'\n  specforge settings set @gsd/core mode 'yolo'\n"));
    assert!(text.contains("\nowed: record each verdict anew\n  /gsd-verify-work 01\n"));
    assert!(text.ends_with("import: 0 entities (), 5 report entries\n"));
    assert!(!text.chars().any(|c| hidden(c) && c != '\n'));
}

#[test]
fn report_payload_has_every_section() {
    let payload = filled(hostile()).payload(&BTreeMap::from([("requirement", 1)]));
    let mut keys: Vec<&String> = payload.as_object().expect("an object").keys().collect();
    keys.sort();
    assert_eq!(
        keys,
        [
            "authored_after_import",
            "counts",
            "entries",
            "format",
            "ids",
            "owed",
            "settings"
        ]
    );
    assert_eq!(payload["format"], 1);
    assert_eq!(
        payload["ids"],
        json!([{"kind": "requirement", "old": "Out of Scope row 1", "new": "OOS-01"}])
    );
    let first = &payload["entries"][0];
    assert_eq!(first["reason"], "bidi \\u{202e}txt.exe");
    assert_eq!(first["line"], 3);
    assert_eq!(payload["entries"][3]["line"], Value::Null);
}
