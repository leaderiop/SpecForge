//! Extension compiler passes: the input every `__pass_<name>` export
//! receives and how its answer is read. What an extension declares, and in
//! which order its passes run, is the registry build's
//! (`specforge_registry::RegistryBuild::passes`). Check-phase passes run with
//! every compile ([`crate::check_passes`]); the others under
//! `specforge analyze` (`specforge_ops::analysis::analyze`). Both phases run
//! their passes through [`run_extension_passes`]. Findings are standard host
//! diagnostics.

use specforge_common::{Diagnostic, codes};
use specforge_diagnostics::{Codebook, Level, qualify};
use specforge_protocol_types::{PassInput, PassOutput, PassSeverity, TestResults, Trigger};
use specforge_wasm::{Encoded, ExtensionCalls};
use std::collections::HashSet;

use crate::baseline::Baseline;
use crate::coverage::TestReport;
use crate::file_reads::{self, FileSource};
use specforge_registry::EntitySnapshot;

/// What the passes of one run are given: the entity snapshot they read, and
/// what the phase has beside it. Each field is read: by [`pass_input`], and
/// `entities` also for the spans of findings.
pub struct PassContext<'a> {
    /// The graph's entity snapshot (ADR 0019): what the passes receive,
    /// each entity with its standing (who is exempt).
    pub entities: &'a EntitySnapshot,
    /// Parsed `--test-results` report, when provided; forwarded to extension
    /// passes (the coverage pass scores proof from it).
    pub test_results: Option<&'a TestReport>,
    /// Entity ids whose formal claims the prove pass ENTAILED from the
    /// declared bounds (RES-25). `None` when the prove pass did not run;
    /// a proved claim discharges `verify property` obligations without
    /// executable tests.
    pub proved_claims: Option<&'a HashSet<String>>,
    /// The baseline, moves applied (ADR 0287): handed to a pass as `previous`, restricted to the kinds it names in
    /// `baseline`.
    pub previous: Option<&'a Baseline>,
    /// Where each judged rule stands on each entity (ADR 0290): handed to a pass on the entity, gated rules absent.
    pub judgements: Option<&'a crate::judged::Judgements>,
    /// Why a guard pass runs (ADR 0291): a gate the host is evaluating. Absent, a write (protocol 1.10.0).
    pub trigger: Option<&'a Trigger>,
    /// Where a pass that declares `reads` has its files read from (ADR 0293). Absent, such a pass is sent no
    /// `files`.
    pub files: Option<FileSource<'a>>,
    /// The recorded report, for a check, compute or guard pass that declares `reads_tests` (protocol 1.11.0); an
    /// analyze pass is given `test_results` instead. Absent, no pass is sent a report it did not ask `analyze` for.
    pub reported: Option<&'a TestResults>,
}

impl<'a> PassContext<'a> {
    /// `entities`, and nothing beside them: what a compile has before it
    /// reads the baseline.
    pub fn of(entities: &'a EntitySnapshot) -> Self {
        Self {
            entities,
            test_results: None,
            proved_claims: None,
            previous: None,
            judgements: None,
            trigger: None,
            files: None,
            reported: None,
        }
    }
}

// ── Extension-owned compiler passes (WASM-only migration, Phase 4) ─────────

/// Result of one extension-owned compiler pass.
pub struct ExtensionPassReport {
    /// `<extension>:<pass>` identifier.
    pub name: String,
    pub findings: Vec<Diagnostic>,
    pub summary: serde_json::Value,
}

/// The input every `__pass_<name>` export receives (the protocol's
/// `PassInput`): the entity snapshot's adapters (its entities and edges,
/// ADR 0019), and the test results and proved claims when the caller has
/// them. Each entity carries how the coverage rule sees it: `testable` is
/// its kind's flag (a kind that merely accepts `verify` statements, a
/// formal `property`, does not count), and `owes_none` says it owes no
/// obligations of its own (ADR 0004, D2-b), its standing's.
pub fn pass_input(input: &PassContext) -> PassInput {
    let mut entities = input.entities.pass_entities();
    if let Some(judgements) = input.judgements {
        for entity in &mut entities {
            entity.judgements = judgements.of(&entity.id).cloned().collect();
        }
    }
    PassInput {
        entities,
        edges: input.entities.pass_edges(),
        test_results: input.test_results.map(|r| r.results(input.entities)),
        proved_claims: input.proved_claims.map(sorted_ids),
        // Every kind: a pass is handed only the kinds it names ([`run_extension_passes`]).
        previous: None,
        trigger: input.trigger.cloned(),
        files: Default::default(),
    }
}

/// The ids of `claims`, sorted.
fn sorted_ids(claims: &std::collections::HashSet<String>) -> Vec<String> {
    let mut ids: Vec<String> = claims.iter().cloned().collect();
    ids.sort();
    ids
}

/// The host diagnostics of `extension`'s pass `pass` answering `output`: in
/// canonical order, a span-less one naming an entity of `entities` given
/// that entity's span. Each finding's code is the extension's local code under
/// its namespace (`acme.lint/W001`; a first-party extension's stays bare), so
/// it never prints as core's or another extension's. A code the extension may
/// not report at the severity it gave (`Codebook::check`) is reported once, as
/// W150 after the findings; the findings themselves are kept.
pub fn pass_findings(
    extension: &str,
    pass: &str,
    output: PassOutput,
    entities: &EntitySnapshot,
    codes: &Codebook,
) -> Vec<Diagnostic> {
    let mut misused = code_misuse(codes, extension, pass, &output);
    let mut findings = specforge_wasm::pass_diagnostics(extension, output, |id| {
        entities.get(id).map(|(record, _)| record.span.clone())
    });
    for finding in &mut findings {
        finding.code = qualify(extension, &finding.code);
    }
    findings.append(&mut misused);
    findings
}

/// W150 for each distinct (code, severity) of `output` that `extension`
/// may not report.
fn code_misuse(
    codes_of: &Codebook,
    extension: &str,
    pass: &str,
    output: &PassOutput,
) -> Vec<Diagnostic> {
    let mut seen = HashSet::new();
    let mut diagnostics = Vec::new();
    for finding in &output.diagnostics {
        let level = match finding.severity {
            PassSeverity::Error => Level::Error,
            PassSeverity::Warning => Level::Warning,
            PassSeverity::Info => Level::Info,
        };
        let Err(misuse) = codes_of.check(extension, &finding.code, level) else {
            continue;
        };
        if !seen.insert((finding.code.as_str(), level)) {
            continue;
        }
        diagnostics.push(
            Diagnostic::new(
                codes::W150,
                format!(
                    "extension '{extension}' pass '{pass}' reported '{}': {misuse}",
                    finding.code
                ),
            )
            .with_suggestion(misuse.suggestion()),
        );
    }
    diagnostics.sort_by(|a, b| a.message.cmp(&b.message));
    diagnostics
}

/// Run `passes`, in the order given (the caller picks the phase:
/// `check_passes` for a compile, `analyze_passes` for analyze), each on
/// `context`'s input, encoded once: one report per pass. A pass that fails
/// (it traps, answers what does not parse, or its input did not encode) has
/// a report of its own with one E028 finding and `failed` in its summary;
/// the others still run.
pub fn run_extension_passes<'p>(
    passes: impl IntoIterator<Item = &'p specforge_registry::DeclaredPass>,
    context: &PassContext,
    runtime: &dyn specforge_wasm::runtime::WasmRuntime,
    codes: &Codebook,
) -> Vec<ExtensionPassReport> {
    let passes: Vec<&specforge_registry::DeclaredPass> = passes.into_iter().collect();
    if passes.is_empty() {
        return Vec::new();
    }

    let payload = pass_input(context);
    let entities_analyzed = payload.entities.len();
    let encoded = ExtensionCalls::encode(&payload);
    // `previous` differs by the kinds a pass names: encoded once per distinct set.
    let mut with_baseline: std::collections::HashMap<Vec<String>, Encoded<PassInput>> =
        Default::default();
    let calls = ExtensionCalls::new(runtime);
    let mut memo = file_reads::Memo::default();

    let mut reports = Vec::new();
    for declared in passes {
        let report_name = declared.full_name();
        let pass = &declared.pass;
        let mut summary = serde_json::Map::new();
        summary.insert("extension".into(), declared.extension.clone().into());
        summary.insert("pass".into(), pass.name.clone().into());
        summary.insert("entities_analyzed".into(), entities_analyzed.into());
        // A pass that declares `reads` is given the facts of its files, one that declares `reads_tests` the recorded
        // report, so its payload is its own (ADR 0293).
        let own;
        let tests = context.reported.filter(|_| declared.reads_tests());
        let input: &Encoded<PassInput> = match (context.files, declared.reads()) {
            (files, reads) if tests.is_some() || (files.is_some() && !reads.is_empty()) => {
                own = ExtensionCalls::encode(&PassInput {
                    previous: context
                        .previous
                        .filter(|_| !declared.baseline().is_empty())
                        .map(|b| b.pass_view(declared.baseline())),
                    files: files
                        .filter(|_| !reads.is_empty())
                        .map(|source| memo.read(reads, context.entities, &source))
                        .unwrap_or_default(),
                    test_results: tests.cloned().or_else(|| payload.test_results.clone()),
                    ..payload.clone()
                });
                &own
            }
            _ => match context.previous {
                Some(baseline) if !declared.baseline().is_empty() => {
                    let mut kinds = declared.baseline().to_vec();
                    kinds.sort();
                    with_baseline.entry(kinds).or_insert_with_key(|kinds| {
                        ExtensionCalls::encode(&PassInput {
                            previous: Some(baseline.pass_view(kinds)),
                            ..payload.clone()
                        })
                    })
                }
                _ => &encoded,
            },
        };
        let answer = calls.run_pass(&declared.extension, &pass.name, input);
        let findings = match answer {
            Ok(output) => {
                let extra = output.summary.clone();
                summary.extend(extra);
                let refused = (!output.values.is_empty() && !declared.is_compute_phase())
                    .then_some(output.values.len());
                let mut findings = pass_findings(
                    &declared.extension,
                    &pass.name,
                    output,
                    context.entities,
                    codes,
                );
                // Only a compute pass gives values (ADR 0287): once per pass, and ignored.
                if let Some(count) = refused {
                    findings.push(Diagnostic::new(
                        codes::W165,
                        format!(
                            "pass '{}' of '{}' answered {count} value(s), but is not a compute pass: they are ignored",
                            pass.name, declared.extension
                        ),
                    ));
                }
                findings
            }
            Err(error) => {
                summary.insert("failed".into(), true.into());
                vec![error.diagnostic()]
            }
        };
        reports.push(ExtensionPassReport {
            name: report_name,
            findings,
            summary: serde_json::Value::Object(summary),
        });
    }
    reports
}
