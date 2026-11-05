//! What a command, prompt, hook or collector declared the host serves it before the call (ADR 0337 D2), served from
//! one place. An input its call kind is not served was W021 at the declaration check and is never asked for here; a
//! pass's inputs are `specforge-project`'s. A hook's `git` input is the hook's own (its facts depend on the event).

use std::collections::BTreeMap;

use serde_json::Value;
use specforge_protocol_types::{
    CheckOutcome, FileMap, Input, Inputs, RawGraph, ServedSlash, StateView,
};

use crate::view::ProjectView;

/// The inputs one call of `extension` declares, over the project `view` reads.
pub(crate) struct Serve<'v, 'p> {
    view: &'v ProjectView<'p>,
    extension: &'v str,
    inputs: Inputs<'v>,
}

impl<'v, 'p> Serve<'v, 'p> {
    pub(crate) fn new(view: &'v ProjectView<'p>, extension: &'v str, inputs: &'v [Input]) -> Self {
        Serve {
            view,
            extension,
            inputs: Inputs(inputs),
        }
    }

    /// The facts of each `files` input, by read name. A call runs once: they are read from the plain disk (ADR 0293).
    pub(crate) fn files(&self) -> FileMap {
        let reads = self.inputs.files();
        if reads.is_empty() {
            return FileMap::default();
        }
        specforge_project::file_reads::read(
            &reads,
            self.view.entities(),
            &self.view.env().file_source(&specforge_registry::PlainDisk),
        )
    }

    /// The extension's state, for the `state` input; `session` names a hook's session.
    pub(crate) fn state(&self, session: Option<&str>) -> Option<StateView> {
        self.inputs
            .state()
            .then(|| crate::host::state::read(self.view, self.extension, session))
    }

    /// The graph, for the `graph` input.
    pub(crate) fn graph(&self) -> Option<RawGraph> {
        self.inputs.graph().then(|| {
            RawGraph::new(super::command::graph_export(self.view))
                .expect("the graph export is one JSON value")
        })
    }

    /// The slash commands the project serves, for the `slash_commands` input.
    pub(crate) fn slash_commands(&self) -> Vec<ServedSlash> {
        if self.inputs.slash_commands() {
            crate::host::render::served_slashes(self.view)
        } else {
            Vec::new()
        }
    }

    /// Each checked value's outcome, for the `check_outcomes` input: the rows of the run the view compiled (ADR 0336).
    pub(crate) fn check_outcomes(&self) -> Vec<CheckOutcome> {
        if self.inputs.check_outcomes() {
            self.view.project().value_checks().outcomes()
        } else {
            Vec::new()
        }
    }

    /// Each `setting` input's value, keyed `<extension>:<name>`; a setting that is not set is left out (ADR 0323), and
    /// so is one W021 served empty (its own extension's, or none).
    pub(crate) fn other_settings(&self) -> BTreeMap<String, Value> {
        self.inputs
            .settings()
            .filter(|(extension, name)| {
                specforge_registry::inputs::setting_problem(self.extension, extension, name)
                    .is_none()
            })
            .filter_map(|(extension, name)| {
                crate::host::render::setting(self.view, extension, name)
                    .map(|value| (format!("{extension}:{name}"), value))
            })
            .collect()
    }
}
