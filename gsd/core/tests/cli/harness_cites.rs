//! What a played verifier cites: a verdict on a truth reads only what it cites (ADR 0377), so each truth of the
//! parity fixtures cites the files its text is about. A child of `harness.rs`; `@gsd/security`'s tests compile it
//! through their include.
//!
//! The table is chosen so a file is cited by a strict subset of the truths: on the taskboard `src/cli.js` is cited by
//! no truth (only 01-02's artifact and key link name it), on the vault `src/cli.js` and `test/vault.test.js` each by
//! one of the two, on the ledger `tests/entries.rs` and `tests/totals.rs` each by one plan's truths. No truth cites a
//! test fact: a cited test digests the collected results, and the committed verdicts must stay fresh in an
//! uncollected copy.

use super::Project;

/// `(truth title, the files its verdict cites)` of the taskboard, vault and ledger fixtures.
const CITES: &[(&str, &[&str])] = &[
    // taskboard
    (
        "A task added to a board has the next integer id",
        &["src/tasks.js", "test/tasks.test.js"],
    ),
    (
        "A task is completed by its id",
        &["src/tasks.js", "test/tasks.test.js"],
    ),
    (
        "Completing a missing id fails with a message naming it",
        &["src/tasks.js", "test/tasks.test.js"],
    ),
    // vault
    (
        "A sealed vault opens to what was sealed, and to nothing else",
        &["src/vault.js", "test/vault.test.js"],
    ),
    (
        "A secret set from the command line is read back by its name",
        &["src/cli.js", "src/vault.js"],
    ),
    // ledger
    (
        "An added entry keeps its cents and its memo",
        &["src/lib.rs", "tests/entries.rs"],
    ),
    (
        "A memo with a tab or a line break is refused",
        &["src/lib.rs", "tests/entries.rs"],
    ),
    (
        "The total is the sum of every entry",
        &["src/lib.rs", "tests/totals.rs"],
    ),
    (
        "A total beyond the range of an i32 is exact",
        &["src/lib.rs", "tests/totals.rs"],
    ),
];

impl Project {
    /// The node `id` names, as `specforge query <id> --depth 0 --format json` answers it.
    pub(super) fn node(&self, id: &str) -> serde_json::Value {
        let query = self.run(&["query", id, "--depth", "0", "--format", "json"]);
        let graph: serde_json::Value = serde_json::from_str(&query.stdout)
            .unwrap_or_else(|e| panic!("query is JSON ({e}): {}", query.all()));
        graph["nodes"][0].clone()
    }

    /// The `--cited` JSON list of a verdict on `truth`: the files of its row in the fixtures' table, else the files of
    /// its plan's `files_modified`. A test project's truth with neither panics: it must have something to cite.
    pub fn cites(&self, truth: &str) -> String {
        let node = self.node(truth);
        let title = node["title"].as_str().unwrap_or_default();
        let files: Vec<String> = match CITES.iter().find(|(t, _)| *t == title) {
            Some((_, files)) => files.iter().map(|f| f.to_string()).collect(),
            None => {
                let plan = node["fields"]["plan"].as_str().unwrap_or_default();
                self.fields_of(plan)["files_modified"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(|f| Some(f.as_str()?.trim().to_string()))
                    .collect()
            }
        };
        assert!(
            !files.is_empty(),
            "truth '{truth}' has nothing to cite: give it a row in the citation table or its plan `files_modified`"
        );
        let list: Vec<serde_json::Value> = files
            .into_iter()
            .map(|file| serde_json::json!({ "file": file }))
            .collect();
        serde_json::Value::Array(list).to_string()
    }

    /// The options a played verifier's verdict on `id` takes: `--cited` with [`Project::cites`] on a truth, none on
    /// anything else (an artifact and a key link read the files they name).
    pub fn citing(&self, id: &str) -> Vec<String> {
        if self.node(id)["kind"] == "truth" {
            vec!["--cited".to_string(), self.cites(id)]
        } else {
            Vec::new()
        }
    }
}

/// The `specforge query --inputs` flags naming what a `--cited` JSON list cites, so the basis read is the one the
/// citing verdict is recorded over (citations are part of the basis). A list that is not one (a test of the command's
/// own refusal) names nothing: the command refuses it.
pub(super) fn cite_flags(cited: &str) -> Vec<String> {
    let list: Vec<serde_json::Value> = serde_json::from_str(cited).unwrap_or_default();
    list.iter()
        .filter_map(|cite| {
            let (flag, value) = match (cite.get("file"), cite.get("test"), cite.get("entity")) {
                (Some(file), _, _) => ("--cite-file", file),
                (_, Some(test), _) => ("--cite-test", test),
                (_, _, Some(entity)) => ("--cite", entity),
                _ => return None,
            };
            Some([flag.to_string(), value.as_str()?.to_string()])
        })
        .flatten()
        .collect()
}
