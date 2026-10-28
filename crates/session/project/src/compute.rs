//! Compute passes (ADR 0287): the passes an extension declares with `phase: "compute"` run before the rules, each
//! over the snapshot with the values computed so far, and the values they answer join the compiled project beside
//! the graph. Each value is admitted by the form of the field it is for; one that is not is W165 and dropped.

use serde_json::Value;
use specforge_common::{Diagnostic, codes};
use specforge_graph::field_form::declared_path;
use specforge_graph::{Computed, Found};
use specforge_protocol_types::PassValue;
use specforge_registry::{DeclaredPass, EntitySnapshot};
use specforge_wasm::{ExtensionCalls, WasmRuntime};

use crate::Environment;
use crate::baseline::Baseline;
use crate::judged::Judgements;
use crate::passes::{PassContext, pass_findings, pass_input};

/// Run `env`'s compute passes over `authored`: what they computed, and what they reported (their findings, E028
/// for one that failed, W165 for each value refused).
pub(crate) fn run(
    env: &Environment,
    authored: &EntitySnapshot,
    runtime: &dyn WasmRuntime,
    baseline: Option<&Baseline>,
    given: &Computed,
    judgements: &Judgements,
) -> (Computed, Vec<Diagnostic>) {
    let mut computed = given.clone();
    let mut diagnostics = Vec::new();
    let calls = ExtensionCalls::new(runtime);
    let mut seen = if given.is_empty() {
        authored.clone()
    } else {
        authored.with_computed(given, env.registries().fields())
    };
    for declared in env.registries().compute_passes() {
        let input = {
            let mut payload = pass_input(&PassContext {
                judgements: Some(judgements),
                ..PassContext::of(&seen)
            });
            payload.previous = baseline
                .filter(|_| !declared.baseline().is_empty())
                .map(|b| b.pass_view(declared.baseline()));
            ExtensionCalls::encode(&payload)
        };
        let output = match calls.run_pass(&declared.extension, &declared.pass.name, &input) {
            Ok(output) => output,
            Err(error) => {
                diagnostics.push(error.diagnostic());
                continue;
            }
        };
        let values = output.values.clone();
        diagnostics.extend(pass_findings(
            &declared.extension,
            &declared.pass.name,
            output,
            authored,
            env.registries().codes(),
        ));
        let before = computed.clone();
        for value in values {
            if let Err(why) = admit(env, authored, declared, &value, &mut computed) {
                let span = authored.get(&value.entity).map(|(r, _)| r.span.clone());
                let mut refused = Diagnostic::new(
                    codes::W165,
                    format!(
                        "pass '{}' of '{}' gave '{}' of '{}': {why}",
                        declared.pass.name, declared.extension, value.field, value.entity
                    ),
                );
                if let Some(span) = span {
                    refused = refused.with_span(span);
                }
                diagnostics.push(refused);
            }
        }
        if computed != before {
            seen = authored.with_computed(&computed, env.registries().fields());
        }
    }
    (computed, diagnostics)
}

/// Admit `value` into `computed`, or say why not.
fn admit(
    env: &Environment,
    authored: &EntitySnapshot,
    pass: &DeclaredPass,
    value: &PassValue,
    computed: &mut Computed,
) -> Result<(), String> {
    let id = match authored.find(&value.entity) {
        Found::One(id) => id,
        Found::Ambiguous(_) => return Err("the entity is ambiguous".into()),
        Found::None(_) | Found::Mistyped(_) => {
            return Err("the project holds no such entity".into());
        }
    };
    let (record, _) = authored
        .get(id.as_str())
        .ok_or("the project holds no such entity")?;
    let declared = declared_path(&value.field);
    if !pass
        .computes()
        .contains(&format!("{}.{declared}", record.kind))
    {
        return Err(format!(
            "it does not list '{}.{declared}' in computes",
            record.kind
        ));
    }
    let (_, form) = env
        .registries()
        .fields()
        .get_path(&record.kind, &declared)
        .ok_or("the kind has no such field")?;
    if let Some(at) = value.field.find(['.', '[']) {
        let top = record.fields.iter().find(|f| f.key == value.field[..at]);
        let holds = |written: &Value| match value.field[at..].strip_prefix('[') {
            Some(tail) => tail
                .split_once(']')
                .and_then(|(i, _)| i.parse::<usize>().ok())
                .is_some_and(|i| written.as_array().is_some_and(|items| i < items.len())),
            None => written.is_object(),
        };
        if !top.and_then(|t| t.written.as_ref()).is_some_and(holds) {
            return Err("the entity writes no such item".into());
        }
    }
    let admitted = form.admit_json(&value.value).map_err(|refusal| {
        refusal
            .suggestion
            .unwrap_or_else(|| format!("the field is declared {}", form.field_type().as_str()))
    })?;
    computed.insert(id.as_str(), &value.field, admitted);
    Ok(())
}
