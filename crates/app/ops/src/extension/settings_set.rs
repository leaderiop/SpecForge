//! `specforge settings set` and `unset` and the MCP tool `specforge.settings_set` (ADR 0296): one setting of
//! `specforge.json` written, or removed, admitted by its form first and never a secret, spliced as text so the rest of
//! the file is as the author left it, and written as one change under the write lock and the journal.
//!
//! The value is admitted by [`specforge_project::admit`], the load's own `secret` and `check`; under the lock the file
//! is read again and the splice is admitted again over the new text, so a load and a `set` cannot disagree.

use serde::Serialize;
use serde_json::{Value, json};
use specforge_common::shape::Shape;
use specforge_common::{Diagnostic, DiagnosticList, codes};
use specforge_project::{ResolvedSettings, admit, is_secret};
use specforge_protocol_types::FieldType;

use crate::change::locked;
use crate::config::{self, CONFIG_FILE};
use crate::view::ProjectView;
use crate::{Caller, OpError, OpErrorKind, Refusal, Wrote, config_edit};

/// What to do to the setting.
#[derive(Debug, Clone, PartialEq)]
pub enum SettingChange {
    /// The words of a command line, read by the setting's declared type.
    Words(Vec<String>),
    /// A JSON value, as given.
    Json(Value),
    /// Remove the member: the setting reverts to its preset or default.
    Unset,
}

/// One `settings set` or `unset`.
pub struct SetRequest<'a> {
    /// The extension, as `specforge settings` lists it (`@gsd/core`).
    pub extension: &'a str,
    /// The flat dotted name or the family key, as `specforge.json` writes it.
    pub name: &'a str,
    pub change: SettingChange,
    /// Preview the change and write nothing.
    pub dry_run: bool,
    pub caller: Caller<'a>,
}

/// What `specforge settings set` prints and `specforge.settings_set` answers (`McpSettingsSetResult`).
#[derive(Debug, Clone, PartialEq, Serialize, Shape)]
pub struct SettingsSetDocument {
    pub extension: String,
    pub name: String,
    /// The value `specforge.json` holds now; absent after an unset.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<Value>,
    /// The value it held before; absent when it was not written.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub previous: Option<Value>,
    /// Whether `specforge.json` differs from what it was.
    pub changed: bool,
    pub dry_run: bool,
    /// The file that is, or would be, written.
    pub file: String,
    /// The E086, E087 and W166 the settings of the new file produce.
    pub diagnostics: DiagnosticList,
}

/// A `set` or `unset` done: its document, and what it did to disk.
pub struct SetOutcome {
    pub document: SettingsSetDocument,
    pub wrote: Wrote,
}

fn refused(diagnostic: Diagnostic) -> OpError {
    OpError::of(OpErrorKind::InvalidInput, diagnostic)
}

fn invalid(message: String, suggestion: &str) -> OpError {
    OpError::new(OpErrorKind::InvalidInput, "invalid_input", message).with_suggestion(suggestion)
}

/// The value `words` mean for a setting of type `kind`: one word for a scalar, every word an item of a list. A word
/// that is not the scalar's type is passed on as text, so the form's own E086 says what it should have been.
fn read_words(kind: Option<FieldType>, words: &[String]) -> Result<Value, OpError> {
    let one = || match words {
        [word] => Ok(word.as_str()),
        [] => Err(invalid(
            "no value given".to_string(),
            "give the value, or use `unset` to remove the setting",
        )),
        _ => Err(invalid(
            format!("{} words given for a setting of one value", words.len()),
            "quote the value, or give it as JSON",
        )),
    };
    Ok(match kind {
        Some(FieldType::StringList | FieldType::EnumList | FieldType::ReferenceList) => {
            json!(words)
        }
        Some(FieldType::Block | FieldType::BlockList) => {
            return Err(invalid(
                "a block setting is given as JSON".to_string(),
                "give the value as one JSON document",
            ));
        }
        Some(FieldType::Integer) => {
            let word = one()?;
            word.parse::<i64>()
                .map_or_else(|_| json!(word), |n| json!(n))
        }
        Some(FieldType::Number) => {
            let word = one()?;
            // A whole number stays an integer (`30`, not `30.0`).
            word.parse::<i64>()
                .map(|n| json!(n))
                .ok()
                .or_else(|| {
                    let n = word.parse::<f64>().ok().filter(|n| n.is_finite())?;
                    serde_json::Number::from_f64(n).map(Value::Number)
                })
                .unwrap_or_else(|| json!(word))
        }
        Some(FieldType::Bool) => match one()? {
            "true" => json!(true),
            "false" => json!(false),
            word => json!(word),
        },
        _ => json!(one()?),
    })
}

/// The field of a `set` or `unset` request a refusal can be about (ADR 0072).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsSetField {
    /// `SetRequest::extension`: not enabled, or declaring no settings.
    Extension,
    /// `SetRequest::name`: no setting of the extension, or a secret.
    Name,
    /// `SetRequest::change`: a value its setting's form refuses.
    Value,
}

/// Write the change `request` asks of `view`'s `specforge.json`, or preview it.
pub fn set_setting(
    view: &ProjectView<'_>,
    request: &SetRequest<'_>,
) -> Result<SetOutcome, Refusal<SettingsSetField>> {
    write(view, request).map_err(|error| {
        let declared = view
            .registries()
            .settings()
            .of(request.extension)
            .is_some_and(|d| d.find(request.name).is_some());
        let field = match &*error.code {
            "extension_not_enabled" => SettingsSetField::Extension,
            code if codes::E087.matches(code) => SettingsSetField::Name,
            code if codes::E086.matches(code) && !declared => SettingsSetField::Name,
            _ => SettingsSetField::Value,
        };
        error.about(field)
    })
}

/// Whether `specforge.json` now writes the member the request names (an unreadable or malformed file writes none:
/// the edit itself reports that).
fn is_written(view: &ProjectView<'_>, request: &SetRequest<'_>) -> Result<bool, OpError> {
    let held = request
        .caller
        .containment()
        .resolve(&view.env().sources().path(CONFIG_FILE))?;
    let document = held
        .read_if_there()
        .ok()
        .flatten()
        .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok());
    Ok(document.is_some_and(|document| {
        ["settings", request.extension, request.name]
            .iter()
            .try_fold(&document, |at, key| at.get(*key))
            .is_some()
    }))
}

fn write(view: &ProjectView<'_>, request: &SetRequest<'_>) -> Result<SetOutcome, OpError> {
    let root = view.project_root()?;
    let registry = view.registries().settings();
    let enabled = view.env().enabled();
    if registry.of(request.extension).is_none()
        || !enabled.iter().any(|e| e.name == request.extension)
    {
        return Err(OpError::new(
            OpErrorKind::ExtensionNotFound,
            "extension_not_enabled",
            format!(
                "'{}' is not an enabled extension that declares settings",
                request.extension
            ),
        )
        .with_suggestion(specforge_common::remedy::command(
            "specforge add {}",
            &[request.extension],
        )));
    }
    let kind = registry
        .of(request.extension)
        .and_then(|declared| declared.find(request.name))
        .map(|setting| setting.form.field_type());
    let value = match &request.change {
        SettingChange::Words(words) => Some(read_words(kind, words)?),
        SettingChange::Json(value) => Some(value.clone()),
        SettingChange::Unset => None,
    };
    // shortcut: the value of a rejected secret is never scanned or echoed, only its name is checked; a secret hidden inside a JSON block value is not looked for.
    let written = matches!(request.change, SettingChange::Unset) && is_written(view, request)?;
    let admitted = admit(
        request.extension,
        request.name,
        value.as_ref(),
        written,
        registry,
    )
    .map_err(refused)?;
    if value.is_some() && admitted.is_none() {
        return Err(invalid(
            format!("'{}' cannot be set to null", request.name),
            "use `settings unset` to remove it",
        ));
    }
    config::usable(root)?;
    let path = ["settings", request.extension, request.name];
    let sources = view.env().sources().clone();
    let held = request
        .caller
        .containment()
        .resolve(&sources.path(CONFIG_FILE))?;
    let plan = || -> Result<(Option<Vec<u8>>, String, String), OpError> {
        let before = held.read_if_there().map_err(|e| {
            OpError::new(
                OpErrorKind::of_io(&e),
                crate::FILE_UNREADABLE,
                format!("cannot read {CONFIG_FILE}: {e}"),
            )
        })?;
        let text = String::from_utf8(before.clone().unwrap_or_default()).map_err(|_| {
            OpError::new(
                OpErrorKind::SchemaMismatch,
                "config_invalid",
                format!("{CONFIG_FILE} is not UTF-8 text"),
            )
        })?;
        let after = match &admitted {
            Some(value) => config_edit::set_member(&text, &path, value),
            None => config_edit::remove_member(&text, &path),
        }
        .map_err(|why| {
            OpError::new(
                OpErrorKind::Internal,
                "config_edit_failed",
                format!("{CONFIG_FILE}: {why}"),
            )
        })?;
        // The splice, read as the load reads it, adds no E086 or E087.
        let errors = |text: &str| {
            let (_, found) = ResolvedSettings::resolve(Some(text), registry, enabled);
            found
        };
        let bad = |found: &[Diagnostic]| {
            found
                .iter()
                .filter(|d| d.code == codes::E086.id() || d.code == codes::E087.id())
                .count()
        };
        let now = errors(&after);
        if bad(&now) > bad(&errors(&text)) {
            let first = now
                .into_iter()
                .find(|d| d.code == codes::E086.id() || d.code == codes::E087.id())
                .map(refused);
            return Err(first.unwrap_or_else(|| {
                OpError::new(
                    OpErrorKind::Internal,
                    "config_edit_failed",
                    format!("{CONFIG_FILE}: the edit adds an error"),
                )
            }));
        }
        Ok((before, text, after))
    };
    let planned = std::cell::RefCell::new(None);
    let wrote = if request.dry_run {
        *planned.borrow_mut() = Some(plan()?);
        Wrote::Previewed
    } else {
        let writes = locked::commit(
            &sources,
            "cannot write specforge.json",
            request.caller,
            true,
            || {
                *planned.borrow_mut() = Some(plan()?);
                Ok(())
            },
            |stager| {
                if let Some((before, text, after)) = planned.borrow().as_ref()
                    && after != text
                {
                    stager.write(CONFIG_FILE, before.clone(), after.clone().into_bytes())?;
                }
                Ok(())
            },
        )?;
        Wrote::Ran(writes)
    };
    let (_, text, after) = planned.into_inner().expect("the change was planned");
    let current = |file: &str| -> Option<Value> {
        let document: Value = serde_json::from_str(file).ok()?;
        path.iter()
            .try_fold(&document, |at, key| at.get(*key))
            .cloned()
    };
    let (_, found) = ResolvedSettings::resolve(Some(&after), registry, enabled);
    Ok(SetOutcome {
        document: SettingsSetDocument {
            extension: request.extension.to_string(),
            name: request.name.to_string(),
            value: current(&after),
            // A committed secret is being removed: its value is not repeated.
            previous: current(&text)
                .filter(|_| !is_secret(request.extension, request.name, registry)),
            changed: after != text,
            dry_run: request.dry_run,
            file: CONFIG_FILE.to_string(),
            diagnostics: DiagnosticList::new(found, view.registries().codes()),
        },
        wrote,
    })
}
