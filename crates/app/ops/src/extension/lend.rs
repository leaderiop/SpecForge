//! What an effectful call is lent (ADR 0335 D4): the caller's ports, in its run's effects mode, under the extension's
//! grant, settled before the call so nothing prompts during it. A guest that imports no port (every builtin) is lent
//! nothing and costs nothing.

use specforge_common::{Diagnostic, codes};
use specforge_effects::{Roots, Uses};
use specforge_protocol_types::{Operation, SettingsItem};
use specforge_wasm::PortCall;

use crate::Caller;
use crate::settle::settle;
use crate::view::ProjectView;

/// What `extension`'s call of kind `kind` is lent over `view`: none when `caller` lends no port, or its guest imports
/// none (a port it then calls is answered `denied(scope)`). Approvals nobody gave stay ungiven: a request that needs
/// one is answered `denied(scope)`. W186 of a store that could not be read, or that lost an approval just given, is
/// kept with the denials.
pub fn port_call(
    view: &ProjectView<'_>,
    caller: Caller<'_>,
    extension: &str,
    kind: Operation,
) -> Option<PortCall> {
    let lending = caller.lent()?;
    if view.runtime()?.host_imports(extension).is_empty() {
        return None;
    }
    let root = view.project_root().ok()?;
    let env = view.env();
    let mut grant = specforge_project::effects::grants(env, root, lending.consent())
        .into_iter()
        .find(|grant| grant.extension() == extension)?;
    let uses: Uses = grant
        .capabilities()
        .iter()
        .map(|granted| (kind, granted.declared.name.clone()))
        .collect();
    let unusable: Vec<Diagnostic> = match settle(&mut grant, &uses, caller.asking(), lending.consent()) {
        Ok(settled) => settled
            .unsaved
            .iter()
            .map(|why| {
                Diagnostic::new(
                    codes::W186,
                    format!("an approval given for '{extension}' holds for this run and was not kept: {why}"),
                )
            })
            .collect(),
        Err(unsettled) => unsettled.store.map(|store| *store).into_iter().collect(),
    };
    let secrets = view
        .registries()
        .declarations()
        .iter()
        .flat_map(|declaration| &declaration.settings)
        .filter_map(|item| match item {
            SettingsItem::Secret(secret) => Some(secret.env.clone()),
            _ => None,
        })
        .collect();
    let roots = env
        .effects_allowed()
        .with_named(Roots::new(root, env.spec_root()));
    let effects =
        specforge_project::effects::lent(lending.mode(), lending.ports(), env.project_root());
    let call = PortCall::new(effects, lending.ports(), roots, grant, secrets);
    for unusable in unusable {
        call.deny(unusable);
    }
    Some(call)
}

/// `denials` as a person reads them, one a line: what a command adds to its stderr.
pub fn written(denials: &[Diagnostic]) -> String {
    denials.iter().map(|denial| format!("{denial}\n")).collect()
}
