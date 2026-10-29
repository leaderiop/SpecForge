//! Every harness (`H`) field has exactly one writer: a writing command, or the collector `gsd-git`. The table is
//! `gsd_core::commands::WRITERS`; this pins it against the kinds the extension declares.

use std::collections::BTreeSet;

use gsd_core::commands::{UNWRITTEN, WRITERS};

fn harness_fields() -> BTreeSet<(String, String)> {
    let text = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/golden/kinds.json"
    ))
    .expect("the kinds golden");
    let golden: serde_json::Value = serde_json::from_str(&text).expect("JSON");
    golden["entities"]["items"]
        .as_array()
        .expect("kinds")
        .iter()
        .flat_map(|kind| {
            let name = kind["name"].as_str().unwrap_or_default().to_string();
            kind["fields"]
                .as_array()
                .into_iter()
                .flatten()
                .filter(|f| f["writer"] == "harness")
                .map(move |f| {
                    (
                        name.clone(),
                        f["name"].as_str().unwrap_or_default().to_string(),
                    )
                })
        })
        .collect()
}

#[test]
fn every_h_field_has_one_writer() {
    let declared = harness_fields();
    assert_eq!(declared.len(), 33, "the 33 harness fields of the model");
    let rows: Vec<(String, String)> = WRITERS
        .iter()
        .map(|(k, f, _)| (k.to_string(), f.to_string()))
        .collect();
    let unwritten: Vec<(String, String)> = UNWRITTEN
        .iter()
        .map(|(k, f)| (k.to_string(), f.to_string()))
        .collect();
    let mut listed: BTreeSet<(String, String)> = rows.iter().cloned().collect();
    assert_eq!(listed.len(), rows.len(), "a field has two writer rows");
    for field in &unwritten {
        assert!(
            listed.insert(field.clone()),
            "{field:?} is both written and unwritten"
        );
    }
    assert_eq!(listed, declared, "the rows and the H fields differ");
    assert_eq!(
        (WRITERS.len(), UNWRITTEN.len()),
        (30, 3),
        "30 fields have a writer, 3 wait for a kernel answer"
    );
}

#[test]
fn each_writer_is_a_writing_command_or_the_git_collector() {
    let declaration = gsd_core::build().declaration();
    let writing: BTreeSet<&str> = declaration
        .surfaces
        .commands
        .iter()
        .filter(|c| c.writes)
        .map(|c| c.id.as_str())
        .collect();
    let collectors: BTreeSet<&str> = declaration
        .collectors
        .iter()
        .map(|c| c.name.as_str())
        .collect();
    for (kind, field, writer) in WRITERS {
        assert!(
            writing.contains(writer) || collectors.contains(writer),
            "{kind}.{field}: '{writer}' is neither a writing command ({writing:?}) nor a collector ({collectors:?})"
        );
    }
}

/// A row is a claim: the command it names declares itself and mentions the field it writes. A row for a command that
/// does not write the field (a typo, a moved write) fails here.
#[test]
fn each_row_names_a_command_that_writes_its_field() {
    fn sources(dir: &std::path::Path, out: &mut Vec<String>) {
        for entry in std::fs::read_dir(dir).expect("read dir").flatten() {
            let path = entry.path();
            if path.is_dir() {
                sources(&path, out);
            } else if path.extension().is_some_and(|e| e == "rs") && !path.ends_with("tests.rs") {
                out.push(std::fs::read_to_string(&path).expect("read"));
            }
        }
    }
    let mut files = Vec::new();
    sources(
        &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/commands"),
        &mut files,
    );
    for (kind, field, writer) in WRITERS {
        if *writer == "gsd-git" {
            continue;
        }
        let declares = format!("c.command(\"{writer}\"");
        assert!(
            files
                .iter()
                .any(|f| f.contains(&declares) && f.contains(&format!("\"{field}\""))),
            "{kind}.{field}: no file declaring `{writer}` mentions \"{field}\""
        );
    }
}
