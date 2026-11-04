//! What a string value offers in the editor by the form its checker declares (ADR 0336 D5): a path completes its roots
//! and the entries inside them, and a path or a URL links to what it names. The form is the checker's `value`, never its
//! name. `Contained` holds every path, so nothing outside a root, and nothing on the credential deny-list, is listed or
//! linked.

use specforge_effects::contained::{Contained, Lookup, RealFs};
use specforge_ops::view::ProjectView;
use specforge_protocol_types::{Root, RootedPath, ValueForm};
use specforge_registry::CheckRegistry;
use tower_lsp::lsp_types::{CompletionItem, CompletionItemKind, DocumentLink, Url};

use crate::document::StringValue;
use crate::state::LspState;

/// What a checker reads a value as, with where a bare path starts.
enum Form {
    Path(Root),
    Url,
}

/// The forms the checks on `field` of `kind` read its values as, in check order.
fn forms(view: &ProjectView, kind: &str, field: &str) -> Vec<Form> {
    let checks = view.registries().checks();
    checks
        .on(kind, field)
        .iter()
        .flat_map(|check| check.calls())
        .filter_map(|call| {
            let checker = checks.checker(&call.extension, &call.checker)?;
            match checker.value {
                ValueForm::Path => {
                    Some(Form::Path(CheckRegistry::default_root(checker, &call.args)))
                }
                ValueForm::Url => Some(Form::Url),
                _ => None,
            }
        })
        .collect()
}

/// What completes inside `value` when a check reads it as a path: the `<root>/` prefixes while none is written, then
/// the entries of the written directory. A prefix that leaves its root offers nothing.
pub fn path_items(view: &ProjectView, value: &StringValue<'_>) -> Vec<CompletionItem> {
    let default = forms(view, value.kind, &value.field)
        .into_iter()
        .find_map(|form| match form {
            Form::Path(root) => Some(root),
            Form::Url => None,
        });
    let Some(default) = default else {
        return Vec::new();
    };
    let prefix = value.prefix;
    let roots = view.env().read_roots();
    if prefix.is_empty() || (prefix.starts_with('<') && !prefix.contains('>')) {
        let spec = roots.get(&Root::Spec).map(|_| "spec");
        let named = roots.named().map(|(name, _)| name);
        return spec
            .into_iter()
            .chain(["project"])
            .chain(named)
            .map(|name| format!("<{name}>/"))
            .filter(|label| label.starts_with(prefix))
            .map(|label| item(label, CompletionItemKind::FOLDER))
            .collect();
    }
    if prefix.starts_with('/') {
        return Vec::new();
    }
    let (directory, part) = prefix.rsplit_once('/').unwrap_or(("", prefix));
    let Ok(rooted) = RootedPath::parse(directory, default) else {
        return Vec::new();
    };
    let contained = Contained::new(&roots, &RealFs);
    let (base, below) = if rooted.path.is_empty() {
        let Some(base) = roots.get(&rooted.root) else {
            return Vec::new();
        };
        (base.to_path_buf(), String::new())
    } else {
        match contained.resolve_rooted(&rooted) {
            Ok(held) if !held.withheld => (held.at, format!("{}/", held.rel)),
            _ => return Vec::new(),
        }
    };
    let mut entries: Vec<CompletionItem> = RealFs
        .names(&base)
        .unwrap_or_default()
        .into_iter()
        .filter(|name| name.starts_with(part) && (!name.starts_with('.') || part.starts_with('.')))
        .filter_map(|name| {
            let entry = RootedPath {
                root: rooted.root.clone(),
                path: format!("{below}{name}"),
                written_root: None,
            };
            let held = contained.resolve_rooted(&entry).ok()?;
            (!held.withheld).then(|| {
                if held.at.is_dir() {
                    item(format!("{name}/"), CompletionItemKind::FOLDER)
                } else {
                    item(name, CompletionItemKind::FILE)
                }
            })
        })
        .collect();
    entries.sort_by(|a, b| a.label.cmp(&b.label));
    entries
}

fn item(label: String, kind: CompletionItemKind) -> CompletionItem {
    CompletionItem {
        label,
        kind: Some(kind),
        ..Default::default()
    }
}

/// The links of the open document `uri`: each path value to its file, each URL value to its URL.
pub fn document_links(state: &LspState, uri: &Url) -> Option<Vec<DocumentLink>> {
    let doc = state.document(uri)?;
    let view = state.view();
    let links = doc
        .string_values()
        .into_iter()
        .filter_map(|(value, range)| {
            Some(DocumentLink {
                range,
                target: Some(target(&view, &value)?),
                tooltip: None,
                data: None,
            })
        })
        .collect();
    Some(links)
}

/// What `value` links to: `None` for a path that leaves its root, is missing or names an unknown root, and for a URL
/// that is not `http` or `https`.
fn target(view: &ProjectView, value: &StringValue<'_>) -> Option<Url> {
    forms(view, value.kind, &value.field)
        .into_iter()
        .find_map(|form| match form {
            Form::Url => Url::parse(value.text)
                .ok()
                .filter(|url| matches!(url.scheme(), "http" | "https")),
            Form::Path(default) => {
                let roots = view.env().read_roots();
                let held = Contained::new(&roots, &RealFs)
                    .resolve(value.text, default)
                    .ok()?;
                (held.exists && !held.withheld && held.at.is_file())
                    .then(|| Url::from_file_path(&held.at).ok())
                    .flatten()
            }
        })
}
