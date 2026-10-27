//! The writing GSD-shaped extensions the K5 tests share (ADR 0289), served in process and written as the wire
//! carries them, so an attribute the host does not read yet is dropped by the descriptor and the test that needs it
//! fails, not the fixture:
//!
//! - [`CORE`] (`@t/core`): the kinds of [`super::gsd_history_wire`] and `override` (human-only), `commit`; a writing
//!   command `record`, a command `quiet` that declares no `writes`, a writing collector `git`
//! - [`UI`] (`@t/ui`, a peer of core): a writing collector `dom`, a writing command `mark`
//! - [`OTHER`] (`@t/other`, no peer): a writing command `record`
//!
//! A command answers what the test gives it ([`GsdWrites::answering`]): the fixture holds no handler, so a test
//! spells the plan it expects the host to read. [`GsdWrites::guarding`] adds the guard pass `phase_complete`.

use serde_json::{Value, json};
use specforge_extension_sdk::DeclaredCategory;
use specforge_extension_sdk::prelude::{ContributionsBuilder, ExtensionMeta};
use specforge_wasm::WasmCallResult;
use specforge_wasm::testing::InProcessRuntime;

/// The extension that declares the kinds and the fields.
pub const CORE: &str = "@t/core";
/// A peer of [`CORE`]: it may write [`CORE`]'s harness fields.
pub const UI: &str = "@t/ui";
/// An extension that names no peer.
pub const OTHER: &str = "@t/other";

/// The three extensions' names, as a project enables them.
pub const GSD_WRITES_EXTENSIONS: [&str; 3] = [CORE, UI, OTHER];

/// The kinds of [`CORE`], as the wire carries them: the history kinds, a human-only `override` and the `commit` a
/// collector creates.
pub fn gsd_writes_wire() -> Value {
    let mut kinds = super::gsd::gsd_history_wire();
    kinds.as_array_mut().expect("an array").extend([
        json!({"name": "override", "keyword": "override", "writer": "human", "fields": [
            {"name": "by", "field_type": "string"},
            {"name": "at", "field_type": "string"},
            {"name": "phase", "field_type": "reference", "target_kind": "phase"},
        ]}),
        // A block list one of whose members is written by a person.
        json!({"name": "reviewed", "keyword": "reviewed", "fields": [
            {"name": "approvals", "field_type": "block_list", "members": [
                {"name": "by", "field_type": "string", "writer": "human"},
                {"name": "note", "field_type": "string"},
            ]},
        ]}),
        json!({"name": "commit", "keyword": "commit", "scoped_by": "phase", "fields": [
            {"name": "phase", "field_type": "reference", "target_kind": "phase"},
            {"name": "sha", "field_type": "string"},
        ]}),
    ]);
    kinds
}

/// The guard pass of [`CORE`], as the wire carries it.
fn guard_wire() -> Value {
    json!({"name": "phase_complete", "phase": "guard", "baseline": ["phase"]})
}

/// The goal [`GsdWrites::guarding_a_goal`]'s guard objects to.
pub const GUARDED: &str = "GUARDED";

fn marked_guard(export: &str, input: &[u8]) -> Option<Result<Vec<u8>, String>> {
    if export != "__pass_phase_complete" {
        return None;
    }
    let quoted = format!("\"{GUARDED}\"");
    let held = input
        .windows(quoted.len())
        .any(|window| window == quoted.as_bytes());
    let diagnostics = if held {
        json!([{"code": "E700", "severity": "Error", "message": "a phase is not complete", "entity": "v1.0/03"}])
    } else {
        json!([])
    };
    let output = json!({"diagnostics": diagnostics, "summary": {}, "values": []});
    Some(Ok(output.to_string().into_bytes()))
}

/// The revision of a file holding `bytes`, as a plan names it: `sha256:` and the digest.
pub fn revision_of(bytes: &[u8]) -> String {
    format!("sha256:{}", specforge_package::Sha256::of(bytes))
}

/// A command answer: the exit code and the plan it holds (`ops` are the plan's operations as the wire carries
/// them). An empty `ops` is a command that answers no plan.
pub fn gsd_writes_answer(exit_code: i32, ops: Value) -> WasmCallResult {
    let mut output = json!({"exit_code": exit_code, "stdout": "", "stderr": ""});
    if ops.as_array().is_some_and(|ops| !ops.is_empty()) {
        output["plan"] = json!({"ops": ops});
    }
    WasmCallResult::Ok(output.to_string().into_bytes())
}

/// The three extensions, served in process.
pub struct GsdWrites {
    guard: bool,
    /// The guard answers by what the project holds ([`GUARDED`]), not one fixed answer.
    marked: bool,
    answers: Vec<(String, String, WasmCallResult)>,
}

impl GsdWrites {
    pub fn new() -> Self {
        GsdWrites {
            guard: false,
            marked: false,
            answers: Vec::new(),
        }
    }

    /// `export` of `extension` answers `result`.
    pub fn answering(mut self, extension: &str, export: &str, result: WasmCallResult) -> Self {
        self.answers
            .push((extension.to_string(), export.to_string(), result));
        self
    }

    /// [`CORE`] declares the guard pass `phase_complete`, which answers `diagnostics` (the wire's pass findings).
    pub fn guarding(mut self, diagnostics: Value) -> Self {
        self.guard = true;
        let output = json!({"diagnostics": diagnostics, "summary": {}, "values": []});
        self.answering(
            CORE,
            "__pass_phase_complete",
            WasmCallResult::Ok(output.to_string().into_bytes()),
        )
    }

    /// [`CORE`] declares the guard pass `phase_complete`, which reports E700 for as long as the project holds a
    /// phase whose `goal` is [`GUARDED`]: a rule about the state a write leaves, which a project can already violate.
    pub fn guarding_a_goal(mut self) -> Self {
        self.guard = true;
        self.marked = true;
        self
    }

    /// The runtime serving the three extensions and the answers given.
    pub fn runtime(self) -> InProcessRuntime {
        let guard = self.guard;
        let mut runtime = InProcessRuntime::new();
        runtime = if self.marked {
            runtime.with_handler(move || core(guard), marked_guard)
        } else {
            runtime.with(move || core(guard))
        };
        let mut runtime = runtime.with(ui).with(other).answer_raw(
            CORE,
            "__pass_verify_status",
            WasmCallResult::Ok(
                json!({"diagnostics": [], "summary": {}, "values": []})
                    .to_string()
                    .into_bytes(),
            ),
        );
        for (extension, export, result) in self.answers {
            runtime = runtime.answer_raw(&extension, &export, result);
        }
        runtime
    }
}

impl Default for GsdWrites {
    fn default() -> Self {
        Self::new()
    }
}

fn command(id: &str, writes: bool) -> Value {
    let mut command = json!({
        "id": id, "title": id, "description": id, "export": format!("cmd__{id}"), "args": [],
    });
    if writes {
        command["writes"] = json!(true);
    }
    command
}

fn collector(name: &str) -> Value {
    json!({"name": name, "input_formats": ["json"], "export": format!("collect__{name}"), "writes": true})
}

fn core(guard: bool) -> ContributionsBuilder {
    let mut c = ContributionsBuilder::new(ExtensionMeta::new(CORE, "1.0.0"));
    c.raw_category(DeclaredCategory::Entities, gsd_writes_wire());
    let mut passes = super::gsd::gsd_history_passes();
    if guard {
        passes.as_array_mut().expect("an array").push(guard_wire());
    }
    c.raw_category(DeclaredCategory::Passes, passes);
    c.raw_category(
        DeclaredCategory::Surfaces,
        json!([{"commands": [command("record", true), command("quiet", false)]}]),
    );
    c.raw_category(DeclaredCategory::Collectors, json!([collector("git")]));
    c
}

fn ui() -> ContributionsBuilder {
    let mut c = ContributionsBuilder::new(ExtensionMeta::new(UI, "1.0.0"));
    c.peer(CORE, "^1.0");
    c.raw_category(
        DeclaredCategory::Surfaces,
        json!([{"commands": [command("mark", true)]}]),
    );
    c.raw_category(DeclaredCategory::Collectors, json!([collector("dom")]));
    c
}

fn other() -> ContributionsBuilder {
    let mut c = ContributionsBuilder::new(ExtensionMeta::new(OTHER, "1.0.0"));
    // A kind with no harness field: a project enabling only this one writes through a command, not a ledger.
    c.raw_category(
        DeclaredCategory::Entities,
        json!([{"name": "memo", "keyword": "memo", "fields": [{"name": "note", "field_type": "string"}]}]),
    );
    c.raw_category(
        DeclaredCategory::Surfaces,
        json!([{"commands": [command("record", true)]}]),
    );
    c
}
