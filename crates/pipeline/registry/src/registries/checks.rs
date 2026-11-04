//! Which checks each declared field's values pass (ADR 0336): every `checks` entry resolved to a loaded checker, and
//! every checker or check the host cannot run refused and dropped, so the rest is kept. W019 is about a name that
//! leads nowhere, W189 about a shape the host cannot run.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use serde_json::{Map, Value};
use specforge_common::{ChecksConfig, Diagnostic, SourceSpan, codes};
use specforge_diagnostics::{Codebook, letter_level};
use specforge_graph::FieldForm;
use specforge_protocol_types::{
    CheckRef, CheckerDescriptor, ExtensionDeclaration, FieldDescriptor, FieldType, FieldWriter,
    SettingsItem, ValueForm,
};

use super::{FieldRegistry, FieldRegistryEntry, KindRegistry};
use crate::compilation::{CandidateVocabulary, UnloadedPeers};
use crate::surface::prompts::snake_case;

/// The field types a checker may read.
const VALUE_TYPES: [FieldType; 5] = [
    FieldType::String,
    FieldType::StringList,
    FieldType::Identifier,
    FieldType::Date,
    FieldType::Integer,
];

/// One checker with the arguments a field gave it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckCall {
    pub extension: String,
    pub checker: String,
    pub args: Map<String, Value>,
}

impl CheckCall {
    /// The checker it calls.
    pub fn qualified(&self) -> QualifiedChecker {
        QualifiedChecker {
            extension: self.extension.clone(),
            checker: self.checker.clone(),
        }
    }
}

/// A checker by its extension and its name, written `<extension>:<checker>`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QualifiedChecker {
    pub extension: String,
    pub checker: String,
}

impl std::fmt::Display for QualifiedChecker {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.extension, self.checker)
    }
}

/// A check a field's values pass, resolved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolvedCheck {
    One(CheckCall),
    /// A value passes when one branch yields no error-level finding.
    AnyOf(Vec<CheckCall>),
}

impl ResolvedCheck {
    /// The checkers it runs, in order.
    pub fn calls(&self) -> &[CheckCall] {
        match self {
            ResolvedCheck::One(call) => std::slice::from_ref(call),
            ResolvedCheck::AnyOf(calls) => calls,
        }
    }
}

/// Every checker the registry kept, and every field's checks.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct CheckRegistry {
    checkers: HashMap<(String, String), CheckerDescriptor>,
    /// Kind, then the declared path (`requires.phase`), then its checks in order.
    fields: BTreeMap<String, BTreeMap<String, Vec<ResolvedCheck>>>,
    /// What a kept checker's arguments read as, each with whether it is required.
    forms: HashMap<(String, String), BTreeMap<String, (FieldForm, bool)>>,
    /// A checker refused (W189 already said so): a check naming it is dropped without a second word.
    refused: BTreeSet<(String, String)>,
}

impl CheckRegistry {
    /// The checks on `path` of `kind`, in order.
    pub fn on(&self, kind: &str, path: &str) -> &[ResolvedCheck] {
        self.fields
            .get(kind)
            .and_then(|paths| paths.get(path))
            .map_or(&[], Vec::as_slice)
    }

    /// The checked paths of `kind`, with their checks.
    pub fn of_kind(&self, kind: &str) -> impl Iterator<Item = (&str, &[ResolvedCheck])> {
        self.fields
            .get(kind)
            .into_iter()
            .flatten()
            .map(|(path, checks)| (path.as_str(), checks.as_slice()))
    }

    /// The checker `name` of `extension`, as the registry kept it.
    pub fn checker(&self, extension: &str, name: &str) -> Option<&CheckerDescriptor> {
        self.checkers
            .get(&(extension.to_string(), name.to_string()))
    }

    pub fn is_empty(&self) -> bool {
        self.fields.is_empty()
    }
}

struct Resolving<'a> {
    declarations: &'a [ExtensionDeclaration],
    loaded: Vec<&'a str>,
    candidates: &'a CandidateVocabulary,
    registry: CheckRegistry,
    diagnostics: Vec<Diagnostic>,
}

impl<'a> Resolving<'a> {
    fn new(
        declarations: &'a [ExtensionDeclaration],
        candidates: &'a CandidateVocabulary,
        registry: CheckRegistry,
    ) -> Self {
        Resolving {
            declarations,
            loaded: declarations
                .iter()
                .map(ExtensionDeclaration::name)
                .collect(),
            candidates,
            registry,
            diagnostics: Vec::new(),
        }
    }
}

/// The check registry of `declarations` (load order), and its W019 and W189.
pub(crate) fn build(
    declarations: &[ExtensionDeclaration],
    fields: &FieldRegistry,
    kinds: &KindRegistry,
    book: &Codebook,
    candidates: &CandidateVocabulary,
) -> (CheckRegistry, Vec<Diagnostic>) {
    let mut resolving = Resolving::new(declarations, candidates, CheckRegistry::default());
    for declaration in declarations {
        for checker in &declaration.checkers {
            resolving.keep_checker(declaration, checker, book);
        }
    }
    let mut entries: Vec<(&str, &str, &FieldRegistryEntry)> = fields.iter().collect();
    entries.sort_by_key(|(kind, field, _)| (*kind, *field));
    for (kind, _, entry) in entries {
        let collected = kinds.get(kind).is_some_and(|k| k.is_collected());
        let name = entry.name().to_string();
        entry.declared().for_each_path(&name, &mut |path, field| {
            resolving.field(kind, &path, field, entry.source_extension(), collected);
        });
    }
    (resolving.registry, resolving.diagnostics)
}

/// `registry` with what `config` attaches added after each field's declared checks, and the W019 and W189 of each key,
/// spanned at the key by `at` (the path of a member of `specforge.json`).
pub(crate) fn attach(
    registry: CheckRegistry,
    config: &ChecksConfig,
    declarations: &[ExtensionDeclaration],
    fields: &FieldRegistry,
    kinds: &KindRegistry,
    candidates: &CandidateVocabulary,
    at: &dyn Fn(&[&str]) -> Option<SourceSpan>,
) -> (CheckRegistry, Vec<Diagnostic>) {
    let mut resolving = Resolving::new(declarations, candidates, registry);
    for (key, checks) in &config.fields {
        let from = resolving.diagnostics.len();
        resolving.attach(key, checks, fields, kinds);
        if let Some(span) = at(&["checks", "fields", key]) {
            for diagnostic in &mut resolving.diagnostics[from..] {
                diagnostic.span = Some(span.clone());
            }
        }
    }
    (resolving.registry, resolving.diagnostics)
}

impl Resolving<'_> {
    fn w019(&mut self, message: String) {
        self.diagnostics.push(Diagnostic::new(codes::W019, message));
    }

    fn w189(&mut self, message: String) {
        self.diagnostics.push(Diagnostic::new(codes::W189, message));
    }

    fn keep_checker(
        &mut self,
        declaration: &ExtensionDeclaration,
        checker: &CheckerDescriptor,
        book: &Codebook,
    ) {
        let extension = declaration.name();
        let key = (extension.to_string(), checker.name.clone());
        let problem = if self.registry.checkers.contains_key(&key) {
            Some("is declared twice".to_string())
        } else {
            checker_problem(declaration, checker, book)
        };
        let forms = problem.is_none().then(|| arg_forms(extension, checker));
        match (problem, forms) {
            (None, Some(Ok(forms))) => {
                self.registry.checkers.insert(key.clone(), checker.clone());
                self.registry.forms.insert(key, forms);
            }
            (Some(why), _) | (None, Some(Err(why))) => {
                self.w189(format!(
                    "extension '{extension}': checker '{}' {why}: the checker is dropped",
                    checker.name
                ));
                // A later declaration of the same name is the one refused; the first stands.
                if !self.registry.checkers.contains_key(&key) {
                    self.registry.refused.insert(key);
                }
            }
            (None, None) => {}
        }
    }

    /// The checks `field` declares, on `kind` at `path`, kept or refused.
    fn field(
        &mut self,
        kind: &str,
        path: &str,
        field: &FieldDescriptor,
        owner: &str,
        collected: bool,
    ) {
        let on = format!("field '{path}' on kind '{kind}'");
        let ty = FieldType::parse(&field.field_type);
        // A value read as a path has to be text; a `file_reference` rule reads its value the same way.
        if field.file_reference && !matches!(ty, Some(FieldType::String | FieldType::StringList)) {
            self.w189(format!(
                "{on} declares a file check, but its type is '{}': a path is text",
                field.field_type
            ));
        }
        if field.checks.is_empty() {
            return;
        }
        if let Some(why) = not_authored(field, collected) {
            self.w189(format!(
                "{on} declares checks, but it {why}: its values are not authored, so the checks are dropped"
            ));
            return;
        }
        let kept: Vec<ResolvedCheck> = field
            .checks
            .iter()
            .filter_map(|check| self.resolve(check, Some(owner), &on, ty))
            .collect();
        if !kept.is_empty() {
            self.registry
                .fields
                .entry(kind.to_string())
                .or_default()
                .insert(path.to_string(), kept);
        }
    }

    /// What `specforge.json` writes for `key` (`kind.field` or a member path), after the field's declared checks.
    fn attach(
        &mut self,
        key: &str,
        checks: &[CheckRef],
        fields: &FieldRegistry,
        kinds: &KindRegistry,
    ) {
        let found = key
            .split_once('.')
            .and_then(|(kind, path)| Some((kind, path, fields.get_path(kind, path)?)));
        let Some((kind, path, (field, _))) = found else {
            self.w019(format!(
                "`checks.fields` names '{key}', which is no `kind.field` of a loaded extension"
            ));
            return;
        };
        let on = format!("`checks.fields` key '{key}'");
        let collected = kinds.get(kind).is_some_and(|k| k.is_collected());
        if let Some(why) = not_authored(field, collected) {
            self.w189(format!(
                "{on} attaches checks to a field that {why}: its values are not authored, so the checks are dropped"
            ));
            return;
        }
        let ty = FieldType::parse(&field.field_type);
        let mut kept = Vec::new();
        for check in checks {
            let check = self.qualified(check, &on);
            kept.extend(check.and_then(|check| self.resolve(&check, None, &on, ty)));
        }
        if !kept.is_empty() {
            self.registry
                .fields
                .entry(kind.to_string())
                .or_default()
                .entry(path.to_string())
                .or_default()
                .extend(kept);
        }
    }

    /// `check` with each short name replaced by the one checker it names; W019 when it names none or several.
    fn qualified(&mut self, check: &CheckRef, on: &str) -> Option<CheckRef> {
        match check {
            CheckRef::One { checker, args } if !checker.contains(':') => {
                let mut owners: Vec<&str> = self
                    .registry
                    .checkers
                    .keys()
                    .filter(|(_, name)| name == checker)
                    .map(|(extension, _)| extension.as_str())
                    .collect();
                owners.sort_unstable();
                match owners.as_slice() {
                    [owner] => Some(CheckRef::One {
                        checker: format!("{owner}:{checker}"),
                        args: args.clone(),
                    }),
                    [] => {
                        self.w019(format!(
                            "{on} names the checker '{checker}', which no loaded extension declares"
                        ));
                        None
                    }
                    several => {
                        let message = format!(
                            "{on} names the checker '{checker}', which several extensions declare: write one of {}",
                            several
                                .iter()
                                .map(|owner| format!("{owner}:{checker}"))
                                .collect::<Vec<_>>()
                                .join(", ")
                        );
                        self.w019(message);
                        None
                    }
                }
            }
            CheckRef::One { .. } => Some(check.clone()),
            CheckRef::AnyOf { any_of } => any_of
                .iter()
                .map(|branch| self.qualified(branch, on))
                .collect::<Option<Vec<_>>>()
                .map(|any_of| CheckRef::AnyOf { any_of }),
        }
    }

    /// `check` as `owner` (none: `specforge.json`) wrote it on a field of type `ty`; none when it is refused or inert.
    fn resolve(
        &mut self,
        check: &CheckRef,
        owner: Option<&str>,
        on: &str,
        ty: Option<FieldType>,
    ) -> Option<ResolvedCheck> {
        match check {
            CheckRef::One { checker, args } => self
                .call(checker, args, owner, on, ty)
                .map(ResolvedCheck::One),
            CheckRef::AnyOf { any_of } => {
                if any_of.len() < 2 {
                    self.w189(format!(
                        "{on} declares an any_of with fewer than two branches: the check is dropped"
                    ));
                    return None;
                }
                if any_of.iter().any(|b| matches!(b, CheckRef::AnyOf { .. })) {
                    self.w189(format!(
                        "{on} nests an any_of: one level only, so the check is dropped"
                    ));
                    return None;
                }
                let calls: Vec<Option<CheckCall>> = any_of
                    .iter()
                    .map(|branch| match branch {
                        CheckRef::One { checker, args } => self.call(checker, args, owner, on, ty),
                        CheckRef::AnyOf { .. } => None,
                    })
                    .collect();
                // A branch that cannot run could have been the one that passes: the whole check is out.
                calls
                    .into_iter()
                    .collect::<Option<Vec<_>>>()
                    .map(ResolvedCheck::AnyOf)
            }
        }
    }

    /// `written` resolved to a checker this project can run for `owner`, its args admitted.
    fn call(
        &mut self,
        written: &str,
        args: &Map<String, Value>,
        owner: Option<&str>,
        on: &str,
        ty: Option<FieldType>,
    ) -> Option<CheckCall> {
        let (extension, checker) = match (written.rsplit_once(':'), owner) {
            (Some((extension, checker)), _) => (extension.to_string(), checker.to_string()),
            (None, Some(owner)) => (owner.to_string(), written.to_string()),
            // `qualified` gave every name of `specforge.json` its extension.
            (None, None) => return None,
        };
        let declaring = owner.and_then(|o| self.declarations.iter().find(|d| d.name() == o));
        if !self.loaded.contains(&extension.as_str()) {
            let inert = declaring.is_some_and(|d| {
                UnloadedPeers::of(d, &self.loaded, self.candidates).names(&extension)
            });
            if !inert {
                self.w019(format!(
                    "{on} names the checker '{written}', but no loaded extension is '{extension}'"
                ));
            }
            return None;
        }
        let key = (extension.clone(), checker.clone());
        if self.registry.refused.contains(&key) {
            return None;
        }
        let Some(descriptor) = self.registry.checkers.get(&key).cloned() else {
            self.w019(format!(
                "{on} names the checker '{written}', but '{extension}' declares no checker '{checker}'"
            ));
            return None;
        };
        if let Some(declaring) = declaring
            && declaring.name() != extension
            && !declaring.peers().iter().any(|p| p.name == extension)
        {
            self.w019(format!(
                "{on} names the checker '{written}', but '{}' declares no peer '{extension}': add it to \
                 the extension's peers",
                declaring.name()
            ));
            return None;
        }
        let applies = ty.is_some_and(|ty| descriptor.applies_to.iter().any(|t| t == ty.as_str()));
        if !applies {
            self.w189(format!(
                "{on} names the checker '{written}', which reads {}: the check is dropped",
                descriptor.applies_to.join(", ")
            ));
            return None;
        }
        if let Err(why) = admit_args(args, &self.registry.forms[&key]) {
            self.w189(format!(
                "{on} gives the checker '{written}' {why}: the check is dropped"
            ));
            return None;
        }
        Some(CheckCall {
            extension,
            checker,
            args: args.clone(),
        })
    }
}

/// Why `field`'s values are not authored, so no check runs on them.
fn not_authored(field: &FieldDescriptor, collected: bool) -> Option<String> {
    if collected {
        return Some("is a field of a kind a collector writes".to_string());
    }
    let writer = field.writer.as_deref()?;
    [FieldWriter::Computed, FieldWriter::Verdict]
        .iter()
        .any(|w| writer == w.as_str())
        .then(|| format!("is a {writer} field"))
}

/// What the registry refuses in `checker`'s own declaration.
fn checker_problem(
    declaration: &ExtensionDeclaration,
    checker: &CheckerDescriptor,
    book: &Codebook,
) -> Option<String> {
    if !snake_case(&checker.name) {
        return Some("is not named in snake_case".to_string());
    }
    if let Some(ty) = checker
        .applies_to
        .iter()
        .find(|ty| !FieldType::parse(ty).is_some_and(|ty| VALUE_TYPES.contains(&ty)))
    {
        return Some(format!(
            "applies to '{ty}', which holds no value a checker reads"
        ));
    }
    if checker.args.iter().any(|arg| arg.name == "severity") {
        return Some(
            "takes an argument named 'severity': severity is a code, so give an enum argument \
             whose values select one code each"
                .to_string(),
        );
    }
    if let Some(scope) = checker.scopes.iter().find(|scope| {
        !declaration
            .capabilities
            .iter()
            .any(|capability| capability.name == **scope)
    }) {
        return Some(format!(
            "uses the scope '{scope}', which the extension declares no capability for"
        ));
    }
    if let Some(setting) = checker.when.iter().find(|name| {
        !declaration.settings.iter().any(|item| {
            matches!(item, SettingsItem::Setting(setting)
                if setting.name == **name && setting.field_type == "bool")
        })
    }) {
        return Some(format!(
            "runs when '{setting}', which is no bool setting of the extension"
        ));
    }
    let extension = declaration.name();
    if let Some(code) = checker.codes.iter().find(|code| {
        !letter_level(code).is_some_and(|level| book.check(extension, code, level).is_ok())
    }) {
        return Some(format!(
            "lists the code '{code}', which its catalogue lacks"
        ));
    }
    if checker.value == ValueForm::Unknown {
        return Some("declares a value form other than text, url or path".to_string());
    }
    if let Some(root_arg) = &checker.root_arg {
        if checker.value != ValueForm::Path {
            return Some(format!(
                "names the root_arg '{root_arg}', but its values are not paths"
            ));
        }
        if !checker.args.iter().any(|arg| arg.name == *root_arg) {
            return Some(format!(
                "names the root_arg '{root_arg}', which is none of its arguments"
            ));
        }
    }
    None
}

/// How `checker`'s arguments read, each with whether it is required; `Err` for one the host cannot read.
fn arg_forms(
    extension: &str,
    checker: &CheckerDescriptor,
) -> Result<BTreeMap<String, (FieldForm, bool)>, String> {
    let mut forms = BTreeMap::new();
    for arg in &checker.args {
        let entry = FieldRegistryEntry::new("(checker)", extension, arg.clone()).map_err(|_| {
            format!(
                "takes the argument '{}' of a type the host does not read",
                arg.name
            )
        })?;
        if !entry.problems().is_empty()
            || matches!(
                entry.field_type(),
                FieldType::Reference | FieldType::ReferenceList | FieldType::Identifier
            )
        {
            return Err(format!(
                "takes the argument '{}' in a form a setting cannot have",
                arg.name
            ));
        }
        forms.insert(arg.name.clone(), (entry.form().clone(), arg.required));
    }
    Ok(forms)
}

/// `Err` for an argument the schema does not name or admit, or one it requires that is absent.
fn admit_args(
    args: &Map<String, Value>,
    forms: &BTreeMap<String, (FieldForm, bool)>,
) -> Result<(), String> {
    for (name, value) in args {
        let Some((form, _)) = forms.get(name) else {
            return Err(format!("the argument '{name}', which it does not take"));
        };
        if form.admit_json(value).is_err() {
            return Err(format!("a value of '{name}' its type does not admit"));
        }
    }
    match forms
        .iter()
        .find(|(name, (_, required))| *required && !args.contains_key(*name))
    {
        Some((name, _)) => Err(format!("no '{name}', which it requires")),
        None => Ok(()),
    }
}
