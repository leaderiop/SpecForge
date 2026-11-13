//! A discussion log entry's fields.

use super::checks_clean;
use crate::harness::project;

/// A log entry keeps the user's notes and assumptions mode's confidence and evidence; a confidence outside the three
/// levels is refused.
#[test]
fn a_log_entry_keeps_its_notes_confidence_and_evidence() {
    let p = project("discuss");
    let entry = |confidence: &str| {
        format!(
            r#"[{{"area": "Layout", "question": "Cards or rows?", "options": ["Cards", "Rows"], "chosen": "Cards", "mode": "assumptions", "notes": "Rows read poorly on a phone", "confidence": "{confidence}", "evidence": ["src/board.js", "src/list.js"]}}]"#
        )
    };
    let new = |confidence: &str| {
        p.run(&[
            "new",
            "discussion",
            "01-DISCUSSION-LOG",
            "Core discussion",
            "--in",
            "spec/discuss.spec",
            "--phase",
            "v1.0/01",
            "--entries",
            &entry(confidence),
        ])
    };
    let refused = new("certain");
    assert_ne!(refused.code, 0, "{}", refused.all());
    assert!(refused.all().contains("confidence"), "{}", refused.all());
    let written = new("likely");
    assert_eq!(written.code, 0, "{}", written.all());
    checks_clean(&p);
    let read = |pick: &str| {
        let run = p.run(&[
            "query",
            "v1.0/01/01-DISCUSSION-LOG",
            "--depth",
            "0",
            "--pick",
            pick,
        ]);
        assert_eq!(run.code, 0, "{}", run.all());
        run.stdout.trim().to_string()
    };
    assert_eq!(
        read(".nodes[0].fields.entries[0].notes"),
        "Rows read poorly on a phone"
    );
    assert_eq!(read(".nodes[0].fields.entries[0].confidence"), "likely");
    assert_eq!(
        read(".nodes[0].fields.entries[0].evidence[1]"),
        "src/list.js"
    );
}
