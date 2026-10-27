use crate::{
    CheckKind, CollectInput, CollectOutput, ConstraintKind, EdgeTypeDescriptor,
    EntityEnhancementDescriptor, EntityKindDescriptor, FieldConstraintDescriptor, FieldDescriptor,
    FieldType, FieldWriter, FileBase, PassAnswer, PassInput, ScanRequest, ScanResponse,
    ValidationRuleDescriptor, ValidationSeverity, ValidatorContext, ValidatorVerdict, Vocabulary,
    operations,
};
use specforge_protocol_types::{
    AnalyzerDescriptor, AutoDetectConfig, CodeDescriptor, CollectorDescriptor,
    CompilerPassDescriptor, GUARD_PHASE,
};

/// Builder for [`EntityKindDescriptor`].
pub struct KindBuilder(pub(crate) EntityKindDescriptor);

impl KindBuilder {
    pub(crate) fn new(name: &str) -> Self {
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
    /// The kind's body carries syntax the extension parses, not the core
    /// grammar: the host does not report the core grammar's parse errors
    /// inside its entities.
    pub fn has_body_parser(&mut self) -> &mut Self {
        self.0.has_body_parser = true;
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
    /// The single `reference` field (with a target kind) naming the entity this kind's entities are scoped by:
    /// a name is unique within its scope, and an entity's id is its scope's path and its name (ADR 0284).
    /// The field may be absent from an entity, which is then in the project scope.
    pub fn scoped_by(&mut self, field: &str) -> &mut Self {
        self.0.scoped_by = Some(field.to_string());
        self
    }
    /// The kind's entities are a log (ADR 0287): one the baseline holds stays, and keeps every authored value.
    pub fn append_only(&mut self) -> &mut Self {
        self.0.append_only = true;
        self
    }
    /// Only a person writes the kind's entities (ADR 0289): create, edit, delete and rename by an extension's
    /// plan, the MCP tools or a non-interactive CLI are refused.
    pub fn human(&mut self) -> &mut Self {
        self.0.writer = Some(FieldWriter::Human.as_str().to_string());
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
pub struct FieldBuilder(pub(crate) FieldDescriptor);

impl FieldBuilder {
    pub(crate) fn new(name: &str) -> Self {
        Self(FieldDescriptor {
            name: name.to_string(),
            ..Default::default()
        })
    }
    /// The field's type. An enum is declared with [`Self::vocabulary`]:
    /// `FieldType::Enum` here panics, so an enum always has its values.
    pub fn field_type(&mut self, t: FieldType) -> &mut Self {
        assert!(
            t != FieldType::Enum,
            "an enum field is declared with FieldBuilder::vocabulary"
        );
        assert!(
            t != FieldType::EnumList,
            "an enum list field is declared with FieldBuilder::enum_list"
        );
        self.0.field_type = t.as_str().to_string();
        self
    }
    pub fn required(&mut self) -> &mut Self {
        self.0.required = true;
        self
    }
    /// A compute pass of this extension gives the field's value (ADR 0287); it is never written in a `.spec`
    /// file. Some pass must list the field in [`PassBuilder::computes`].
    pub fn computed(&mut self) -> &mut Self {
        self.0.writer = Some(FieldWriter::Computed.as_str().to_string());
        self
    }
    /// Only a host write a command or collector plan makes sets the field, recorded in `specforge-harness`
    /// (ADR 0287).
    pub fn harness(&mut self) -> &mut Self {
        self.0.writer = Some(FieldWriter::Harness.as_str().to_string());
        self
    }
    /// Only a person writes the field (ADR 0289): an extension's plan, the MCP tools and a non-interactive CLI
    /// are refused. A `required` field of a kind not human-only is W021: an agent could then create no entity.
    pub fn human(&mut self) -> &mut Self {
        self.0.writer = Some(FieldWriter::Human.as_str().to_string());
        self
    }
    /// A value set at the baseline never changes (ADR 0287): a field unset there may be set once.
    pub fn immutable(&mut self) -> &mut Self {
        self.0.immutable = true;
        self
    }
    /// The baseline list is a prefix of the current one (ADR 0287). A list field only.
    pub fn append_only(&mut self) -> &mut Self {
        self.0.append_only = true;
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
    /// guarantee) rather than prose: the review document quotes it whole,
    /// and with `headline` it is the entity's statement (ADR 0236).
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
    /// What the prove pass reads the field as: `"bound"` (a fact the solver
    /// assumes) or `"claim"` (a statement that must follow from the bounds).
    pub fn proof_role(&mut self, role: &str) -> &mut Self {
        self.0.proof_role = Some(role.to_string());
        self
    }
    /// The host fills this reference field's edges from type names the
    /// entity writes elsewhere: `"type_expressions"` (its field types) or
    /// `"method_signatures"` (its method parameter and return types).
    pub fn derived_from(&mut self, source: &str) -> &mut Self {
        self.0.derived_from = Some(source.to_string());
        self
    }
    pub fn file_reference(&mut self) -> &mut Self {
        self.0.file_reference = true;
        self
    }
    /// Where a [`Self::file_reference`] field's relative paths start (ADR 0285): the spec root by default.
    pub fn file_base(&mut self, base: FileBase) -> &mut Self {
        self.0.file_base = Some(base.as_str().to_string());
        self
    }
    /// The kinds a reference field may name when it names more than one, exclusive with [`Self::target_kind`]
    /// (ADR 0285).
    pub fn target_kinds(&mut self, kinds: &[&str]) -> &mut Self {
        self.0.target_kinds = kinds.iter().map(|k| k.to_string()).collect();
        self
    }
    /// An integer field's least value, inclusive (ADR 0285).
    pub fn minimum(&mut self, n: i64) -> &mut Self {
        self.0.minimum = Some(n);
        self
    }
    /// An integer field's greatest value, inclusive (ADR 0285).
    pub fn maximum(&mut self, n: i64) -> &mut Self {
        self.0.maximum = Some(n);
        self
    }
    /// A member of a `block` or `block_list` field, declared as a field is (ADR 0285).
    pub fn member(&mut self, name: &str, f: impl FnOnce(&mut FieldBuilder)) -> &mut Self {
        let mut b = FieldBuilder::new(name);
        f(&mut b);
        self.0.members.push(b.0);
        self
    }
    /// Declare the field an `enum_list` of `vocabulary`: a list of its values, each once (ADR 0285). A list has
    /// no absent value, so the vocabulary declares none.
    pub fn enum_list(&mut self, vocabulary: &Vocabulary) -> &mut Self {
        assert!(
            vocabulary.absent().is_none(),
            "an enum list has no absent value"
        );
        self.0.field_type = FieldType::EnumList.as_str().to_string();
        self.0.enum_values = vocabulary.values().iter().map(|v| v.to_string()).collect();
        self
    }
    /// Declare the field an `enum` of `vocabulary`: its values are the
    /// field's `enum_values`, in order, and its absent value, if any, the
    /// field's `default_value`. The only way to declare an enum field.
    pub fn vocabulary(&mut self, vocabulary: &Vocabulary) -> &mut Self {
        self.0.field_type = FieldType::Enum.as_str().to_string();
        self.0.enum_values = vocabulary.values().iter().map(|v| v.to_string()).collect();
        self.0.default_value = vocabulary.absent().map(str::to_string);
        self
    }
}

/// Builder for [`EdgeTypeDescriptor`].
pub struct EdgeBuilder(pub(crate) EdgeTypeDescriptor);

impl EdgeBuilder {
    pub(crate) fn new(label: &str) -> Self {
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
pub struct EnhancementBuilder(pub(crate) EntityEnhancementDescriptor);

impl EnhancementBuilder {
    pub(crate) fn new(target_kind: &str) -> Self {
        Self(EntityEnhancementDescriptor {
            target_kind: target_kind.to_string(),
            fields: Vec::new(),
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
}

/// Builder for [`CodeDescriptor`]: one diagnostic code the extension reports.
pub struct CodeBuilder(pub(crate) CodeDescriptor);

impl CodeBuilder {
    pub(crate) fn new(code: &str) -> Self {
        Self(CodeDescriptor {
            code: code.to_string(),
            ..Default::default()
        })
    }
    /// What the code is called (`Requirement not mapped`).
    pub fn title(&mut self, title: &str) -> &mut Self {
        self.0.title = title.to_string();
        self
    }
    /// What the code means and how to fix it.
    pub fn explanation(&mut self, text: &str) -> &mut Self {
        self.0.explanation = text.to_string();
        self
    }
    /// An absolute `http(s)` URL the editor links the code to.
    pub fn docs(&mut self, url: &str) -> &mut Self {
        self.0.docs = Some(url.to_string());
        self
    }
}

/// Builder for [`ValidationRuleDescriptor`], with a `custom` rule's
/// handler.
pub struct RuleBuilder {
    pub(crate) descriptor: ValidationRuleDescriptor,
    pub(crate) validate: Option<operations::Wire>,
}

impl RuleBuilder {
    pub(crate) fn new(code: &str) -> Self {
        Self {
            descriptor: ValidationRuleDescriptor {
                code: code.to_string(),
                severity: ValidationSeverity::Warning,
                message_template: String::new(),
                check: String::new(),
                target_kind: None,
                edge_type: None,
                field: None,
                constraint: None,
                wasm_function: None,
                target_extension: None,
            },
            validate: None,
        }
    }
    /// Decide a `custom` rule: `handler` receives one entity's
    /// [`ValidatorContext`] and answers its [`ValidatorVerdict`].
    pub fn validate(
        &mut self,
        handler: impl Fn(&ValidatorContext) -> ValidatorVerdict + 'static,
    ) -> &mut Self {
        self.validate = Some(operations::wire(
            "validator",
            move |context: &ValidatorContext| Ok::<_, String>(handler(context)),
        ));
        self
    }
    pub fn check(&mut self, kind: CheckKind) -> &mut Self {
        self.descriptor.check = kind.as_str().to_string();
        self
    }
    pub fn target_kind(&mut self, k: &str) -> &mut Self {
        self.descriptor.target_kind = Some(k.to_string());
        self
    }
    pub fn edge_type(&mut self, e: &str) -> &mut Self {
        self.descriptor.edge_type = Some(e.to_string());
        self
    }
    pub fn field(&mut self, f: &str) -> &mut Self {
        self.descriptor.field = Some(f.to_string());
        self
    }
    pub fn severity(&mut self, s: ValidationSeverity) -> &mut Self {
        self.descriptor.severity = s;
        self
    }
    pub fn message_template(&mut self, m: &str) -> &mut Self {
        self.descriptor.message_template = m.to_string();
        self
    }
    pub fn wasm_function(&mut self, f: &str) -> &mut Self {
        self.descriptor.wasm_function = Some(f.to_string());
        self
    }
    /// The extension whose kind or edge type this rule names, when it is
    /// neither this extension nor one of its declared peers (a rule on a
    /// kind of an extension this one works without, so no peer dependency
    /// is declared). While `extension` is not loaded the rule is inert, and
    /// costs the host no W021.
    pub fn target_extension(&mut self, extension: &str) -> &mut Self {
        self.descriptor.target_extension = Some(extension.to_string());
        self
    }
    pub fn constraint(&mut self, f: impl FnOnce(&mut FieldConstraintBuilder)) -> &mut Self {
        let mut b = FieldConstraintBuilder::default();
        f(&mut b);
        self.descriptor.constraint = Some(b.0);
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

/// Builder for [`CompilerPassDescriptor`], with the pass's handler.
pub struct PassBuilder {
    pub(crate) descriptor: CompilerPassDescriptor,
    pub(crate) run: Option<operations::Wire>,
}

impl PassBuilder {
    pub(crate) fn new(name: &str) -> Self {
        Self {
            descriptor: CompilerPassDescriptor {
                name: name.to_string(),
                ..Default::default()
            },
            run: None,
        }
    }
    /// What the pass does: `handler` receives the [`PassInput`] snapshot
    /// and answers its diagnostics, bare (`Vec<PassDiagnostic>`) or with a
    /// summary ([`PassOutput`](crate::PassOutput)).
    pub fn run<R: Into<PassAnswer>>(
        &mut self,
        handler: impl Fn(&PassInput) -> R + 'static,
    ) -> &mut Self {
        self.run = Some(operations::wire("pass", move |input: &PassInput| {
            Ok::<PassAnswer, String>(handler(input).into())
        }));
        self
    }
    pub fn after(&mut self, p: &str) -> &mut Self {
        self.descriptor.after = Some(p.to_string());
        self
    }
    pub fn before(&mut self, p: &str) -> &mut Self {
        self.descriptor.before = Some(p.to_string());
        self
    }
    /// The phase the pass runs in. `"check"` runs it with every compile
    /// (`specforge check`, watch, the LSP, MCP), after the graph checks,
    /// its diagnostics joining the compile's; any other phase, or none,
    /// runs it only under `specforge analyze`.
    pub fn phase(&mut self, p: &str) -> &mut Self {
        self.descriptor.phase = Some(p.to_string());
        self
    }
    /// Run in the `guard` phase (ADR 0289): the pass runs only when a write is judged, receives the project
    /// before the write as `previous`, and its error diagnostics refuse the write (E080). It answers no values.
    pub fn guard(&mut self) -> &mut Self {
        self.phase(GUARD_PHASE)
    }
    /// Run in the `compute` phase (ADR 0287): before the rules, answering [`PassOutput::values`] for the fields
    /// named in [`Self::computes`].
    pub fn compute(&mut self) -> &mut Self {
        self.phase("compute")
    }
    /// The computed fields the pass answers, `kind.field` or `kind.field.member` (compute phase only).
    pub fn computes(&mut self, fields: &[&str]) -> &mut Self {
        self.descriptor.computes = fields.iter().map(|f| f.to_string()).collect();
        self
    }
    /// The kinds whose git-baseline entities the pass receives as `previous` (compute and check phases).
    pub fn baseline(&mut self, kinds: &[&str]) -> &mut Self {
        self.descriptor.baseline = kinds.iter().map(|k| k.to_string()).collect();
        self
    }
}

/// Builder for [`AnalyzerDescriptor`], with its scanner.
pub struct AnalyzerBuilder {
    pub(crate) descriptor: AnalyzerDescriptor,
    pub(crate) scan: Option<operations::Wire>,
}

impl AnalyzerBuilder {
    pub(crate) fn new(language: &str) -> Self {
        Self {
            descriptor: AnalyzerDescriptor {
                language: language.to_string(),
                file_extensions: Vec::new(),
                excluded_dirs: Vec::new(),
                scan_export: format!("scan__{language}"),
                classify_export: format!("classify__{language}"),
                map_export: format!("map__{language}"),
                description: None,
            },
            scan: None,
        }
    }
    /// The scanner: `handler` receives one source file ([`ScanRequest`])
    /// and answers the public items it found ([`ScanResponse`]).
    pub fn scan(&mut self, handler: impl Fn(&ScanRequest) -> ScanResponse + 'static) -> &mut Self {
        self.scan = Some(operations::wire("scan", move |request: &ScanRequest| {
            Ok::<_, String>(handler(request))
        }));
        self
    }
    /// The file extensions it scans, with their dot (`.rs`).
    pub fn file_extensions(&mut self, extensions: &[&str]) -> &mut Self {
        self.descriptor.file_extensions = extensions.iter().map(|e| e.to_string()).collect();
        self
    }
    /// Directories it never scans (`target`, `node_modules`).
    pub fn excluded_dirs(&mut self, dirs: &[&str]) -> &mut Self {
        self.descriptor.excluded_dirs = dirs.iter().map(|d| d.to_string()).collect();
        self
    }
    pub fn scan_export(&mut self, export: &str) -> &mut Self {
        self.descriptor.scan_export = export.to_string();
        self
    }
    pub fn classify_export(&mut self, export: &str) -> &mut Self {
        self.descriptor.classify_export = export.to_string();
        self
    }
    pub fn map_export(&mut self, export: &str) -> &mut Self {
        self.descriptor.map_export = export.to_string();
        self
    }
    pub fn description(&mut self, d: &str) -> &mut Self {
        self.descriptor.description = Some(d.to_string());
        self
    }
}

/// Builder for [`CollectorDescriptor`], with the collector's handler.
pub struct CollectorBuilder {
    pub(crate) descriptor: CollectorDescriptor,
    pub(crate) collect: Option<operations::Wire>,
}

impl CollectorBuilder {
    pub(crate) fn new(name: &str) -> Self {
        Self {
            descriptor: CollectorDescriptor {
                name: name.to_string(),
                input_formats: Vec::new(),
                export: format!("collect__{}", name.replace('-', "_")),
                auto_detect: None,
                run: Vec::new(),
                report: None,
                capture: None,
                writes: false,
            },
            collect: None,
        }
    }
    /// What the collector does: `handler` receives the report files
    /// ([`CollectInput`]) and answers the results ([`CollectOutput`]), or
    /// why it could not read them.
    pub fn collect(
        &mut self,
        handler: impl Fn(&CollectInput) -> Result<CollectOutput, String> + 'static,
    ) -> &mut Self {
        self.collect = Some(operations::wire("collect", handler));
        self
    }
    /// The collector answers an edit plan ([`CollectOutput::plan`]) the host applies after the report, and
    /// receives the project root, the date, the graph and the revisions of its sources (ADR 0289).
    pub fn writes(&mut self) -> &mut Self {
        self.descriptor.writes = true;
        self
    }
    /// A report format the collector reads (informational).
    pub fn input_format(&mut self, format: &str) -> &mut Self {
        self.descriptor.input_formats.push(format.to_string());
        self
    }
    /// Project-root files whose presence selects this collector.
    pub fn detect_files(&mut self, patterns: &[&str]) -> &mut Self {
        let detect = self
            .descriptor
            .auto_detect
            .get_or_insert_with(|| AutoDetectConfig {
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
        self.descriptor.run = argv.iter().map(|a| a.to_string()).collect();
        self
    }
    /// Report file or directory the runner writes, relative to the project
    /// root. A directory is read as every `*.json` file directly inside it.
    pub fn report(&mut self, path: &str) -> &mut Self {
        self.descriptor.report = Some(path.to_string());
        self
    }
    /// Keep the command's standard output and pass it to the export as
    /// `CollectInput::stdout`, for runners whose results only appear there.
    pub fn capture_stdout(&mut self) -> &mut Self {
        self.descriptor.capture = Some("stdout".to_string());
        self
    }
}
