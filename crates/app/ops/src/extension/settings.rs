//! `specforge settings` and its MCP tool: each loaded extension's settings as the project resolves them (ADR
//! 0288): the value, where it came from, the default, the codes it gates, and each secret's status. Read from the
//! view's environment, never `specforge.json` again.

use crate::view::ProjectView;
use serde::Serialize;
use serde_json::Value;
use specforge_common::shape::Shape;
use specforge_common::{DiagnosticList, Severity};
use specforge_project::SettingOrigin;
use specforge_protocol_types::Family;

/// One setting (one key of a family) as both surfaces print it (`McpSettingRow`).
#[derive(Debug, Clone, PartialEq, Serialize, Shape)]
pub struct SettingRow {
    /// The setting's name, a family's key as written (`models.planning`).
    pub name: String,
    /// The field type, as the declaration spells it.
    pub field_type: String,
    /// The effective value; absent: unset.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<Value>,
    /// `explicit`, `preset`, `default` or `unset`.
    pub origin: String,
    /// For a preset, the setting and value that selected it (`profile = strict`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preset: Option<String>,
    /// The declared default.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<Value>,
    /// The codes, as they print, this bool setting gates while it is false.
    pub gates: Vec<String>,
    /// The codes, as they print, this bool setting gates while it is true (ADR 0305).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub gates_when_on: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// One declared secret: whether its environment variable is set, never its value (`McpSecretRow`).
#[derive(Debug, Clone, PartialEq, Serialize, Shape)]
pub struct SecretRow {
    pub name: String,
    pub env: String,
    pub set: bool,
}

/// One extension's settings and secrets (`McpExtensionSettings`).
#[derive(Debug, Clone, PartialEq, Serialize, Shape)]
pub struct ExtensionSettingsRow {
    pub extension: String,
    pub settings: Vec<SettingRow>,
    pub secrets: Vec<SecretRow>,
}

/// A code a preset raises, and the level it is raised to (`McpRaisedCode`).
#[derive(Debug, Clone, PartialEq, Serialize, Shape)]
pub struct RaisedRow {
    pub code: String,
    pub severity: String,
}

/// What `specforge settings --format json` and `specforge.settings` print (`McpSettingsResult`).
#[derive(Debug, Clone, PartialEq, Serialize, Shape)]
pub struct SettingsDocument {
    /// The extensions that declare a setting or a secret, by name.
    pub extensions: Vec<ExtensionSettingsRow>,
    pub count: usize,
    /// The codes reported by no source now, as they print.
    pub gated: Vec<String>,
    /// The codes the selected presets raise.
    pub raised: Vec<RaisedRow>,
    /// The E086, E087 and W166 `specforge.json` produced.
    pub diagnostics: DiagnosticList,
}

fn origin(origin: &SettingOrigin) -> (&'static str, Option<String>) {
    match origin {
        SettingOrigin::Explicit => ("explicit", None),
        SettingOrigin::Preset { source, value } => ("preset", Some(format!("{source} = {value}"))),
        SettingOrigin::Default => ("default", None),
        SettingOrigin::Unset => ("unset", None),
    }
}

/// The view's settings: those with a value, or every declared one with `all`; `extension` names the one to
/// list. A secret is listed whenever its extension is.
pub fn settings(view: &ProjectView, all: bool, extension: Option<&str>) -> SettingsDocument {
    let resolved = view.env().settings();
    let mut extensions = Vec::new();
    for (name, declared) in view.registries().settings().iter() {
        if extension.is_some_and(|only| only != name) {
            continue;
        }
        let values = resolved.of(name);
        let mut rows = Vec::new();
        for item in &declared.settings {
            let descriptor = &item.descriptor;
            let described = |key: &str, value: Option<Value>, origin_of: &SettingOrigin| {
                let (origin, preset) = origin(origin_of);
                SettingRow {
                    name: key.to_string(),
                    field_type: descriptor.field_type.clone(),
                    value,
                    origin: origin.to_string(),
                    preset,
                    default: descriptor.default.clone(),
                    gates: descriptor
                        .gates
                        .iter()
                        .map(|code| specforge_diagnostics::qualify(name, code))
                        .collect(),
                    gates_when_on: descriptor
                        .gates_when_on
                        .iter()
                        .map(|code| specforge_diagnostics::qualify(name, code))
                        .collect(),
                    description: descriptor.description.clone(),
                }
            };
            if Family::is_family(&descriptor.name) {
                let keys: Vec<_> = values
                    .into_iter()
                    .flat_map(|v| v.family(&descriptor.name))
                    .collect();
                for (key, setting) in &keys {
                    rows.push(described(key, setting.value.clone(), &setting.origin));
                }
                if keys.is_empty() && all {
                    rows.push(described(&descriptor.name, None, &SettingOrigin::Unset));
                }
            } else if let Some(setting) = values
                .and_then(|v| v.get(&descriptor.name))
                .filter(|s| all || s.value.is_some())
            {
                rows.push(described(
                    &descriptor.name,
                    setting.value.clone(),
                    &setting.origin,
                ));
            }
        }
        let secrets: Vec<SecretRow> = declared
            .secrets
            .iter()
            .map(|s| SecretRow {
                name: s.name.clone(),
                env: s.env.clone(),
                set: specforge_wasm::ExtensionSettings::secret_is_set(&s.env),
            })
            .collect();
        if !rows.is_empty() || !secrets.is_empty() {
            extensions.push(ExtensionSettingsRow {
                extension: name.to_string(),
                settings: rows,
                secrets,
            });
        }
    }
    let level = |severity: Severity| match severity {
        Severity::Error => "error",
        Severity::Warning => "warning",
        Severity::Info => "info",
    };
    SettingsDocument {
        count: extensions.iter().map(|e| e.settings.len()).sum(),
        extensions,
        gated: resolved.gated().iter().cloned().collect(),
        raised: resolved
            .raises()
            .iter()
            .map(|(code, severity)| RaisedRow {
                code: code.to_string(),
                severity: level(severity).to_string(),
            })
            .collect(),
        diagnostics: DiagnosticList::new(
            view.env().settings_diagnostics().to_vec(),
            view.registries().codes(),
        ),
    }
}
