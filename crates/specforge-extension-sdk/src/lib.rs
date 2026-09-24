//! Plugin-side SDK for authoring [SpecForge](https://github.com/leaderiop/SpecForge)
//! extensions against the handshake/describe protocol (v1.0.0).
//!
//! Wire types come from `specforge-protocol-types` — the same definitions the
//! host (`specforge-wasm`) uses — so the protocol cannot drift between the two
//! sides. The `#[specforge_extension_sdk_macros::extension]` attribute turns a
//! struct plus a [`Contributions`] impl into the `__handshake` / `__describe`
//! exports the host loads.
//!
//! v1 decisions (wayfinder map #1): target `wasm32-unknown-unknown`; local-path
//! installs only; lockfile sha256 is a reproducibility pin, not provenance.

pub use specforge_extension_sdk_macros::extension;

pub use specforge_protocol_types::{
    ContributionFlags, EdgeTypeDescriptor, EntityEnhancementDescriptor, EntityKindDescriptor,
    FeatureFlagDescriptor, FieldConstraintDescriptor, FieldDescriptor, HandshakeResponse,
    PeerDependency, ProtocolError, SandboxPolicy, ValidationRuleDescriptor, ValidationSeverity,
};

use specforge_protocol_types::{CompilerPassDescriptor, DescribeRequest, DescribeResponse};

use std::collections::BTreeMap;

/// Typed access to the host functions (methods available on wasm builds).
pub struct HostApi;

#[cfg(target_arch = "wasm32")]
pub mod host;

/// Runtime-free testing of an extension's contributions.
pub mod testing;

/// How a field's value is interpreted by the compiler and emitters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldType {
    String,
    Integer,
    Boolean,
    Enum,
    Date,
    Reference,
    ReferenceList,
}

impl FieldType {
    pub fn as_str(&self) -> &'static str {
        match self {
            FieldType::String => "string",
            FieldType::Integer => "integer",
            FieldType::Boolean => "boolean",
            FieldType::Enum => "enum",
            FieldType::Date => "date",
            FieldType::Reference => "reference",
            FieldType::ReferenceList => "reference_list",
        }
    }
}

/// Declarative check kinds understood by the host's validation engine
/// (`specforge-registry` `validation_engine.rs`). `Custom` dispatches back into
/// the extension via a wasm function — requires a runtime that passes a
/// `WasmValidationRuntime` (not yet wired in production; see audit C10).
#[derive(Debug, Clone)]
pub enum CheckKind {
    NoOutgoingEdges,
    NoIncomingEdges,
    MissingField,
    FieldConstraint,
    Cycle,
    FileExists,
    ConditionalRequired,
    Custom(String),
}

impl CheckKind {
    pub fn as_str(&self) -> &str {
        match self {
            CheckKind::NoOutgoingEdges => "no_outgoing_edges",
            CheckKind::NoIncomingEdges => "no_incoming_edges",
            CheckKind::MissingField => "missing_field",
            CheckKind::FieldConstraint => "field_constraint",
            CheckKind::Cycle => "cycle",
            CheckKind::FileExists => "file_exists",
            CheckKind::ConditionalRequired => "conditional_required",
            CheckKind::Custom(s) => s,
        }
    }
}

/// Identity of the extension: what `__handshake` reports.
#[derive(Debug, Clone, Default)]
pub struct ExtensionMeta {
    pub name: String,
    pub version: String,
    pub short: Option<String>,
    pub peer_dependencies: Vec<PeerDependency>,
    pub sandbox_policy: Option<SandboxPolicy>,
}

impl ExtensionMeta {
    pub fn new(name: &str, version: &str) -> Self {
        Self {
            name: name.to_string(),
            version: version.to_string(),
            ..Default::default()
        }
    }
}

/// Trait implemented by the extension author; the attribute macro wires it
/// into the protocol exports.
pub trait Contributions {
    fn contribute(c: &mut ContributionsBuilder);
}

/// Accumulates everything an extension contributes, then serializes to the
/// wire format on demand. Contribution flags are **derived** from what was
/// actually contributed, so they cannot drift from the content.
#[derive(Default)]
pub struct ContributionsBuilder {
    pub meta: ExtensionMeta,
    entity_kinds: Vec<EntityKindDescriptor>,
    edge_types: Vec<EdgeTypeDescriptor>,
    fields: Vec<FieldDescriptor>,
    shared_fields: Vec<FieldDescriptor>,
    enhancements: Vec<EntityEnhancementDescriptor>,
    validation_rules: Vec<ValidationRuleDescriptor>,
    passes: Vec<CompilerPassDescriptor>,
    feature_flags: Vec<FeatureFlagDescriptor>,
    raw: BTreeMap<String, serde_json::Value>,
}

impl ContributionsBuilder {
    pub fn new(meta: ExtensionMeta) -> Self {
        Self {
            meta,
            ..Default::default()
        }
    }

    /// Contribute a new entity kind.
    pub fn kind(&mut self, name: &str, f: impl FnOnce(&mut KindBuilder)) -> &mut Self {
        let mut b = KindBuilder::new(name);
        f(&mut b);
        self.entity_kinds.push(b.0);
        self
    }

    /// Contribute an edge type (relationship) between entity kinds.
    pub fn edge(&mut self, label: &str, f: impl FnOnce(&mut EdgeBuilder)) -> &mut Self {
        let mut b = EdgeBuilder::new(label);
        f(&mut b);
        self.edge_types.push(b.0);
        self
    }

    /// Contribute a field shared by all entity kinds.
    pub fn shared_field(&mut self, name: &str, f: impl FnOnce(&mut FieldBuilder)) -> &mut Self {
        let mut b = FieldBuilder::new(name);
        f(&mut b);
        self.shared_fields.push(b.0);
        self
    }

    /// Add fields and/or edge types to an entity kind contributed by another
    /// extension.
    pub fn enhance(
        &mut self,
        target_kind: &str,
        source_extension: &str,
        f: impl FnOnce(&mut EnhancementBuilder),
    ) -> &mut Self {
        let mut b = EnhancementBuilder::new(target_kind, source_extension);
        f(&mut b);
        self.enhancements.push(b.0);
        self
    }

    /// Contribute a declarative validation rule.
    pub fn rule(&mut self, code: &str, f: impl FnOnce(&mut RuleBuilder)) -> &mut Self {
        let mut b = RuleBuilder::new(code);
        f(&mut b);
        self.validation_rules.push(b.0);
        self
    }

    /// Contribute a compiler pass.
    pub fn pass(&mut self, name: &str, f: impl FnOnce(&mut PassBuilder)) -> &mut Self {
        let mut b = PassBuilder::new(name);
        f(&mut b);
        self.passes.push(b.0);
        self
    }

    /// Contribute a feature flag.
    pub fn feature_flag(
        &mut self,
        name: &str,
        default_enabled: bool,
        description: &str,
    ) -> &mut Self {
        self.feature_flags.push(FeatureFlagDescriptor {
            name: name.to_string(),
            description: (!description.is_empty()).then(|| description.to_string()),
            default_enabled,
        });
        self
    }

    /// Escape hatch for categories the SDK does not model yet. `items` is the
    /// raw JSON array the host receives for `category`.
    pub fn raw_category(&mut self, category: &str, items: serde_json::Value) -> &mut Self {
        self.raw.insert(category.to_string(), items);
        self
    }

    fn flags(&self) -> ContributionFlags {
        let mut f = ContributionFlags::default();
        let has = |cat: &str| self.raw.get(cat).is_some_and(|v| !v.is_null());
        // Raw categories count toward their flags too — an extension serving
        // `entities` via raw_category contributes entities exactly as much as
        // one that used the typed builders (formal's migration proved this
        // gap: its describes are static data, not builder calls).
        f.entities = has("entities")
            || has("edges")
            || has("fields")
            || has("shared_fields")
            || has("enhancements")
            || !self.entity_kinds.is_empty()
            || !self.edge_types.is_empty()
            || !self.fields.is_empty()
            || !self.shared_fields.is_empty()
            || !self.enhancements.is_empty();
        f.validators = has("validation_rules") || !self.validation_rules.is_empty();
        f.renderers = has("renderers");
        f.prompts = has("prompts");
        f.parsers = has("parsers");
        f.grammars = has("grammars");
        f.body_parsers = has("body_parsers");
        f.analyzers = has("analyzers");
        f.collectors = has("collectors");
        f.providers = has("providers");
        f
    }

    fn handshake_response(&self) -> HandshakeResponse {
        HandshakeResponse {
            protocol_version: specforge_protocol_types::PROTOCOL_VERSION.to_string(),
            name: self.meta.name.clone(),
            version: self.meta.version.clone(),
            contribution_flags: self.flags(),
            peer_dependencies: self.meta.peer_dependencies.clone(),
            sandbox_policy: self.meta.sandbox_policy.clone(),
        }
    }

    /// The `__handshake` wire payload (pretty JSON, matching the format of the
    /// existing builtin extensions).
    pub fn handshake_json(&self) -> String {
        serde_json::to_string_pretty(&self.handshake_response())
            .expect("handshake serialization cannot fail")
    }

    /// The `__describe` wire payload for `category`, or `None` when the
    /// category is not one of the protocol's supported categories.
    pub fn describe_response_json(&self, category: &str) -> Option<String> {
        use specforge_protocol_types::SUPPORTED_CATEGORIES;
        if !SUPPORTED_CATEGORIES.contains(&category) {
            return None;
        }
        let items = if let Some(raw) = self.raw.get(category) {
            raw.clone()
        } else {
            match category {
                "entities" => serde_json::to_value(&self.entity_kinds).ok()?,
                "edges" => serde_json::to_value(&self.edge_types).ok()?,
                "fields" => serde_json::to_value(&self.fields).ok()?,
                "shared_fields" => serde_json::to_value(&self.shared_fields).ok()?,
                "enhancements" => serde_json::to_value(&self.enhancements).ok()?,
                "validation_rules" => serde_json::to_value(&self.validation_rules).ok()?,
                "passes" => serde_json::to_value(&self.passes).ok()?,
                "feature_flags" => serde_json::to_value(&self.feature_flags).ok()?,
                _ => serde_json::Value::Array(vec![]),
            }
        };
        let resp = DescribeResponse {
            category: category.to_string(),
            items,
        };
        Some(serde_json::to_string_pretty(&resp).expect("describe serialization cannot fail"))
    }

    /// Full dispatch for the generated `__describe` export: parses the host's
    /// request, returns the serialized response, and mirrors the host-visible
    /// error behavior of the hand-written extensions (return-code 1 on
    /// unsupported categories).
    pub fn describe_dispatch(
        &self,
        input: &[u8],
    ) -> Result<Vec<u8>, extism_pdk::WithReturnCode<anyhow::Error>> {
        let request: DescribeRequest = serde_json::from_slice(input)?;
        match self.describe_response_json(&request.category) {
            Some(body) => Ok(body.into_bytes()),
            None => Err(extism_pdk::WithReturnCode::new(
                anyhow::anyhow!("unsupported category: {}", request.category),
                1,
            )),
        }
    }
}

/// Builder for [`EntityKindDescriptor`].
pub struct KindBuilder(EntityKindDescriptor);
impl KindBuilder {
    fn new(name: &str) -> Self {
        Self(EntityKindDescriptor {
            name: name.to_string(),
            ..Default::default()
        })
    }
    pub fn description(&mut self, d: &str) -> &mut Self {
        self.0.description = Some(d.to_string());
        self
    }
    pub fn keyword(&mut self, k: &str) -> &mut Self {
        self.0.keyword = Some(k.to_string());
        self
    }
    pub fn testable(&mut self, t: bool) -> &mut Self {
        self.0.testable = t;
        self
    }
    pub fn singleton(&mut self, s: bool) -> &mut Self {
        self.0.singleton = s;
        self
    }
    pub fn supports_verify(&mut self, s: bool) -> &mut Self {
        self.0.supports_verify = s;
        self
    }
    pub fn incremental(&mut self, i: bool) -> &mut Self {
        self.0.incremental = Some(i);
        self
    }
    pub fn open_fields(&mut self, o: bool) -> &mut Self {
        self.0.open_fields = o;
        self
    }
    pub fn semantic_token(&mut self, t: &str) -> &mut Self {
        self.0.semantic_token = Some(t.to_string());
        self
    }
    pub fn lsp_icon(&mut self, i: &str) -> &mut Self {
        self.0.lsp_icon = Some(i.to_string());
        self
    }
    pub fn dot_shape(&mut self, s: &str) -> &mut Self {
        self.0.dot_shape = Some(s.to_string());
        self
    }
    pub fn dot_color(&mut self, c: &str) -> &mut Self {
        self.0.dot_color = Some(c.to_string());
        self
    }
    pub fn dot_fillcolor(&mut self, c: &str) -> &mut Self {
        self.0.dot_fillcolor = Some(c.to_string());
        self
    }
    pub fn verify_kinds(&mut self, kinds: &[&str]) -> &mut Self {
        self.0.verify_kinds = kinds.iter().map(|s| s.to_string()).collect();
        self
    }
    pub fn inference_guide(&mut self, g: &str) -> &mut Self {
        self.0.inference_guide = Some(g.to_string());
        self
    }
    pub fn field(&mut self, name: &str, f: impl FnOnce(&mut FieldBuilder)) -> &mut Self {
        let mut b = FieldBuilder::new(name);
        f(&mut b);
        self.0.fields.push(b.0);
        self
    }
}

/// Builder for [`FieldDescriptor`].
pub struct FieldBuilder(FieldDescriptor);
impl FieldBuilder {
    fn new(name: &str) -> Self {
        Self(FieldDescriptor {
            name: name.to_string(),
            ..Default::default()
        })
    }
    pub fn field_type(&mut self, t: FieldType) -> &mut Self {
        self.0.field_type = t.as_str().to_string();
        self
    }
    pub fn required(&mut self) -> &mut Self {
        self.0.required = true;
        self
    }
    pub fn description(&mut self, d: &str) -> &mut Self {
        self.0.description = Some(d.to_string());
        self
    }
    pub fn edge(&mut self, label: &str) -> &mut Self {
        self.0.edge = Some(label.to_string());
        self
    }
    pub fn target_kind(&mut self, k: &str) -> &mut Self {
        self.0.target_kind = Some(k.to_string());
        self
    }
    pub fn inverse_of(&mut self, k: &str) -> &mut Self {
        self.0.inverse_of = Some(k.to_string());
        self
    }
    pub fn default_value(&mut self, v: &str) -> &mut Self {
        self.0.default_value = Some(v.to_string());
        self
    }
    pub fn file_reference(&mut self) -> &mut Self {
        self.0.file_reference = true;
        self
    }
    pub fn enum_values(&mut self, values: &[&str]) -> &mut Self {
        self.0.enum_values = values.iter().map(|s| s.to_string()).collect();
        self
    }
}

/// Builder for [`EdgeTypeDescriptor`].
pub struct EdgeBuilder(EdgeTypeDescriptor);
impl EdgeBuilder {
    fn new(label: &str) -> Self {
        Self(EdgeTypeDescriptor {
            label: label.to_string(),
            description: None,
            source_kind: None,
            target_kind: None,
            edge_style: None,
            edge_color: None,
            edge_arrowhead: None,
        })
    }
    pub fn description(&mut self, d: &str) -> &mut Self {
        self.0.description = Some(d.to_string());
        self
    }
    pub fn source_kind(&mut self, k: &str) -> &mut Self {
        self.0.source_kind = Some(k.to_string());
        self
    }
    pub fn target_kind(&mut self, k: &str) -> &mut Self {
        self.0.target_kind = Some(k.to_string());
        self
    }
    pub fn edge_style(&mut self, s: &str) -> &mut Self {
        self.0.edge_style = Some(s.to_string());
        self
    }
    pub fn edge_color(&mut self, c: &str) -> &mut Self {
        self.0.edge_color = Some(c.to_string());
        self
    }
    pub fn edge_arrowhead(&mut self, a: &str) -> &mut Self {
        self.0.edge_arrowhead = Some(a.to_string());
        self
    }
}

/// Builder for [`EntityEnhancementDescriptor`].
pub struct EnhancementBuilder(EntityEnhancementDescriptor);
impl EnhancementBuilder {
    fn new(target_kind: &str, source_extension: &str) -> Self {
        Self(EntityEnhancementDescriptor {
            target_kind: target_kind.to_string(),
            source_extension: source_extension.to_string(),
            fields: Vec::new(),
            edge_types: Vec::new(),
        })
    }
    pub fn field(&mut self, name: &str, f: impl FnOnce(&mut FieldBuilder)) -> &mut Self {
        let mut b = FieldBuilder::new(name);
        f(&mut b);
        self.0.fields.push(b.0);
        self
    }
    pub fn edge_type(&mut self, label: &str, f: impl FnOnce(&mut EdgeBuilder)) -> &mut Self {
        let mut b = EdgeBuilder::new(label);
        f(&mut b);
        self.0.edge_types.push(b.0);
        self
    }
}

/// Builder for [`ValidationRuleDescriptor`].
pub struct RuleBuilder(ValidationRuleDescriptor);
impl RuleBuilder {
    fn new(code: &str) -> Self {
        Self(ValidationRuleDescriptor {
            code: code.to_string(),
            severity: ValidationSeverity::Warning,
            message_template: String::new(),
            check: String::new(),
            target_kind: None,
            edge_type: None,
            field: None,
            constraint: None,
            wasm_function: None,
        })
    }
    pub fn check(&mut self, kind: CheckKind) -> &mut Self {
        self.0.check = kind.as_str().to_string();
        self
    }
    pub fn target_kind(&mut self, k: &str) -> &mut Self {
        self.0.target_kind = Some(k.to_string());
        self
    }
    pub fn edge_type(&mut self, e: &str) -> &mut Self {
        self.0.edge_type = Some(e.to_string());
        self
    }
    pub fn field(&mut self, f: &str) -> &mut Self {
        self.0.field = Some(f.to_string());
        self
    }
    pub fn severity(&mut self, s: ValidationSeverity) -> &mut Self {
        self.0.severity = s;
        self
    }
    pub fn message_template(&mut self, m: &str) -> &mut Self {
        self.0.message_template = m.to_string();
        self
    }
    pub fn wasm_function(&mut self, f: &str) -> &mut Self {
        self.0.wasm_function = Some(f.to_string());
        self
    }
    pub fn constraint(&mut self, f: impl FnOnce(&mut FieldConstraintBuilder)) -> &mut Self {
        let mut b = FieldConstraintBuilder::default();
        f(&mut b);
        self.0.constraint = Some(b.0);
        self
    }
}

/// Builder for [`FieldConstraintDescriptor`].
pub struct FieldConstraintBuilder(FieldConstraintDescriptor);
impl Default for FieldConstraintBuilder {
    fn default() -> Self {
        Self(FieldConstraintDescriptor {
            kind: String::new(),
            pattern: None,
            values: Vec::new(),
        })
    }
}
impl FieldConstraintBuilder {
    pub fn kind(&mut self, k: &str) -> &mut Self {
        self.0.kind = k.to_string();
        self
    }
    pub fn pattern(&mut self, p: &str) -> &mut Self {
        self.0.pattern = Some(p.to_string());
        self
    }
    pub fn values(&mut self, values: &[&str]) -> &mut Self {
        self.0.values = values.iter().map(|s| s.to_string()).collect();
        self
    }
}

/// Builder for [`CompilerPassDescriptor`].
pub struct PassBuilder(CompilerPassDescriptor);
impl PassBuilder {
    fn new(name: &str) -> Self {
        Self(CompilerPassDescriptor {
            name: name.to_string(),
            after: None,
            before: None,
            phase: None,
        })
    }
    pub fn after(&mut self, p: &str) -> &mut Self {
        self.0.after = Some(p.to_string());
        self
    }
    pub fn before(&mut self, p: &str) -> &mut Self {
        self.0.before = Some(p.to_string());
        self
    }
    pub fn phase(&mut self, p: &str) -> &mut Self {
        self.0.phase = Some(p.to_string());
        self
    }
}

/// Generated by the `extension` attribute: serializes `__handshake`.
pub fn handshake_json(b: &ContributionsBuilder) -> String {
    b.handshake_json()
}

/// Generated by the `extension` attribute: dispatches `__describe`.
pub fn describe_dispatch(
    b: &ContributionsBuilder,
    input: &[u8],
) -> Result<Vec<u8>, extism_pdk::WithReturnCode<anyhow::Error>> {
    b.describe_dispatch(input)
}

/// Everything an extension needs, behind one `use`.
pub use specforge_extension_sdk_macros::compiler_pass;

pub mod prelude {
    pub use crate::{
        CheckKind, Contributions, ContributionsBuilder, EdgeBuilder, EnhancementBuilder,
        ExtensionMeta, FieldBuilder, FieldConstraintBuilder, FieldType, HostApi, KindBuilder,
        PassBuilder, PassDiagnostic, PassEdge, PassEntity, PassInput, PassSeverity, PassSpan,
        RuleBuilder,
    };
    pub use specforge_extension_sdk_macros::{compiler_pass, extension};

    pub use specforge_protocol_types::{PeerDependency, SandboxPolicy, ValidationSeverity};
}

#[cfg(test)]
mod raw_category_flag_tests {
    use super::*;

    /// Formal's migration exposed this: an extension serving describes via
    /// `raw_category` (static data) must still raise the corresponding
    /// contribution flags, or the host never requests those categories.
    #[test]
    fn raw_categories_raise_their_flags() {
        let mut b = ContributionsBuilder::new(ExtensionMeta::new("@acme/formal", "1.0.0"));
        b.raw_category("entities", serde_json::json!([{ "name": "Property" }]));
        b.raw_category("validation_rules", serde_json::json!([{ "code": "F100" }]));

        let handshake = b.handshake_json();
        let value: serde_json::Value = serde_json::from_str(&handshake).unwrap();
        let flags = &value["contribution_flags"];
        assert_eq!(
            flags["entities"],
            serde_json::json!(true),
            "raw entities must raise entities flag"
        );
        assert_eq!(
            flags["validators"],
            serde_json::json!(true),
            "raw validation_rules must raise validators flag"
        );
        assert_eq!(flags["grammars"], serde_json::json!(false));
    }
}

// ── Compiler pass ABI (v1) ─────────────────────────────────────────────────
// A compiler pass is a `__pass_<name>` wasm export that receives a snapshot
// of the compiled project's entities and returns host Diagnostics. The
// `#[compiler_pass]` attribute (specforge-extension-sdk-macros) wraps a
// plain function with that export; these types are its parameter and return
// vocabulary.

/// One entity in the snapshot handed to a compiler pass. Mirrors the host's
/// `ValidationEntity` (id, kind, stringified fields, edge counts).
#[derive(Debug, Clone, serde::Deserialize)]
pub struct PassEntity {
    pub id: String,
    pub kind: String,
    #[serde(default)]
    pub fields: std::collections::BTreeMap<String, String>,
    #[serde(default)]
    pub incoming_edge_count: usize,
    #[serde(default)]
    pub outgoing_edge_count: usize,
    #[serde(default)]
    pub span: Option<PassSpan>,
    /// Whether the entity's kind supports verify obligations (the host
    /// derives this from its kind registry).
    #[serde(default)]
    pub testable: bool,
}

/// One resolved reference in the snapshot (label = edge label, e.g.
/// "produces", "consumes", "BehaviorRequiresInvariant").
#[derive(Debug, Clone, serde::Deserialize)]
pub struct PassEdge {
    pub source: String,
    pub target: String,
    pub label: String,
}

/// The `__pass_<name>` export input.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct PassInput {
    pub entities: Vec<PassEntity>,
    /// Resolved references between snapshot entities. Serde default keeps
    /// passes written against the entities-only ABI compatible.
    #[serde(default)]
    pub edges: Vec<PassEdge>,
}

/// Severity mirror of the host diagnostic enum. Serializes to the same wire
/// strings ("Error" / "Warning" / "Info").
#[derive(Debug, Clone, Copy, serde::Serialize)]
pub enum PassSeverity {
    Error,
    Warning,
    Info,
}

/// Source location attached to a pass diagnostic. Field names mirror the
/// host's `SourceSpan`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PassSpan {
    pub file: String,
    pub start_line: usize,
    pub start_col: usize,
    pub end_line: usize,
    pub end_col: usize,
}

/// A diagnostic returned by a compiler pass. Serializes into the host's
/// `Diagnostic` wire shape.
#[derive(Debug, Clone, serde::Serialize)]
pub struct PassDiagnostic {
    pub code: String,
    pub severity: PassSeverity,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub span: Option<PassSpan>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggestion: Option<String>,
}

impl PassDiagnostic {
    /// A diagnostic with a code, severity, and message; attach a span or
    /// suggestion with [`Self::with_span`] / [`Self::with_suggestion`].
    pub fn new(
        code: impl Into<String>,
        severity: PassSeverity,
        message: impl Into<String>,
    ) -> Self {
        Self {
            code: code.into(),
            severity,
            message: message.into(),
            span: None,
            suggestion: None,
        }
    }

    /// Convenience constructor for warnings (the common pass finding).
    pub fn warning(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(code, PassSeverity::Warning, message)
    }

    pub fn with_span(mut self, span: PassSpan) -> Self {
        self.span = Some(span);
        self
    }

    pub fn with_suggestion(mut self, suggestion: impl Into<String>) -> Self {
        self.suggestion = Some(suggestion.into());
        self
    }
}
