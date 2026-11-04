//! The effects `specforge.json` allows and the roots it names (ADR 0335 D2.3).
//!
//! [`ResolvedEffects::resolve`] is pure over the config text and the loaded declarations: the `effects` ceiling, the
//! `roots` as written, E109 for what is not used as written and W171 per declared capability the ceiling narrows,
//! each spanned at its key. [`ResolvedEffects::locate`] resolves the roots against the disk once, at environment
//! load: each to its canonical target, inside or outside the project (E109 when it is no directory). Either key of
//! the wrong type is E069, read where the config is.

use std::path::{Path, PathBuf};

use serde_json::{Map, Value};
use specforge_common::json_span::{self, JsonIndex};
use specforge_common::{Diagnostic, SourceSpan, codes};
use specforge_effects::{EffectsCeiling, FsCeiling, GitCeiling, HttpCeiling, Narrow, Roots};
use specforge_protocol_types::{
    ExtensionDeclaration, FsAccess, GitOp, HttpMethod, is_host_pattern, is_scope_name,
};

use crate::Environment;

/// A root `specforge.json` `roots` names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NamedRoot {
    pub name: String,
    /// As written, relative to the project root.
    pub written: String,
    /// Its canonical target, once [`ResolvedEffects::locate`] found it a directory.
    pub target: Option<PathBuf>,
    /// The target lies under the canonical project root.
    pub inside: bool,
}

/// The effects a project allows and the roots it names.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ResolvedEffects {
    pub ceiling: EffectsCeiling,
    pub roots: Vec<NamedRoot>,
    /// E109 and W171, in file order.
    pub diagnostics: Vec<Diagnostic>,
}

/// The roots every project has: never named in `roots`.
const RESERVED: [&str; 2] = ["spec", "project"];

/// The config text, indexed once, and the E109s found.
struct Reader<'a> {
    text: &'a str,
    index: JsonIndex,
    found: Vec<Diagnostic>,
}

impl Reader<'_> {
    fn span(&self, path: &[&str]) -> Option<SourceSpan> {
        span_at(self.text, &self.index, path)
    }

    fn e109(&mut self, path: &[&str], why: impl Into<String>) {
        let mut diagnostic = Diagnostic::new(
            codes::E109,
            format!(
                "`{}` in specforge.json is not used: {}",
                path.join("."),
                why.into()
            ),
        );
        if let Some(span) = self.span(path) {
            diagnostic = diagnostic.with_span(span);
        }
        self.found.push(diagnostic);
    }

    /// The member `member` of `effects.<port>`, a list each item of which `parse` takes; an item it refuses is
    /// E109 and left out. Absent: `None` (not narrowed).
    fn list<T>(
        &mut self,
        members: &Map<String, Value>,
        port: &str,
        member: &str,
        parse: impl Fn(&str) -> Result<T, String>,
    ) -> Option<Vec<T>> {
        let value = members.get(member)?;
        let path = ["effects", port, member];
        let Some(items) = value.as_array() else {
            self.e109(&path, "write a list of strings");
            return None;
        };
        let mut kept = Vec::new();
        for item in items {
            match item.as_str().map(&parse) {
                Some(Ok(value)) => kept.push(value),
                Some(Err(why)) => self.e109(&path, why),
                None => self.e109(&path, "write a list of strings"),
            }
        }
        Some(kept)
    }

    fn port(&mut self, ceiling: &mut EffectsCeiling, port: &str, value: &Value) {
        let path = ["effects", port];
        let known: &[&str] = match port {
            "http" => &["hosts", "schemes", "methods"],
            "fs" => &["roots", "ops"],
            "git" => &["ops"],
            "process" => &[],
            _ => {
                self.e109(
                    &path,
                    format!("no port is named '{port}' (http, fs, git, process)"),
                );
                return;
            }
        };
        if value.as_str() == Some("off") {
            match port {
                "http" => ceiling.http = Narrow::Off,
                "fs" => ceiling.fs = Narrow::Off,
                "git" => ceiling.git = Narrow::Off,
                _ => ceiling.process = Narrow::Off,
            }
            return;
        }
        if port == "process" {
            self.e109(
                &path,
                "a command runs as declared or not at all: write \"off\"",
            );
            return;
        }
        let Value::Object(members) = value else {
            self.e109(&path, "a port is \"off\" or an object");
            return;
        };
        for member in members.keys().filter(|m| !known.contains(&m.as_str())) {
            self.e109(
                &["effects", port, member],
                format!("'{port}' takes {}", known.join(", ")),
            );
        }
        match port {
            "http" => {
                let hosts = self.list(members, port, "hosts", |h| {
                    if is_host_pattern(h) {
                        Ok(h.to_string())
                    } else {
                        Err(format!("'{h}' is not a host or a `*.`-prefixed host"))
                    }
                });
                let schemes = self.list(members, port, "schemes", |s| match s {
                    "http" | "https" => Ok(s.to_string()),
                    _ => Err(format!("'{s}' is neither http nor https")),
                });
                let methods = self.list(members, port, "methods", |m| {
                    HttpMethod::parse(m).ok_or_else(|| format!("'{m}' is no HTTP method"))
                });
                ceiling.http = Narrow::Only(HttpCeiling {
                    hosts,
                    schemes,
                    methods,
                });
            }
            "fs" => {
                let roots = self.list(members, port, "roots", |r| {
                    if is_scope_name(r) {
                        Ok(r.to_string())
                    } else {
                        Err(format!("'{r}' is not a root name"))
                    }
                });
                let ops = self.list(members, port, "ops", |o| {
                    fs_access(o)
                        .ok_or_else(|| format!("'{o}' is no file op (stat, read, walk, write)"))
                });
                ceiling.fs = Narrow::Only(FsCeiling { roots, ops });
            }
            _ => {
                let ops = self.list(members, port, "ops", |o| {
                    git_op(o).ok_or_else(|| format!("'{o}' is no git op (read, write)"))
                });
                ceiling.git = Narrow::Only(GitCeiling { ops });
            }
        }
    }

    fn roots(&mut self, named: &Map<String, Value>) -> Vec<NamedRoot> {
        let mut roots = Vec::new();
        for (name, path) in named {
            let at = ["roots", name.as_str()];
            if RESERVED.contains(&name.as_str()) {
                self.e109(&at, format!("'{name}' is built in"));
            } else if !is_scope_name(name) {
                self.e109(&at, format!("'{name}' is not a word ([a-z][a-z0-9-]*)"));
            } else if let Some(written) = path.as_str().filter(|p| !p.trim().is_empty()) {
                roots.push(NamedRoot {
                    name: name.clone(),
                    written: written.to_string(),
                    target: None,
                    inside: false,
                });
            } else {
                self.e109(&at, "a root is a non-empty path");
            }
        }
        roots
    }
}

fn span_at(text: &str, index: &JsonIndex, path: &[&str]) -> Option<SourceSpan> {
    let key = index.locate(path)?.key.clone()?;
    Some(json_span::span(text, "specforge.json", key))
}

impl ResolvedEffects {
    /// `config_text`'s `effects` and `roots`, and the W171 of each sound capability `declarations` declare that the
    /// ceiling narrows. A member or item not used as written is E109 and ignored; the rest is kept.
    pub fn resolve(config_text: Option<&str>, declarations: &[ExtensionDeclaration]) -> Self {
        let mut resolved = ResolvedEffects::default();
        let Some(text) = config_text else {
            return resolved;
        };
        let Ok(Value::Object(root)) = serde_json::from_str::<Value>(text) else {
            return resolved;
        };
        let mut reader = Reader {
            text,
            index: json_span::index(text),
            found: Vec::new(),
        };
        if let Some(Value::Object(ports)) = root.get("effects") {
            for (port, value) in ports {
                reader.port(&mut resolved.ceiling, port, value);
            }
        }
        if let Some(Value::Object(named)) = root.get("roots") {
            resolved.roots = reader.roots(named);
        }
        for declaration in declarations {
            for capability in declaration.sound_capabilities() {
                let narrowed = resolved.ceiling.narrow(&capability.port);
                let Some(first) = narrowed.removed.first() else {
                    continue;
                };
                let removed: Vec<&str> = narrowed.removed.iter().map(|r| r.what.as_str()).collect();
                let nothing = if narrowed.scope.is_none() {
                    "; it grants nothing"
                } else {
                    ""
                };
                let mut diagnostic = Diagnostic::new(
                    codes::W171,
                    format!(
                        "capability '{}' of '{}' is narrowed by the project's effects ceiling: {}{nothing}",
                        capability.name,
                        declaration.name(),
                        removed.join("; ")
                    ),
                );
                let key: Vec<&str> = first.key.split('.').collect();
                if let Some(span) = reader.span(&key) {
                    diagnostic = diagnostic.with_span(span);
                }
                reader.found.push(diagnostic);
            }
        }
        resolved.diagnostics = reader.found;
        resolved
    }

    /// The roots resolved against the project at `project`: each to its canonical target, inside or outside the
    /// project. A root that is no directory is E109 and dropped.
    pub fn locate(&mut self, config_text: Option<&str>, project: &Path) {
        let canonical = project
            .canonicalize()
            .unwrap_or_else(|_| project.to_path_buf());
        let mut missing = Vec::new();
        self.roots
            .retain_mut(|root| match project.join(&root.written).canonicalize() {
                Ok(target) if target.is_dir() => {
                    root.inside = target.starts_with(&canonical);
                    root.target = Some(target);
                    true
                }
                _ => {
                    missing.push((root.name.clone(), root.written.clone()));
                    false
                }
            });
        let index = config_text.map(json_span::index);
        for (name, written) in missing {
            let mut diagnostic = Diagnostic::new(
                codes::E109,
                format!(
                    "`roots.{name}` in specforge.json is not used: '{written}' is no directory"
                ),
            );
            if let (Some(text), Some(index)) = (config_text, &index)
                && let Some(span) = span_at(text, index, &["roots", &name])
            {
                diagnostic = diagnostic.with_span(span);
            }
            self.diagnostics.push(diagnostic);
        }
    }

    /// `roots` with every named root that resolved inside the project: what a reader asks no machine's approval for
    /// (O5). A root outside the project is not among them, so a path naming it is refused like an unknown root.
    pub fn with_inside(&self, mut roots: Roots) -> Roots {
        for root in self.roots.iter().filter(|root| root.inside) {
            if let Some(target) = &root.target {
                roots = roots.with_named(root.name.clone(), target.clone());
            }
        }
        roots
    }

    /// `roots` with every named root that resolved.
    pub fn with_named(&self, mut roots: Roots) -> Roots {
        for root in &self.roots {
            if let Some(target) = &root.target {
                roots = roots.with_named(root.name.clone(), target.clone());
            }
        }
        roots
    }
}

fn fs_access(op: &str) -> Option<FsAccess> {
    match op {
        "stat" => Some(FsAccess::Stat),
        "read" => Some(FsAccess::Read),
        "walk" => Some(FsAccess::Walk),
        "write" => Some(FsAccess::Write),
        _ => None,
    }
}

fn git_op(op: &str) -> Option<GitOp> {
    match op {
        "read" => Some(GitOp::Read),
        "write" => Some(GitOp::Write),
        _ => None,
    }
}

impl Environment {
    /// The effects `specforge.json` allows, the roots it names (located, for an environment loaded at a project
    /// root) and their E109 and W171 (ADR 0335 D2.3).
    pub fn effects_allowed(&self) -> &ResolvedEffects {
        &self.effects_allowed
    }

    /// The roots a path in a value may name with no approval (ADR 0336 D6): the project's, the spec root and the named
    /// roots inside the project.
    pub fn read_roots(&self) -> Roots {
        self.effects_allowed
            .with_inside(Roots::new(self.root(), self.spec_root()))
    }
}

#[cfg(test)]
mod tests;
