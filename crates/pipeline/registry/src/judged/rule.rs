//! A judged rule as the registry holds it: the declaration after its shape was checked (`shape`) and its selectors
//! resolved against the loaded kinds and fields (`resolve`). A rule that cannot work is an `Err` with the reason, and
//! W112 is the caller's.

use std::collections::BTreeSet;

use specforge_common::Severity;
use specforge_protocol_types::{FieldType, FieldWriter, JudgedDescriptor};

use super::selector::{Chain, Head, Selector, Step};
use crate::compilation::UnloadedPeers;
use crate::rules::Registries;

/// The name a verdict's cited files and entities digest under: an input cannot take it.
pub const CITED: &str = "cited";
/// The input a verdict's `cited_tests` are digested as, beside `cited`.
pub const CITED_TESTS: &str = "cited_tests";

/// One target kind of a judged rule.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JudgedTargetRule {
    pub kind: String,
    /// Named inputs, in order.
    pub inputs: Vec<(String, Selector)>,
    pub unless_set: Vec<String>,
}

/// A registered judged rule (ADR 0290).
#[derive(Debug, Clone)]
pub struct JudgedRule {
    /// The qualified code: the verdict's name.
    pub code: String,
    pub extension: String,
    pub judge: String,
    pub outcomes: Vec<String>,
    pub passing: Vec<String>,
    pub targets: Vec<JudgedTargetRule>,
    /// `kind.field`s set from the verdict's `values`.
    pub values: Vec<String>,
    /// `kind.field`s that hold the verdict's outcome.
    pub outcome_fields: Vec<String>,
    pub severity: Severity,
    pub template: String,
}

impl JudgedRule {
    /// The target of `kind`, when the rule judges it.
    pub fn target(&self, kind: &str) -> Option<&JudgedTargetRule> {
        self.targets.iter().find(|t| t.kind == kind)
    }

    /// Whether `outcome` passes.
    pub fn passes(&self, outcome: &str) -> bool {
        self.passing.iter().any(|p| p == outcome)
    }
}

/// The prefix of a judge that is a prompt of the rule's own extension (ADR 0295).
pub const PROMPT_JUDGE: &str = "prompt:";

/// A judge: an agent id (a word) or `prompt:<id>`, `<id>` a word.
fn is_judge(text: &str) -> bool {
    is_word(text.strip_prefix(PROMPT_JUDGE).unwrap_or(text))
}

pub(crate) fn is_word(text: &str) -> bool {
    !text.is_empty()
        && text
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

fn unique<'a>(items: impl IntoIterator<Item = &'a String>) -> bool {
    let mut seen = BTreeSet::new();
    items.into_iter().all(|item| seen.insert(item))
}

/// The rule `descriptor` declares, without its code, severity and template: its shape checked, its selectors parsed.
pub(crate) fn shape(
    descriptor: &JudgedDescriptor,
) -> Result<(Vec<JudgedTargetRule>, JudgedDescriptor), String> {
    let d = descriptor;
    if !is_judge(&d.judge) {
        return Err(format!(
            "judge '{}' is neither an agent id (a word) nor prompt:<id>",
            d.judge
        ));
    }
    if d.outcomes.is_empty() || !d.outcomes.iter().all(|o| is_word(o)) || !unique(&d.outcomes) {
        return Err("outcomes must be words, each once, and at least one".into());
    }
    if d.passing.is_empty() || !d.passing.iter().all(|p| d.outcomes.contains(p)) {
        return Err("passing must be a non-empty subset of outcomes".into());
    }
    if d.targets.is_empty() || !unique(d.targets.iter().map(|t| &t.kind)) {
        return Err("targets must name each kind once, and at least one".into());
    }
    let mut targets = Vec::new();
    for target in &d.targets {
        if !unique(target.inputs.iter().map(|i| &i.name))
            || target
                .inputs
                .iter()
                .any(|i| !is_word(&i.name) || i.name == CITED || i.name == CITED_TESTS)
        {
            return Err(format!(
                "target '{}': input names must be words, each once, and not '{CITED}' or '{CITED_TESTS}'",
                target.kind
            ));
        }
        let mut inputs = Vec::new();
        for input in &target.inputs {
            let selector = Selector::parse(&input.selector).map_err(|why| {
                format!(
                    "target '{}': input '{}': selector '{}': {why}",
                    target.kind, input.name, input.selector
                )
            })?;
            check_paths(&selector).map_err(|why| {
                format!(
                    "target '{}': input '{}': selector '{}': {why}",
                    target.kind, input.name, input.selector
                )
            })?;
            inputs.push((input.name.clone(), selector));
        }
        targets.push(JudgedTargetRule {
            kind: target.kind.clone(),
            inputs,
            unless_set: target.unless_set.clone(),
        });
    }
    Ok((targets, d.clone()))
}

/// The paths of a `file("…")` selector: from the project root, inside it, no glob, no spec source.
fn check_paths(selector: &Selector) -> Result<(), String> {
    let Selector::Files(paths) = selector else {
        return Ok(());
    };
    for path in paths {
        let mut depth = 0i32;
        for segment in path.split('/') {
            match segment {
                ".." => depth -= 1,
                "" | "." => {}
                _ => depth += 1,
            }
            if depth < 0 {
                return Err(format!("path '{path}' leaves the project root"));
            }
        }
        // shortcut: every `.spec` path is refused, not only those under the spec root; a spec source is read
        // through the entities it declares. Narrow it when a project keeps spec text outside its spec root.
        if path.is_empty()
            || path.starts_with('/')
            || path.contains(['*', '?', '['])
            || path.ends_with(".spec")
        {
            return Err(format!(
                "path '{path}' must be a file from the project root, with no glob, and not a .spec source"
            ));
        }
    }
    Ok(())
}

/// Who reads a chain the registry checks: a judged rule's verdict, or a trace route (ADR 0330). It picks the reason a
/// computed or verdict-written reference is refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Reader {
    Verdict,
    Route,
}

/// Why a chain cannot be walked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum WalkError {
    /// It names, or its reference reaches only, `kind`, which no loaded extension declares.
    Unloaded { kind: String, why: String },
    /// Anything else.
    Invalid(String),
}

impl WalkError {
    /// The reason, whichever it is.
    pub(crate) fn why(self) -> String {
        match self {
            WalkError::Unloaded { why, .. } | WalkError::Invalid(why) => why,
        }
    }
}

impl From<String> for WalkError {
    fn from(why: String) -> Self {
        WalkError::Invalid(why)
    }
}

/// The words of `path` the first field of `kind` takes: a block field takes the member words after it, down to the
/// reference that ends it (`requires.phase`); `Err` when `kind` declares none, or it ends in no reference.
/// Type-directed (ADR 0290): the words left are the next step's.
pub(crate) fn reference_end(
    registries: Registries<'_>,
    kind: &str,
    words: &[&str],
    reader: Reader,
) -> Result<usize, String> {
    for end in 1..=words.len() {
        let path = words[..end].join(".");
        let (_, form) = registries
            .fields
            .get_path(kind, &path)
            .ok_or_else(|| format!("kind '{kind}' declares no field '{path}'"))?;
        if form.field_type().is_reference() {
            forbid_written(kind, &path, form.writer(), reader)?;
            return Ok(end);
        }
        if !form.field_type().is_block() {
            return Err(format!("'{kind}.{path}' is not a reference"));
        }
    }
    Err(format!("'{kind}.{}' is not a reference", words.join(".")))
}

/// What a chain's first `steps` steps reach, statically, from entities of the kinds `this` (its `self`): the kinds
/// its entities may be of.
pub(crate) fn walk(
    chain: &Chain,
    steps: usize,
    this: &BTreeSet<String>,
    registries: Registries<'_>,
    reader: Reader,
) -> Result<BTreeSet<String>, WalkError> {
    let loaded = |kind: &String| {
        if registries.kinds.contains(kind) {
            Ok(())
        } else {
            Err(WalkError::Unloaded {
                kind: kind.clone(),
                why: format!("kind '{kind}' is not loaded"),
            })
        }
    };
    let mut kinds: BTreeSet<String> = match &chain.head {
        Head::This => this.clone(),
        Head::Kinds(kinds) => {
            kinds.iter().try_for_each(loaded)?;
            kinds.iter().cloned().collect()
        }
    };
    for step in &chain.steps[..steps] {
        kinds = match step {
            Step::Path(path) => follow(registries, kinds, path, reader)?,
            Step::Referrers {
                kinds: sources,
                path,
            } => {
                sources.iter().try_for_each(loaded)?;
                let words: Vec<&str> = path.split('.').collect();
                // The path may go on past its reference (ADR 0304): the prefix finds the referrers, the rest is a
                // forward path from each.
                let mut ends = BTreeSet::new();
                for kind in sources {
                    let end = reference_end(registries, kind, &words, reader)?;
                    ends.insert(end);
                    let targets = targets_of(registries, kind, &words[..end].join("."));
                    if !targets.is_empty() && targets.is_disjoint(&kinds) {
                        return Err(WalkError::Invalid(format!(
                            "'{kind}.{path}' names no kind of the entities before it"
                        )));
                    }
                }
                let end = match ends.iter().collect::<Vec<_>>()[..] {
                    [one] => *one,
                    _ => {
                        return Err(WalkError::Invalid(format!(
                            "'{path}' ends at a different reference in each of its kinds"
                        )));
                    }
                };
                let referrers: BTreeSet<String> = sources.iter().cloned().collect();
                if end == words.len() {
                    referrers
                } else {
                    follow(registries, referrers, &words[end..].join("."), reader)?
                }
            }
        };
    }
    Ok(kinds)
}

/// One hop from `kinds` along the reference the first words of `words` end at: the words it took and the kinds it
/// reaches. A reference whose target kinds are all unloaded is [`WalkError::Unloaded`], naming the first.
fn hop(
    registries: Registries<'_>,
    kinds: &BTreeSet<String>,
    words: &[&str],
    reader: Reader,
) -> Result<(usize, BTreeSet<String>), WalkError> {
    let mut ends = BTreeSet::new();
    let mut reached = BTreeSet::new();
    let mut unloaded = BTreeSet::new();
    for kind in kinds {
        let end = reference_end(registries, kind, words, reader)?;
        ends.insert(end);
        let path = words[..end].join(".");
        reached.extend(targets_of(registries, kind, &path));
        if let Some((_, form)) = registries.fields.get_path(kind, &path) {
            let absent = |k: &&String| !registries.kinds.contains(k);
            unloaded.extend(form.target_kinds().iter().filter(absent).cloned());
        }
    }
    let why = || {
        format!(
            "'{}' reaches no one kind of loaded entities",
            words.join(".")
        )
    };
    match (ends.len(), reached.is_empty(), unloaded.into_iter().next()) {
        (1, false, _) => Ok((ends.into_iter().next().unwrap_or(1), reached)),
        (1, true, Some(kind)) => Err(WalkError::Unloaded { kind, why: why() }),
        _ => Err(WalkError::Invalid(why())),
    }
}

/// The kinds reached by following `path` from entities of `kinds`: a reference after a reference is the next hop.
fn follow(
    registries: Registries<'_>,
    mut kinds: BTreeSet<String>,
    path: &str,
    reader: Reader,
) -> Result<BTreeSet<String>, WalkError> {
    let words: Vec<&str> = path.split('.').collect();
    let mut at = 0;
    while at < words.len() {
        let (taken, reached) = hop(registries, &kinds, &words[at..], reader)?;
        at += taken;
        kinds = reached;
    }
    Ok(kinds)
}

/// Whether following `path` from `kinds` ends at a file field (or member) of every kind it reaches.
fn ends_at_file(
    registries: Registries<'_>,
    mut kinds: BTreeSet<String>,
    path: &str,
) -> Result<(), String> {
    let words: Vec<&str> = path.split('.').collect();
    let mut at = 0;
    loop {
        let rest = words[at..].join(".");
        // A file field, or (ADR 0304) a string or string list field holding a path from the project root.
        let file = |kind: &String| {
            registries
                .fields
                .get_path(kind, &rest)
                .is_some_and(|(declared, form)| {
                    declared.file_reference
                        || matches!(form.field_type(), FieldType::String | FieldType::StringList)
                })
        };
        if kinds.iter().all(file) {
            return Ok(());
        }
        if at + 1 >= words.len() {
            return Err(format!("'{path}' is not a file field"));
        }
        let (taken, reached) =
            hop(registries, &kinds, &words[at..], Reader::Verdict).map_err(WalkError::why)?;
        at += taken;
        kinds = reached;
    }
}

/// The loaded kinds the reference `path` of `kind` may name.
fn targets_of(registries: Registries<'_>, kind: &str, path: &str) -> BTreeSet<String> {
    registries
        .fields
        .get_path(kind, path)
        .map(|(_, form)| {
            form.target_kinds()
                .iter()
                .filter(|k| registries.kinds.contains(k))
                .cloned()
                .collect()
        })
        .unwrap_or_default()
}

/// A computed or verdict-written reference makes no edge (ADR 0299 D1): a verdict cannot read what it helps write, and
/// a route cannot follow it.
fn forbid_written(
    kind: &str,
    path: &str,
    writer: Option<FieldWriter>,
    reader: Reader,
) -> Result<(), String> {
    match (writer, reader) {
        (Some(FieldWriter::Computed | FieldWriter::Verdict), Reader::Verdict) => Err(format!(
            "'{kind}.{path}' is computed or written by a verdict: a verdict cannot read what it helps write"
        )),
        (Some(FieldWriter::Computed | FieldWriter::Verdict), Reader::Route) => Err(format!(
            "'{kind}.{path}' is computed or written by a verdict: it makes no edge, so a route cannot follow it"
        )),
        _ => Ok(()),
    }
}

/// Every selector of the rule resolved, and every field it names declared: `Err` is the reason the rule can never
/// work, which is W112. An entity or test chain that reaches only a kind of an optional peer the project does not
/// load (`unloaded`) is inert, as a route's step is (ADR 0330): it selects nothing, and the rule is kept (ADR 0331
/// D7). A chain into any other kind no loaded extension declares is W112.
pub(crate) fn resolve(
    targets: &[JudgedTargetRule],
    descriptor: &JudgedDescriptor,
    extension: &str,
    registries: Registries<'_>,
    unloaded: &UnloadedPeers<'_>,
) -> Result<(), String> {
    for target in targets {
        if !registries.kinds.contains(&target.kind) {
            return Err(format!("target kind '{}' is not loaded", target.kind));
        }
        let this = BTreeSet::from([target.kind.clone()]);
        for (name, selector) in &target.inputs {
            let why = |why: String| format!("target '{}': input '{name}': {why}", target.kind);
            let walked = |steps: usize, chain: &Chain| {
                walk(chain, steps, &this, registries, Reader::Verdict)
            };
            let selects = |chain: &Chain| match walked(chain.steps.len(), chain) {
                Err(WalkError::Unloaded { kind, .. }) if unloaded.declare(&kind) => Ok(()),
                walked => walked.map(drop).map_err(|e| why(e.why())),
            };
            match selector {
                Selector::Entities(chain) => selects(chain)?,
                Selector::Tests(chains) => chains.iter().try_for_each(selects)?,
                Selector::FilesOf(chain) => {
                    let (last, steps) = chain.steps.split_last().ok_or_else(|| {
                        why("file() of a chain needs a step ending at a file field".into())
                    })?;
                    let kinds = walked(steps.len(), chain).map_err(|e| why(e.why()))?;
                    match last {
                        Step::Path(path) => ends_at_file(registries, kinds, path).map_err(why)?,
                        Step::Referrers { kinds, path } => {
                            ends_at_file(registries, kinds.iter().cloned().collect(), path)
                                .map_err(why)?;
                        }
                    }
                }
                Selector::Files(_) => {}
            }
        }
        for field in &target.unless_set {
            if registries.fields.get_path(&target.kind, field).is_none() {
                return Err(format!(
                    "target '{}': unless_set field '{field}' is not declared",
                    target.kind
                ));
            }
        }
    }
    let judged_kinds: BTreeSet<&str> = targets.iter().map(|t| t.kind.as_str()).collect();
    let bound = |entry: &String, what: &str| -> Result<(String, String), String> {
        let (kind, field) = entry
            .split_once('.')
            .ok_or_else(|| format!("{what} '{entry}' is not 'kind.field'"))?;
        let own = judged_kinds.contains(kind)
            && registries
                .fields
                .get(kind, field.split('.').next().unwrap_or(field))
                .is_some_and(|f| f.source_extension() == extension);
        let (_, form) = registries
            .fields
            .get_path(kind, field)
            .filter(|_| own)
            .ok_or_else(|| {
                format!("{what} '{entry}' is not a field of a target kind of this extension")
            })?;
        if form.writer() != Some(FieldWriter::Verdict) {
            return Err(format!("{what} '{entry}' is not written by a verdict"));
        }
        Ok((kind.to_string(), field.to_string()))
    };
    for entry in &descriptor.values {
        bound(entry, "value")?;
    }
    for entry in &descriptor.outcome_fields {
        let (kind, field) = bound(entry, "outcome field")?;
        let enum_values: BTreeSet<&String> = registries
            .fields
            .get_path(&kind, &field)
            .map(|(_, form)| form.enum_values().iter().collect())
            .unwrap_or_default();
        if enum_values != descriptor.outcomes.iter().collect() {
            return Err(format!(
                "outcome field '{entry}' must be an enum of exactly the outcomes"
            ));
        }
    }
    Ok(())
}
