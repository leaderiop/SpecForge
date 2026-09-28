//! Doc drift guard (audit C7-03): the host-function tables in
//! `docs/extension-sdk.md` and `docs/extension-protocol.md` must enumerate
//! exactly the host functions the host's permission matrix recognizes
//! (`specforge_wasm::HOST_FUNCTIONS`) — no phantom functions documented
//! (the old `resolve_ref`), no real functions left undocumented.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use specforge_wasm::{CallSite, HOST_FUNCTIONS, is_host_function_allowed};

fn docs_dir() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("docs")
}

/// Extracts host-function names from the markdown table under `heading`
/// (matched as a line prefix, e.g. "## Host Functions"). Scanning is
/// confined to that section so attribute tables elsewhere in the doc
/// (e.g. the `#[extension]` attribute named `host_api`) cannot leak in.
fn documented_host_functions(doc: &str, heading: &str) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    let mut in_section = false;
    for line in doc.lines() {
        if in_section {
            if line.starts_with('#') {
                // The next heading ends the host-function section.
                break;
            }
            if !line.trim_start().starts_with('|') {
                continue;
            }
            let mut cells = line.split('|');
            cells.next(); // leading empty segment before the first '|'
            if let Some(first) = cells.next() {
                let name = first.trim().trim_matches('`').trim();
                if name.starts_with("host_") {
                    names.insert(name.to_string());
                }
            }
        } else if line.starts_with(heading) {
            in_section = true;
        }
    }
    names
}

fn actual_host_functions() -> BTreeSet<String> {
    HOST_FUNCTIONS.iter().map(|s| s.to_string()).collect()
}

#[test]
fn sdk_doc_host_function_table_matches_host_registry() {
    let doc = fs::read_to_string(docs_dir().join("extension-sdk.md"))
        .expect("docs/extension-sdk.md exists");
    let documented = documented_host_functions(&doc, "## Host Functions");
    let actual = actual_host_functions();
    assert_eq!(
        documented, actual,
        "docs/extension-sdk.md host-function table drifted from the host registry \
         (specforge_wasm::HOST_FUNCTIONS). Update the table and/or the registry so \
         they name exactly the same functions."
    );
}

#[test]
fn protocol_doc_host_function_table_matches_host_registry() {
    let doc = fs::read_to_string(docs_dir().join("extension-protocol.md"))
        .expect("docs/extension-protocol.md exists");
    let documented = documented_host_functions(&doc, "### Host Function Table");
    let actual = actual_host_functions();
    assert_eq!(
        documented, actual,
        "docs/extension-protocol.md host-function table drifted from the host registry \
         (specforge_wasm::HOST_FUNCTIONS). Update the table and/or the registry so \
         they name exactly the same functions."
    );
}

#[test]
fn no_phantom_host_function_names_in_docs() {
    // The registry must stay the sole source of truth: if a doc grows a
    // `host_*` first-cell table row that the permission matrix does not
    // recognize for ANY call site, it is a phantom function (C7-03).
    let all_sites = [
        CallSite::Validator,
        CallSite::Renderer,
        CallSite::Provider,
        CallSite::Parser,
        CallSite::Collector,
        CallSite::Analyzer,
    ];
    for doc_name in ["extension-sdk.md", "extension-protocol.md"] {
        let doc = fs::read_to_string(docs_dir().join(doc_name)).expect("doc file exists");
        let heading = if doc_name == "extension-sdk.md" {
            "## Host Functions"
        } else {
            "### Host Function Table"
        };
        for name in documented_host_functions(&doc, heading) {
            let recognized = all_sites
                .iter()
                .any(|site| is_host_function_allowed(*site, &name));
            assert!(
                recognized,
                "{doc_name} documents host function '{name}' that the host's \
                 permission matrix does not recognize"
            );
        }
    }
}

/// Every registry name must be documented in BOTH docs' tables, so the
/// per-function table checks above cannot pass vacuously on one side.
#[test]
fn both_docs_document_the_full_registry() {
    for doc_name in ["extension-sdk.md", "extension-protocol.md"] {
        let doc = fs::read_to_string(docs_dir().join(doc_name)).expect("doc file exists");
        let heading = if doc_name == "extension-sdk.md" {
            "## Host Functions"
        } else {
            "### Host Function Table"
        };
        let documented = documented_host_functions(&doc, heading);
        for name in HOST_FUNCTIONS {
            assert!(
                documented.contains(*name),
                "{doc_name} is missing host function '{name}' from its host-function table"
            );
        }
    }
}
