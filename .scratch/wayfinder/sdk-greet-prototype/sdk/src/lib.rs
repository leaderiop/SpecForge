//! PROTOTYPE — imagined public API of the future `specforge-extension-sdk` crate.
//!
//! This file is a design artifact for wayfinder map #1, ticket #6:
//! it makes the proposed developer experience concrete so people can react to
//! it. Bodies are `unimplemented!()` on purpose — nothing here runs.
//!
//! Decisions this sketch already encodes (map #1):
//! - Target: wasm32-unknown-unknown (ticket #3)
//! - Installs: local path only; lockfile sha256 = reproducibility pin (ticket #5)
//! - Surface: typed descriptor structs + one attribute macro + HostApi +
//!   MockHost; operational exports stay raw extism-pdk in v1 (ticket #2)

// ---------------------------------------------------------------------------
// What the extension author writes (the whole greet extension):
// ---------------------------------------------------------------------------

// use specforge_extension_sdk::prelude::*;
//
// #[specforge::extension(name = "greet", version = "0.1.0", short = "Friendly greetings")]
// struct Greet;
//
// impl Contributions for Greet {
//     fn contribute(mut c: Contributions) {
//         c.kind("greeting", |k| {
//             k.title("Greeting").testable(false)
//              .field("style", |f| f.kind(FieldType::Enum).values(["warm", "formal"]).required());
//         });
//         c.edge("greets", |e| e.from("behavior").to("greeting"));
//         c.rule("greeting_style_known", |r| r
//              .check(Check::FieldValue { field: "style", matches: "^(warm|formal)$" })
//              .severity(Severity::Error)
//              .message("unknown greeting style"));
//     }
// }

// ---------------------------------------------------------------------------
// What the SDK exposes (sketch — this is the review surface):
// ---------------------------------------------------------------------------

/// Protocol version this SDK speaks. The macro embeds it in `__handshake`.
pub const PROTOCOL_VERSION: &str = "1.0.0";

/// Typed mirror of the host's handshake answer (wire-exact with
/// `crates/specforge-wasm/src/protocol/types.rs` — pinned by a golden-JSON
/// round-trip test so drift is a compile/test error, never a runtime surprise).
pub struct Handshake {
    pub extension: String,
    pub version: String,
    pub protocol_version: String,
    pub contribution_flags: ContributionFlags,
}

#[derive(Default)]
pub struct ContributionFlags {
    pub entities: bool,
    pub edges: bool,
    pub fields: bool,
    pub validation_rules: bool,
    // ... 13 categories total; derived by the macro from what `Contributions`
    // actually contributes — authors can never forget to flip a flag.
}

/// Typed wrapper over the three host functions the runtime actually injects
/// (host_query_graph / host_emit_diagnostic / host_read_file).
pub struct HostApi;

impl HostApi {
    pub fn query_graph(&self, q: &str) -> String {
        unimplemented!("prototyped surface — not implemented")
    }
    pub fn emit_diagnostic(&self, code: &str, message: &str) {
        unimplemented!()
    }
    pub fn read_file(&self, rel: &str) -> Option<Vec<u8>> {
        unimplemented!()
    }
}

/// The trait the `#[specforge::extension]` macro asks the author to implement.
pub trait Contributions {
    fn contribute(c: &mut ContributionsBuilder);
}

pub struct ContributionsBuilder;

impl ContributionsBuilder {
    pub fn kind(&mut self, name: &str, f: impl FnOnce(&mut KindBuilder)) -> &mut Self {
        unimplemented!()
    }
    pub fn edge(&mut self, name: &str, f: impl FnOnce(&mut EdgeBuilder)) -> &mut Self {
        unimplemented!()
    }
    pub fn rule(&mut self, name: &str, f: impl FnOnce(&mut RuleBuilder)) -> &mut Self {
        unimplemented!()
    }
}

pub struct KindBuilder;
impl KindBuilder {
    pub fn title(&mut self, t: &str) -> &mut Self { unimplemented!() }
    pub fn testable(&mut self, t: bool) -> &mut Self { unimplemented!() }
    pub fn field(&mut self, name: &str, f: impl FnOnce(&mut FieldBuilder)) -> &mut Self { unimplemented!() }
}

pub struct FieldBuilder;
impl FieldBuilder {
    pub fn kind(&mut self, k: FieldType) -> &mut Self { unimplemented!() }
    pub fn required(&mut self) -> &mut Self { unimplemented!() }
}

pub enum FieldType {
    String,
    Integer,
    Boolean,
    Enum,
    Reference,
    ReferenceList,
}

pub struct EdgeBuilder;
impl EdgeBuilder {
    pub fn from(&mut self, kind: &str) -> &mut Self { unimplemented!() }
    pub fn to(&mut self, kind: &str) -> &mut Self { unimplemented!() }
}

pub struct RuleBuilder;
impl RuleBuilder {
    pub fn check(&mut self, c: Check) -> &mut Self { unimplemented!() }
    pub fn severity(&mut self, s: Severity) -> &mut Self { unimplemented!() }
    pub fn message(&mut self, m: &str) -> &mut Self { unimplemented!() }
}

pub enum Check {
    FieldValue { field: &'static str, matches: &'static str },
}

pub enum Severity {
    Error,
    Warning,
}

/// Test-side helper: run the extension's contributions in-process, no Wasm.
pub mod testing {
    pub struct MockHost;
    impl MockHost {
        pub fn new() -> Self { unimplemented!() }
        pub fn assert_describe(&self, category: &str, expected_json: &str) { unimplemented!() }
    }
}

/// Re-exported so authors write one `use` and nothing else.
pub mod prelude {
    pub use super::{Contributions, ContributionsBuilder, Handshake, HostApi, *};
}
