//! `specforge.json` as the editor shows it (ADR 0288): its `settings` completed, hovered and checked as they are
//! typed. Everything is read from the unsaved buffer against what the loaded extensions declare, with the same
//! resolution the environment runs on the saved file ([`ResolvedSettings::resolve`]); a save reloads the
//! environment, and the buffer is checked again against it. The file is never compiled as a source.

use serde_json::Value;
use specforge_common::json_span;
use specforge_project::{ResolvedSettings, SettingOrigin};
use specforge_protocol_types::Family;
use tower_lsp::lsp_types::{
    CompletionItem, CompletionItemKind, CompletionResponse, Diagnostic, Documentation, Hover,
    HoverContents, MarkupContent, MarkupKind, Position, Range, Url,
};

use crate::LspState;

/// The served project's own `specforge.json`.
pub fn config_path(state: &LspState) -> std::path::PathBuf {
    state.environment().root().join("specforge.json")
}

/// Whether `uri` names the served project's own `specforge.json`. A nested project's file of the same name is a
/// source of nothing here: it is not this project's config and its extensions are not this project's.
pub fn is_config(state: &LspState, uri: &Url) -> bool {
    let (path, own) = (crate::uri::path_of(uri), config_path(state));
    path == own || matches!((path.canonicalize(), own.canonicalize()), (Ok(a), Ok(b)) if a == b)
}

/// Whether `diagnostic` is a settings finding about `specforge.json`: published by [`diagnostics`] at its key in
/// the file, the one publisher, never with the project's diagnostics (which cannot place a span in that file).
pub fn is_finding(diagnostic: &specforge_common::Diagnostic) -> bool {
    use specforge_common::codes;
    [codes::E086, codes::E087, codes::W166]
        .iter()
        .any(|code| diagnostic.is(*code))
        && diagnostic
            .span
            .as_ref()
            .is_some_and(|s| s.file.as_str() == "specforge.json")
}

fn resolve(state: &LspState, text: &str) -> (ResolvedSettings, Vec<specforge_common::Diagnostic>) {
    ResolvedSettings::resolve(
        Some(text),
        state.registries().settings(),
        state.environment().enabled(),
    )
}

/// The E086, E087 and W166 `specforge.json` has now, as the client receives them: of the open buffer, else of the
/// file on disk.
pub fn diagnostics(state: &LspState, uri: &Url) -> Vec<Diagnostic> {
    let text = match state.document(uri) {
        Some(document) => document.text().to_string(),
        None => match std::fs::read_to_string(crate::uri::path_of(uri)) {
            Ok(text) => text,
            Err(_) => return Vec::new(),
        },
    };
    let lines = crate::LineIndex::new(&text);
    let codes = state.registries().codes();
    resolve(state, &text)
        .1
        .iter()
        .map(|found| {
            crate::publish::diagnostic_to_lsp(found, codes, |span| Some(lines.range(span)))
        })
        .collect()
}

/// The word a type's value is started from: its default, else a placeholder of its type.
fn starter(descriptor: &specforge_protocol_types::SettingDescriptor) -> Value {
    if let Some(default) = &descriptor.default {
        return default.clone();
    }
    match descriptor.field_type.as_str() {
        "bool" => Value::Bool(false),
        "integer" => Value::from(descriptor.minimum.unwrap_or(0)),
        "enum" => descriptor
            .enum_values
            .first()
            .map_or(Value::from(""), |v| Value::from(v.clone())),
        "string_list" | "enum_list" | "block_list" => Value::Array(Vec::new()),
        "block" => Value::Object(Default::default()),
        _ => Value::from(""),
    }
}

/// Every key a family can be written as, when each `*` lists its keys; none when one takes any segment.
fn closed_keys(descriptor: &specforge_protocol_types::SettingDescriptor) -> Vec<String> {
    let mut keys = vec![String::new()];
    let mut star = 0;
    for (n, segment) in descriptor.name.split('.').enumerate() {
        let options: Vec<String> = if segment == "*" {
            let Some(listed) = descriptor.keys.get(star).filter(|k| !k.is_empty()) else {
                return Vec::new();
            };
            star += 1;
            listed.clone()
        } else {
            vec![segment.to_string()]
        };
        keys = keys
            .iter()
            .flat_map(|prefix| {
                options.iter().map(move |option| {
                    if n == 0 {
                        option.clone()
                    } else {
                        format!("{prefix}.{option}")
                    }
                })
            })
            .collect();
    }
    keys
}

fn item(
    label: String,
    kind: CompletionItemKind,
    detail: Option<String>,
    documentation: Option<String>,
    insert: Option<String>,
) -> CompletionItem {
    CompletionItem {
        label,
        kind: Some(kind),
        detail,
        documentation: documentation.map(Documentation::String),
        insert_text: insert,
        ..CompletionItem::default()
    }
}

/// What the cursor at `position` of the open buffer `uri` can write: the extensions with settings under
/// `settings`; under an extension the settings not yet written (each with its default) and a family's closed keys;
/// at a value its enum words or `true` and `false`.
pub fn completion(state: &LspState, uri: &Url, position: Position) -> Option<CompletionResponse> {
    let document = state.document(uri)?;
    let offset = document.index().offset(position)?;
    let text = document.text();
    let index = json_span::index(text);
    let at = index.path_at(offset)?;
    let registry = state.registries().settings();
    let path: Vec<&str> = at.path.iter().map(String::as_str).collect();
    let mut items = Vec::new();
    match path.as_slice() {
        ["settings"] => {
            for (name, declared) in registry.iter() {
                if !declared.settings.is_empty() {
                    items.push(item(
                        format!("\"{name}\""),
                        CompletionItemKind::MODULE,
                        Some("extension".into()),
                        None,
                        Some(format!("\"{name}\": {{}}")),
                    ));
                }
            }
        }
        ["settings", extension] | ["settings", extension, _]
            if at.key.as_ref().is_some_and(|k| k.contains(&offset)) || path.len() == 2 =>
        {
            let declared = registry.of(extension)?;
            let written: Vec<&str> = index
                .children(&["settings", extension])
                .iter()
                .filter_map(|c| c.path.last().map(String::as_str))
                .collect();
            for setting in &declared.settings {
                let descriptor = &setting.descriptor;
                let keys = if Family::is_family(&descriptor.name) {
                    closed_keys(descriptor)
                } else {
                    vec![descriptor.name.clone()]
                };
                for key in keys {
                    if written.contains(&key.as_str()) {
                        continue;
                    }
                    let value = starter(descriptor);
                    items.push(item(
                        key.clone(),
                        CompletionItemKind::PROPERTY,
                        Some(descriptor.field_type.clone()),
                        descriptor.description.clone(),
                        Some(format!("\"{key}\": {value}")),
                    ));
                }
            }
        }
        ["settings", extension, name] => {
            let setting = registry.of(extension)?.find(name)?;
            let descriptor = &setting.descriptor;
            match descriptor.field_type.as_str() {
                "bool" => {
                    for word in ["true", "false"] {
                        items.push(item(
                            word.into(),
                            CompletionItemKind::VALUE,
                            Some("bool".into()),
                            None,
                            None,
                        ));
                    }
                }
                "enum" => {
                    for word in &descriptor.enum_values {
                        items.push(item(
                            format!("\"{word}\""),
                            CompletionItemKind::ENUM_MEMBER,
                            Some("enum".into()),
                            None,
                            None,
                        ));
                    }
                }
                _ => {}
            }
        }
        _ => {}
    }
    Some(CompletionResponse::Array(items))
}

/// The setting under the cursor of the open buffer `uri`: its type, effective value and where it came from, its
/// default, and the codes it gates with whether they are reported now.
pub fn hover(state: &LspState, uri: &Url, position: Position) -> Option<Hover> {
    let document = state.document(uri)?;
    let offset = document.index().offset(position)?;
    let text = document.text();
    let located = json_span::path_at(text, offset)?;
    let key = located.key.as_ref()?;
    if !key.contains(&offset) && !(key.start..=key.end).contains(&offset) {
        return None;
    }
    let (resolved, _) = resolve(state, text);
    let path: Vec<&str> = located.path.iter().map(String::as_str).collect();
    let body = match path.as_slice() {
        ["settings", extension, name] => {
            let declared = state.registries().settings().of(extension)?.find(name)?;
            let setting = resolved.of(extension)?.get(name)?;
            let mut lines = vec![format!("**{name}** `{}`", declared.descriptor.field_type)];
            if let Some(description) = &declared.descriptor.description {
                lines.push(description.clone());
            }
            let value = setting
                .value
                .as_ref()
                .map_or_else(|| "unset".to_string(), ToString::to_string);
            let origin = match &setting.origin {
                SettingOrigin::Explicit => "written in specforge.json".to_string(),
                SettingOrigin::Preset { source, value } => format!("preset by {source} = {value}"),
                SettingOrigin::Default => "the default".to_string(),
                SettingOrigin::Unset => "nothing sets it".to_string(),
            };
            lines.push(format!("value: `{value}` ({origin})"));
            if let Some(default) = &declared.descriptor.default {
                lines.push(format!("default: `{default}`"));
            }
            let descriptor = &declared.descriptor;
            for (code, when) in descriptor.gates.iter().map(|c| (c, "gates")).chain(
                descriptor
                    .gates_when_on
                    .iter()
                    .map(|c| (c, "gates while on")),
            ) {
                let qualified = specforge_diagnostics::qualify(extension, code);
                let state = if resolved.gated().contains(&qualified) {
                    "not reported now"
                } else {
                    "reported"
                };
                lines.push(format!("{when} `{qualified}`: {state}"));
            }
            lines.join("\n\n")
        }
        ["settings", extension] => {
            let declared = state.registries().settings().of(extension)?;
            format!(
                "**{extension}**: {} setting(s), {} secret(s)",
                declared.settings.len(),
                declared.secrets.len()
            )
        }
        _ => return None,
    };
    let range = Range::new(
        document.index().position(key.start),
        document.index().position(key.end),
    );
    Some(Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value: body,
        }),
        range: Some(range),
    })
}
