//! The extensions the facts tests share (ADR 0327), served in process and written as the wire carries them:
//!
//! - [`CODE`] (`@t/code`): the kind `test`, written by a collector under the scheme `js`, with the fields `file` and
//!   `status`
//! - [`NODE`] (`@t/node`, a peer of code): the collector `node` (`collect__node`, detected by `package.json`, its
//!   report `.specforge/reports/node.xml`), and `unit` (`collect__unit`, detected by `package.json` too, its report
//!   `.specforge/reports/unit.json`), the reporting collector a mixed run pairs it with
//! - [`SPEC`] (`@t/spec`, a peer of code): the kind `truth`, whose `proof` lists the tests that prove it
//!
//! The collector answers what the test gives it ([`Collected::answering`]): the fixture holds no handler, so a test
//! spells the facts it expects the host to store. [`Collected::peering`] puts a builtin (`@code/core`) in [`CODE`]'s
//! place.

use serde_json::{Value, json};
use specforge_extension_sdk::DeclaredCategory;
use specforge_extension_sdk::prelude::{ContributionsBuilder, ExtensionMeta};
use specforge_wasm::WasmCallResult;
use specforge_wasm::testing::InProcessRuntime;

/// The extension that declares the collected kind `test`.
pub const CODE: &str = "@t/code";
/// A peer of [`CODE`] that declares the collector `node`.
pub const NODE: &str = "@t/node";
/// The extension whose `truth` cites tests.
pub const SPEC: &str = "@t/spec";

/// The three extensions' names, as a project enables them.
pub const COLLECTED_EXTENSIONS: [&str; 3] = [CODE, NODE, SPEC];

/// The report `node` reads, from the project root.
pub const NODE_REPORT: &str = ".specforge/reports/node.xml";
/// The report `unit` reads, from the project root.
pub const UNIT_REPORT: &str = ".specforge/reports/unit.json";

/// The kinds of [`CODE`], as the wire carries them.
pub fn collected_wire() -> Value {
    json!([{"name": "test", "keyword": "test", "writer": "collector", "schemes": ["js"], "fields": [
        {"name": "file", "field_type": "string"},
        {"name": "status", "field_type": "string"},
        {"name": "covers", "field_type": "reference_list"},
    ]}])
}

/// A `collect__node` answer: no results, and `facts` as the wire carries them.
pub fn collected_answer(facts: Value) -> WasmCallResult {
    WasmCallResult::Ok(
        json!({"entity_results": [], "facts": facts})
            .to_string()
            .into_bytes(),
    )
}

/// The three extensions, served in process.
#[derive(Default)]
pub struct Collected {
    answers: Vec<(String, String, WasmCallResult)>,
    /// The builtin [`NODE`] and [`SPEC`] peer in place of [`CODE`] ([`Collected::peering`]).
    builtin: Option<String>,
}

impl Collected {
    pub fn new() -> Self {
        Self::default()
    }

    /// `export` of `extension` answers `result` (the first answer to an export wins).
    pub fn answering(mut self, extension: &str, export: &str, result: WasmCallResult) -> Self {
        self.answers
            .push((extension.to_string(), export.to_string(), result));
        self
    }

    /// [`NODE`] and [`SPEC`] peer the builtin `builtin` (`@code/core`, whose kind `test` they write and cite), and
    /// [`CODE`] is not served: its `test` would clash with the builtin's (E026). The project enables `builtin`
    /// beside them, and the runtime given to [`Self::over`] loads its component beside
    /// ([`InProcessRuntime::beside`]).
    pub fn peering(mut self, builtin: &str) -> Self {
        self.builtin = Some(builtin.to_string());
        self
    }

    /// The runtime serving the three extensions and the answers given.
    pub fn runtime(self) -> InProcessRuntime {
        self.over(InProcessRuntime::new())
    }

    /// `runtime` serving the three extensions too (two when [`Self::peering`] a builtin), and the answers given (a
    /// mixed run with another fixture's).
    pub fn over(self, runtime: InProcessRuntime) -> InProcessRuntime {
        let peer = self.builtin.unwrap_or_else(|| CODE.to_string());
        let runtime = match peer.as_str() {
            CODE => runtime.with(code),
            _ => runtime,
        };
        let (node_peer, spec_peer) = (peer.clone(), peer);
        let mut runtime = runtime
            .with(move || node(&node_peer))
            .with(move || spec(&spec_peer));
        for (extension, export, result) in self.answers {
            runtime = runtime.answer_raw(&extension, &export, result);
        }
        runtime
    }
}

/// Write `collector`'s store of `facts` (as the wire carries them, from [`NODE`]) under `key` in the project at
/// `root`, and the `specforge-harness` line that records it, as a collect leaves them (ADR 0327 D2, D4).
pub fn stored(root: &std::path::Path, collector: &str, key: &str, facts: Value) {
    use crate::facts::Store;
    use crate::harness::{LEDGER_FILE, Ledger, Line};
    let facts = serde_json::from_value(facts).expect("facts as the wire carries them");
    let bytes = Store::new(collector, NODE, key, facts).bytes();
    let path = root.join(Store::path(collector, key));
    std::fs::create_dir_all(path.parent().expect("a store path has a directory"))
        .expect("the store's directory is made");
    std::fs::write(&path, &bytes).expect("the store is written");
    let ledger = root.join(LEDGER_FILE);
    let mut lines = Ledger::parse(&std::fs::read_to_string(&ledger).unwrap_or_default()).ledger;
    lines.insert(Line::facts(collector, key, &bytes));
    std::fs::write(ledger, lines.canonical()).expect("the ledger is written");
}

fn code() -> ContributionsBuilder {
    let mut c = ContributionsBuilder::new(ExtensionMeta::new(CODE, "1.0.0"));
    c.raw_category(DeclaredCategory::Entities, collected_wire());
    c
}

/// The collectors, peering `peer`, the extension that declares `test`.
fn node(peer: &str) -> ContributionsBuilder {
    let mut c = ContributionsBuilder::new(ExtensionMeta::new(NODE, "1.0.0"));
    c.peer(peer, "^1.0");
    c.raw_category(
        DeclaredCategory::Collectors,
        json!([
            {"name": "node", "input_formats": ["junit"], "export": "collect__node",
             "auto_detect": {"file_patterns": ["package.json"], "env_vars": []}, "report": NODE_REPORT},
            {"name": "unit", "input_formats": ["json"], "export": "collect__unit",
             "auto_detect": {"file_patterns": ["package.json"], "env_vars": []}, "report": UNIT_REPORT},
        ]),
    );
    c
}

/// The citing kind `truth`, peering `peer`, the extension that declares `test`.
fn spec(peer: &str) -> ContributionsBuilder {
    let mut c = ContributionsBuilder::new(ExtensionMeta::new(SPEC, "1.0.0"));
    c.peer(peer, "^1.0");
    c.raw_category(
        DeclaredCategory::Entities,
        json!([{"name": "truth", "keyword": "truth", "fields": [
            {"name": "proof", "field_type": "reference_list", "target_kind": "test"},
        ]}]),
    );
    c
}

#[cfg(test)]
mod tests {
    use super::*;
    use specforge_wasm::WasmRuntime;

    #[test]
    fn the_fixture_serves_a_collected_kind_a_collector_and_a_citing_kind() {
        let runtime = Collected::new().runtime();
        let declarations: Vec<_> = COLLECTED_EXTENSIONS
            .iter()
            .map(|name| specforge_wasm::protocol::load_declaration(&runtime, name).unwrap())
            .collect();

        let test = &declarations[0].declaration.entities[0];
        assert_eq!(
            (test.writer.as_deref(), &test.schemes[..]),
            (Some("collector"), &["js".to_string()][..])
        );
        assert_eq!(
            declarations[1].declaration.collectors[0].export,
            "collect__node"
        );
        assert_eq!(
            declarations[2].declaration.entities[0].fields[0]
                .target_kind
                .as_deref(),
            Some("test")
        );
        assert!(runtime.unload(CODE));
    }
}
