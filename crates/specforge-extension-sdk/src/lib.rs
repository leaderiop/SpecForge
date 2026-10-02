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

/// The extension vocabulary — field types, validation check kinds and
/// constraint kinds — as the host's registry build reads it. `Custom` hands
/// the decision to the rule's `wasm_function` (see
/// [`RuleBuilder::wasm_function`]).
pub use specforge_protocol_types::{CheckKind, ConstraintKind, FieldType};

pub use specforge_protocol_types::{
    ContributionFlags, EdgeTypeDescriptor, EntityEnhancementDescriptor, EntityKindDescriptor,
    FeatureFlagDescriptor, FieldConstraintDescriptor, FieldDescriptor, HandshakeResponse,
    PeerDependency, ProtocolError, SandboxPolicy, ValidationRuleDescriptor, ValidationSeverity,
    ValidatorContext, ValidatorEntity, ValidatorField, ValidatorMethod, ValidatorParam,
    ValidatorRef, ValidatorVerdict,
};

use specforge_protocol_types::{
    AutoDetectConfig, CollectorDescriptor, CompilerPassDescriptor, DescribeRequest,
    DescribeResponse,
};

use std::collections::BTreeMap;

/// Typed access to the host functions (methods available on wasm builds).
pub struct HostApi;

/// Runtime-free testing of an extension's contributions.
pub mod testing;

/// Identity of the extension: what `__handshake` reports.
#[derive(Debug, Clone, Default)]
pub struct ExtensionMeta {
    pub name: String,
    pub version: String,
    pub short: Option<String>,
    pub peer_dependencies: Vec<PeerDependency>,
    pub sandbox_policy: Option<SandboxPolicy>,
    /// The starter `.spec` file `specforge init` writes for a project that
    /// enables this extension; `{project}` stands for the project's id.
    pub starter_template: Option<String>,
    /// The export `specforge migrate` calls after migrating the project's
    /// files.
    pub migration_hook: Option<String>,
    /// The colour diagrams draw the extension in (`#rrggbb`).
    pub theme_color: Option<String>,
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
    collectors: Vec<CollectorDescriptor>,
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

    /// Contribute a test-result collector. Its export is `collect__<name>`
    /// with `-` mapped to `_`; dispatch it to a function taking a
    /// [`CollectInput`] and returning a [`CollectOutput`].
    pub fn collector(&mut self, name: &str, f: impl FnOnce(&mut CollectorBuilder)) -> &mut Self {
        let mut b = CollectorBuilder::new(name);
        f(&mut b);
        self.collectors.push(b.0);
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

    /// Contribute the starter `.spec` file `specforge init` writes for a
    /// project that enables this extension. `{project}` in `template` is
    /// replaced with the project's entity id.
    pub fn starter_template(&mut self, template: &str) -> &mut Self {
        self.meta.starter_template = Some(template.to_string());
        self
    }

    /// The colour diagrams draw this extension in (`#rrggbb`): its cluster
    /// in `specforge model --format dot`, its node in `specforge outline`.
    pub fn theme_color(&mut self, color: &str) -> &mut Self {
        self.meta.theme_color = Some(color.to_string());
        self
    }

    /// Name the export `specforge migrate` calls, after it migrates the
    /// project's `.spec` files, so the extension can migrate its own data.
    /// The extension must export a function by that name.
    pub fn migration_hook(&mut self, export: &str) -> &mut Self {
        self.meta.migration_hook = Some(export.to_string());
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
        f.collectors = has("collectors") || !self.collectors.is_empty();
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
            starter_template: self.meta.starter_template.clone(),
            theme_color: self.meta.theme_color.clone(),
            migration_hook: self.meta.migration_hook.clone(),
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
                "collectors" => serde_json::to_value(&self.collectors).ok()?,
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
    /// request and returns the serialized response, or an error string for
    /// malformed requests and unsupported categories.
    pub fn describe_dispatch(&self, input: &[u8]) -> Result<Vec<u8>, String> {
        let request: DescribeRequest =
            serde_json::from_slice(input).map_err(|e| format!("invalid describe request: {e}"))?;
        match self.describe_response_json(&request.category) {
            Some(body) => Ok(body.into_bytes()),
            None => Err(format!("unsupported category: {}", request.category)),
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
    /// Reference fields that target this kind are contract obligations of
    /// the entity that declares them (the `contracts` analysis, A010).
    pub fn contract_target(&mut self) -> &mut Self {
        self.0.contract_target = true;
        self
    }
    /// The kind's entity ids name types: custom validators receive them as
    /// `ValidatorContext::declared_types`.
    pub fn declares_types(&mut self) -> &mut Self {
        self.0.declares_types = true;
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
    /// The field states what the entity promises (a contract, a
    /// guarantee) rather than prose, so token-optimized exports keep it.
    pub fn normative(&mut self) -> &mut Self {
        self.0.normative = true;
        self
    }
    /// Set on an entity (`true`, or a non-empty value), the entity owes no
    /// obligations of its own (W004, coverage and stats leave it out).
    pub fn exempts_obligations(&mut self) -> &mut Self {
        self.0.exempts_obligations = true;
        self
    }
    /// The context export carries the field at the node's top level.
    pub fn headline(&mut self) -> &mut Self {
        self.0.headline = true;
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
            verify_kinds: None,
        })
    }
    pub fn field(&mut self, name: &str, f: impl FnOnce(&mut FieldBuilder)) -> &mut Self {
        let mut b = FieldBuilder::new(name);
        f(&mut b);
        self.0.fields.push(b.0);
        self
    }
    /// Make the target kind testable, accepting `verify` obligations of these kinds.
    pub fn verify_kinds(&mut self, kinds: &[&str]) -> &mut Self {
        self.0.verify_kinds = Some(kinds.iter().map(|k| k.to_string()).collect());
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
    pub fn kind(&mut self, k: ConstraintKind) -> &mut Self {
        self.0.kind = k.as_str().to_string();
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
    /// The phase the pass runs in. `"check"` runs it with every compile
    /// (`specforge check`, watch, the LSP, MCP), after the graph checks,
    /// its diagnostics joining the compile's; any other phase, or none,
    /// runs it only under `specforge analyze`.
    pub fn phase(&mut self, p: &str) -> &mut Self {
        self.0.phase = Some(p.to_string());
        self
    }
}

/// Builder for [`CollectorDescriptor`].
pub struct CollectorBuilder(CollectorDescriptor);
impl CollectorBuilder {
    fn new(name: &str) -> Self {
        Self(CollectorDescriptor {
            name: name.to_string(),
            input_formats: Vec::new(),
            export: format!("collect__{}", name.replace('-', "_")),
            auto_detect: None,
            run: Vec::new(),
            report: None,
            capture: None,
        })
    }
    /// A report format the collector reads (informational).
    pub fn input_format(&mut self, format: &str) -> &mut Self {
        self.0.input_formats.push(format.to_string());
        self
    }
    /// Project-root files whose presence selects this collector.
    pub fn detect_files(&mut self, patterns: &[&str]) -> &mut Self {
        let detect = self.0.auto_detect.get_or_insert_with(|| AutoDetectConfig {
            file_patterns: Vec::new(),
            env_vars: Vec::new(),
        });
        detect
            .file_patterns
            .extend(patterns.iter().map(|p| p.to_string()));
        self
    }
    /// The command the host runs, with consent. `{report}` in any argument
    /// expands to the absolute report path.
    pub fn run(&mut self, argv: &[&str]) -> &mut Self {
        self.0.run = argv.iter().map(|a| a.to_string()).collect();
        self
    }
    /// Report file or directory the runner writes, relative to the project
    /// root. A directory is read as every `*.json` file directly inside it.
    pub fn report(&mut self, path: &str) -> &mut Self {
        self.0.report = Some(path.to_string());
        self
    }
    /// Keep the command's standard output and pass it to the export as
    /// `CollectInput::stdout`, for runners whose results only appear there.
    pub fn capture_stdout(&mut self) -> &mut Self {
        self.0.capture = Some("stdout".to_string());
        self
    }
}

/// Generated by the `extension` attribute: serializes `__handshake`.
pub fn handshake_json(b: &ContributionsBuilder) -> String {
    b.handshake_json()
}

/// Generated by the `extension` attribute: dispatches `__describe`.
pub fn describe_dispatch(b: &ContributionsBuilder, input: &[u8]) -> Result<Vec<u8>, String> {
    b.describe_dispatch(input)
}

/// Everything an extension needs, behind one `use`.
pub use specforge_extension_sdk_macros::compiler_pass;

pub mod prelude {
    pub use crate::{
        CheckKind, ConstraintKind, Contributions, ContributionsBuilder, EdgeBuilder,
        EnhancementBuilder, ExtensionMeta, FieldBuilder, FieldConstraintBuilder, FieldType,
        KindBuilder, PassBuildCache, PassBuilder, PassCachedStatus, PassDiagnostic, PassEdge,
        PassEntity, PassEntityResults, PassInput, PassOutput, PassSeverity, PassSpan,
        PassTestResult, PassTestResults, RuleBuilder,
    };
    pub use crate::{
        CollectEntityResult, CollectInput, CollectOutput, CollectReportFile, CollectTestResult,
        CollectUnlinkedTest, CollectorBuilder,
    };
    pub use crate::{CommandGraph, CommandInput, CommandOutput, GraphEdge, GraphNode};
    pub use specforge_extension_sdk_macros::{compiler_pass, extension};
    pub use specforge_protocol_types::{
        PeerDependency, SandboxPolicy, ValidationSeverity, ValidatorContext, ValidatorVerdict,
    };
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

    #[test]
    fn a_collector_can_capture_stdout() {
        let mut k = CollectorBuilder::new("cargo-test");
        let plain = serde_json::to_value(&k.0).unwrap();
        assert!(plain.get("capture").is_none());
        k.capture_stdout();
        let captured = serde_json::to_value(&k.0).unwrap();
        assert_eq!(captured["capture"], serde_json::json!("stdout"));
    }

    #[test]
    fn a_starter_template_rides_the_handshake() {
        let mut b = ContributionsBuilder::new(ExtensionMeta::new("@acme/x", "1.0.0"));
        let without: serde_json::Value = serde_json::from_str(&b.handshake_json()).unwrap();
        assert!(without.get("starter_template").is_none(), "{without}");

        b.starter_template("spec \"{project}\" {}\n");
        let with: serde_json::Value = serde_json::from_str(&b.handshake_json()).unwrap();
        assert_eq!(
            with["starter_template"],
            serde_json::json!("spec \"{project}\" {}\n")
        );
    }

    #[test]
    fn a_migration_hook_rides_the_handshake() {
        let mut b = ContributionsBuilder::new(ExtensionMeta::new("@acme/x", "1.0.0"));
        let without: serde_json::Value = serde_json::from_str(&b.handshake_json()).unwrap();
        assert!(without.get("migration_hook").is_none(), "{without}");

        b.migration_hook("migrate_acme");
        let with: serde_json::Value = serde_json::from_str(&b.handshake_json()).unwrap();
        assert_eq!(with["migration_hook"], serde_json::json!("migrate_acme"));
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
#[derive(Debug, Clone, Default, serde::Deserialize)]
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
    /// Whether the entity's kind is testable (its kind registry entry's
    /// `testable` flag), so coverage counts it.
    #[serde(default)]
    pub testable: bool,
    /// One entry per `verify` statement, in order: its kind, or `""` for a
    /// bare `verify "..."`. Empty when the entity declares no obligations.
    #[serde(default)]
    pub verify_kinds: Vec<String>,
    /// The obligations' texts, parallel to `verify_kinds`.
    #[serde(default)]
    pub verify_texts: Vec<String>,
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
#[derive(Debug, Clone, Default, serde::Deserialize)]
pub struct PassInput {
    pub entities: Vec<PassEntity>,
    /// Resolved references between snapshot entities. Serde default keeps
    /// passes written against the entities-only ABI compatible.
    #[serde(default)]
    pub edges: Vec<PassEdge>,
    /// Recorded test results (the normalized `specforge-report.json`), when
    /// the host has them.
    #[serde(default)]
    pub test_results: Option<PassTestResults>,
    /// Entity ids whose formal claims the prove pass entailed; `None` when
    /// the prove pass did not run.
    #[serde(default)]
    pub proved_claims: Option<Vec<String>>,
    /// The build cache (`specforge-cache.json`, written by `specforge check
    /// --cache`): the statuses of the build that wrote it. Check-phase
    /// passes only; `None` without the file (a first build), when it is
    /// invalid (the host warns W144), and for analyze passes.
    #[serde(default)]
    pub previous: Option<PassBuildCache>,
}

/// The previous build's statuses, handed to check-phase passes.
#[derive(Debug, Clone, Default, serde::Deserialize)]
pub struct PassBuildCache {
    /// Per entity id: its kind and status in that build. Entities without
    /// a `status` field are absent.
    #[serde(default)]
    pub statuses: std::collections::BTreeMap<String, PassCachedStatus>,
}

/// One entity's kind and status in the previous build.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct PassCachedStatus {
    pub kind: String,
    pub status: String,
}

/// Normalized test results handed to a pass: per entity id, the recorded tests.
#[derive(Debug, Clone, Default, serde::Deserialize)]
pub struct PassTestResults {
    #[serde(default)]
    pub runner: Option<String>,
    #[serde(default)]
    pub results: std::collections::BTreeMap<String, PassEntityResults>,
}

/// The tests recorded for one entity.
#[derive(Debug, Clone, Default, serde::Deserialize)]
pub struct PassEntityResults {
    #[serde(default)]
    pub tests: Vec<PassTestResult>,
}

/// One recorded test. `status` is `"pass"` for a passing test.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct PassTestResult {
    #[serde(default)]
    pub name: Option<String>,
    pub status: String,
    /// The obligation the test proves, when it names one.
    #[serde(default)]
    pub verify: Option<String>,
}

/// What the host passes to a `collect__<name>` export: the runner's report
/// files, read from the declared report location, and the command's
/// standard output when the collector captures it.
#[derive(Debug, Clone, Default, serde::Deserialize, serde::Serialize)]
pub struct CollectInput {
    #[serde(default)]
    pub reports: Vec<CollectReportFile>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stdout: Option<String>,
}

/// One report file: its path relative to the project root, and its text.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct CollectReportFile {
    pub path: String,
    pub content: String,
}

/// What a `collect__<name>` export returns: test results grouped by the
/// entity each test proves, and the tests the report doesn't link to any
/// entity, which the host links by naming convention when it can.
#[derive(Debug, Clone, Default, serde::Deserialize, serde::Serialize)]
pub struct CollectOutput {
    pub entity_results: Vec<CollectEntityResult>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub unlinked: Vec<CollectUnlinkedTest>,
}

/// A test the report doesn't link to an entity: its name, the name split
/// into its path segments (`["tests", "add_item", "rejects_a_duplicate"]`,
/// the test's own name last), and `passed`, `failed` or `skipped`.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct CollectUnlinkedTest {
    pub name: String,
    pub path: Vec<String>,
    pub status: String,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct CollectEntityResult {
    pub entity_id: String,
    pub test_results: Vec<CollectTestResult>,
}

/// One test. `status` is `passed`, `failed` or `skipped`; `verify` names
/// the obligation the test proves, when the test says so.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct CollectTestResult {
    pub name: String,
    pub status: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verify: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<f64>,
}

// ── Surface command ABI (v1) ───────────────────────────────────────────────
// A CLI command an extension contributes is a `cmd__<name>` export. The host
// parses the command line against the command's declared args, compiles the
// project and calls the export with a [`CommandInput`]: the args, the
// project root, and the compiled graph in the graph export's shape
// (`specforge export --format graph` without the schema). The export answers
// with a [`CommandOutput`]. The same export serves the MCP tool the command
// is auto-promoted to (`specforge.<ext_short>.<id>`), so it never reads the
// file system: the graph is all it knows.

/// What a `cmd__<name>` export receives.
#[derive(Debug, Clone, Default, serde::Deserialize, serde::Serialize)]
pub struct CommandInput {
    /// The declared args the caller set, by name: strings (string, path and
    /// enum args), integers and booleans. An arg the caller left out is
    /// absent unless the host applied its declared default.
    #[serde(default)]
    pub args: serde_json::Map<String, serde_json::Value>,
    /// The project root.
    #[serde(default)]
    pub cwd: String,
    /// The compiled project's graph.
    #[serde(default)]
    pub graph: CommandGraph,
}

impl CommandInput {
    /// A string arg (string, path or enum), when set.
    pub fn arg_str(&self, name: &str) -> Option<&str> {
        self.args.get(name).and_then(|v| v.as_str())
    }

    /// A non-negative integer arg, when set: a JSON number, or a string
    /// holding one.
    pub fn arg_usize(&self, name: &str) -> Option<usize> {
        match self.args.get(name)? {
            serde_json::Value::Number(n) => n.as_u64().map(|n| n as usize),
            serde_json::Value::String(s) => s.parse().ok(),
            _ => None,
        }
    }

    /// A boolean arg; `false` when unset.
    pub fn arg_bool(&self, name: &str) -> bool {
        match self.args.get(name) {
            Some(serde_json::Value::Bool(b)) => *b,
            Some(serde_json::Value::String(s)) => s == "true",
            _ => false,
        }
    }
}

/// The compiled graph a command reads: its entities sorted by id, its
/// resolved references sorted by (source, target, label).
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(from = "GraphWire", into = "GraphWire")]
pub struct CommandGraph {
    nodes: Vec<GraphNode>,
    edges: Vec<GraphEdge>,
    by_id: std::collections::HashMap<String, usize>,
    from: std::collections::HashMap<String, Vec<usize>>,
    to: std::collections::HashMap<String, Vec<usize>>,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct GraphWire {
    #[serde(default)]
    nodes: Vec<GraphNode>,
    #[serde(default)]
    edges: Vec<GraphEdge>,
}

impl From<GraphWire> for CommandGraph {
    fn from(wire: GraphWire) -> Self {
        CommandGraph::new(wire.nodes, wire.edges)
    }
}

impl From<CommandGraph> for GraphWire {
    fn from(graph: CommandGraph) -> Self {
        GraphWire {
            nodes: graph.nodes,
            edges: graph.edges,
        }
    }
}

impl CommandGraph {
    /// A graph of `nodes` and `edges`, indexed for lookups.
    pub fn new(nodes: Vec<GraphNode>, edges: Vec<GraphEdge>) -> Self {
        let mut graph = CommandGraph {
            nodes,
            edges,
            ..Default::default()
        };
        for (i, node) in graph.nodes.iter().enumerate() {
            graph.by_id.insert(node.id.clone(), i);
        }
        for (i, edge) in graph.edges.iter().enumerate() {
            graph.from.entry(edge.source.clone()).or_default().push(i);
            graph.to.entry(edge.target.clone()).or_default().push(i);
        }
        graph
    }

    /// Every entity, sorted by id.
    pub fn nodes(&self) -> &[GraphNode] {
        &self.nodes
    }

    /// Every resolved reference.
    pub fn edges(&self) -> &[GraphEdge] {
        &self.edges
    }

    /// The entity `id`.
    pub fn node(&self, id: &str) -> Option<&GraphNode> {
        self.by_id.get(id).map(|&i| &self.nodes[i])
    }

    /// The entities of `kind`, sorted by id.
    pub fn nodes_of_kind<'a>(&'a self, kind: &'a str) -> impl Iterator<Item = &'a GraphNode> + 'a {
        self.nodes.iter().filter(move |n| n.kind == kind)
    }

    /// The references out of `id`.
    pub fn edges_from(&self, id: &str) -> Vec<&GraphEdge> {
        self.indexed(&self.from, id)
    }

    /// The references into `id`.
    pub fn edges_to(&self, id: &str) -> Vec<&GraphEdge> {
        self.indexed(&self.to, id)
    }

    fn indexed(
        &self,
        index: &std::collections::HashMap<String, Vec<usize>>,
        id: &str,
    ) -> Vec<&GraphEdge> {
        index
            .get(id)
            .map(|is| is.iter().map(|&i| &self.edges[i]).collect())
            .unwrap_or_default()
    }
}

/// One entity of a [`CommandGraph`]: fields as the graph export writes them
/// (text as strings, lists as arrays).
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct GraphNode {
    pub id: String,
    pub kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default)]
    pub fields: std::collections::BTreeMap<String, serde_json::Value>,
}

impl GraphNode {
    /// A text field's value (a string or an identifier); `None` when the
    /// field is absent or holds a list, a number or a block.
    pub fn text(&self, field: &str) -> Option<&str> {
        self.fields.get(field).and_then(|v| v.as_str())
    }

    /// Whether the entity sets `field`, whatever its value.
    pub fn has_field(&self, field: &str) -> bool {
        self.fields.contains_key(field)
    }

    /// A list field's string items, in declaration order; empty when the
    /// field is absent or not a list.
    pub fn list(&self, field: &str) -> Vec<&str> {
        self.fields
            .get(field)
            .and_then(|v| v.as_array())
            .map(|items| items.iter().filter_map(|i| i.as_str()).collect())
            .unwrap_or_default()
    }
}

/// One resolved reference of a [`CommandGraph`]; `label` is the field it
/// was declared in.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GraphEdge {
    pub source: String,
    pub target: String,
    pub label: String,
}

/// What a `cmd__<name>` export returns: the exit code the CLI exits with
/// (nonzero fails the MCP call), and the text for stdout and stderr.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CommandOutput {
    pub exit_code: i32,
    #[serde(default)]
    pub stdout: String,
    #[serde(default)]
    pub stderr: String,
}

impl CommandOutput {
    /// Success, printing `stdout`.
    pub fn ok(stdout: impl Into<String>) -> Self {
        CommandOutput {
            exit_code: 0,
            stdout: stdout.into(),
            stderr: String::new(),
        }
    }

    /// Failure with exit code 1, printing `stderr`.
    pub fn fail(stderr: impl Into<String>) -> Self {
        CommandOutput {
            exit_code: 1,
            stdout: String::new(),
            stderr: stderr.into(),
        }
    }

    /// The wire bytes the export returns.
    pub fn to_bytes(&self) -> Vec<u8> {
        serde_json::to_vec(self).expect("command output serialization cannot fail")
    }
}

#[cfg(test)]
mod command_abi_tests {
    use super::*;

    #[test]
    fn a_command_input_reads_its_args_and_graph() {
        let input: CommandInput = serde_json::from_value(serde_json::json!({
            "args": {"status": "done", "limit": 2, "offset": "1", "all": true},
            "cwd": "/p",
            "graph": {
                "format_version": "1.0",
                "nodes": [
                    {"id": "a", "kind": "k", "title": "A", "file": "x.spec", "line": 1,
                     "fields": {"status": "done", "refs": ["b"]}},
                    {"id": "b", "kind": "k", "file": "x.spec", "line": 2, "fields": {}}
                ],
                "edges": [{"source": "a", "target": "b", "label": "refs"}]
            }
        }))
        .unwrap();
        assert_eq!(input.arg_str("status"), Some("done"));
        assert_eq!(input.arg_usize("limit"), Some(2));
        assert_eq!(input.arg_usize("offset"), Some(1));
        assert!(input.arg_bool("all"));
        assert!(!input.arg_bool("none"));
        let a = input.graph.node("a").unwrap();
        assert_eq!(a.text("status"), Some("done"));
        assert_eq!(a.text("refs"), None);
        assert_eq!(a.list("refs"), ["b"]);
        assert_eq!(input.graph.nodes_of_kind("k").count(), 2);
        assert_eq!(input.graph.edges_from("a")[0].target, "b");
        assert_eq!(input.graph.edges_to("b")[0].source, "a");
        assert!(input.graph.edges_to("a").is_empty());
    }

    #[test]
    fn a_command_output_is_the_wire_shape_the_host_reads() {
        let out: serde_json::Value =
            serde_json::from_slice(&CommandOutput::fail("nope\n").to_bytes()).unwrap();
        assert_eq!(
            out,
            serde_json::json!({"exit_code": 1, "stdout": "", "stderr": "nope\n"})
        );
    }
}

/// A pass result carrying a summary beside its diagnostics. A pass may return
/// either this or a bare `Vec<PassDiagnostic>`; the host merges `summary`
/// into the pass report.
#[derive(Debug, Clone, serde::Serialize)]
pub struct PassOutput {
    pub diagnostics: Vec<PassDiagnostic>,
    pub summary: serde_json::Value,
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
    /// The id of the entity the diagnostic is about. With no span of its
    /// own, the host attaches that entity's.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entity: Option<String>,
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
            entity: None,
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

    /// Name the entity the diagnostic is about (see [`Self::entity`]).
    pub fn with_entity(mut self, id: impl Into<String>) -> Self {
        self.entity = Some(id.into());
        self
    }
}

/// Component-mode export glue (wasm32-wasip2 guests).
///
/// Expands to the `wit_bindgen::generate!` bindings for the
/// `specforge:bridge@0.1.0` world plus a `Guest` implementation that
/// dispatches the host's `call(name, export-name, input)` entry point:
///
/// - `__handshake` / `__describe` are served from the [`ContributionsBuilder`]
///   produced by `$build` (the same JSON wire protocol as always);
/// - every other export name is forwarded to `$handler`, which returns
///   `None` for names it does not implement (the guest then errors, exactly
///   like a missing export).
///
/// ```ignore
/// fn build() -> specforge_extension_sdk::ContributionsBuilder { /* ... */ }
/// fn dispatch(export: &str, input: &[u8]) -> Option<Result<Vec<u8>, String>> {
///     match export {
///         "scan__rust" => Some(serde_json::to_vec(&scan_rust(req)?)),
///         _ => None,
///     }
/// }
/// specforge_extension_sdk::component_guest!(build = build, handler = dispatch);
/// ```
#[macro_export]
macro_rules! component_guest {
    (build = $build:expr, handler = $handler:expr) => {
        ::wit_bindgen::generate!({
            inline: r#"
package specforge:bridge@0.1.0;

world bridge {
    export call: func(name: string, export-name: string, input: list<u8>) ->
        result<list<u8>, string>;
}
"#,
        });

        struct BridgeGuest;

        impl Guest for BridgeGuest {
            fn call(
                _name: String,
                export_name: String,
                input: Vec<u8>,
            ) -> Result<Vec<u8>, String> {
                let build = $build();
                match export_name.as_str() {
                    "__handshake" => Ok(::specforge_extension_sdk::handshake_json(&build)
                        .into_bytes()),
                    "__describe" => build.describe_dispatch(&input),
                    other => match $handler(other, &input) {
                        Some(result) => result,
                        None => Err(format!("unknown export '{other}'")),
                    },
                }
            }
        }

        export!(BridgeGuest);
    };
}
