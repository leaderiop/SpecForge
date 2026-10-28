//! The baseline (ADR 0287): the project compiled at the commit `HEAD` names, read through the [`History`] port. What
//! a field `immutable` or `append_only` is judged against, and a pass's `previous`. Nothing here runs unless the
//! project declares history ([`declared`]).
//!
//! A baseline is cached by commit and environment: in memory for the process, and as
//! `.specforge/baselines/<sha>-<env16>.json` under the project root, where `env16` is the first 16 hex of the digest
//! of what the compile reads besides the sources (the host version, the loaded declarations and the digest of each installed module, the spec root and the
//! excludes). A different extension version is a different environment, never a stale hit.

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock, Mutex};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use specforge_common::{Diagnostic, codes};
use specforge_package::Sha256;
use specforge_registry::RegistryBuild;
use specforge_wasm::WasmRuntime;

use crate::Environment;
use crate::history::{CommitFile, GitPlace, History, Located, Sha};
use crate::sources::Read;

/// One entity at the baseline: its kind, each field it wrote as the host reads it (`graph::wire`), and the values
/// the compute passes gave it, by written path.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct BaselineEntity {
    pub kind: String,
    pub values: BTreeMap<String, Value>,
    pub computed: BTreeMap<String, Value>,
    /// The path it had at the baseline, when a move record matched it to its current one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub was: Option<String>,
}

/// The project at a commit.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Baseline {
    pub revision: Sha,
    /// By full path.
    pub entities: BTreeMap<String, BaselineEntity>,
    /// The text of `specforge-harness` at the baseline commit: the move records it already held are not applied again.
    #[serde(default)]
    pub ledger: String,
}

/// Whether the loaded extensions declare anything that reads history.
pub fn declared(registries: &RegistryBuild) -> bool {
    fn flagged(form: &specforge_graph::FieldForm) -> bool {
        form.immutable()
            || form.append_only()
            || form
                .members()
                .is_some_and(|m| m.iter().any(|(_, f)| flagged(f)))
    }
    registries
        .fields()
        .iter()
        .any(|(_, _, f)| flagged(f.form()))
        || registries
            .kinds()
            .iter()
            .any(|(_, k)| k.declared.append_only)
        || registries.passes().iter().any(|p| !p.baseline().is_empty())
}

/// What the files a baseline is named by held, in a fixed order: a read's bytes, or why there were none.
pub type RefStamp = Vec<Result<Vec<u8>, std::io::ErrorKind>>;

/// A baseline read and the ref stamp it was read under: while the stamp holds, `HEAD` is the same commit and the
/// read is reused with no process (a session; a typed edit).
#[derive(Debug)]
pub struct HeldBaseline {
    pub stamp: Option<RefStamp>,
    /// A `--since` copy: its baseline is the call's, whatever `HEAD` does.
    pub pinned: bool,
    pub read: BaselineRead,
}

/// What reading the baseline gave: the baseline, none (nothing declares history, or no port is installed), or the
/// notice that says why there is none.
pub type BaselineRead = Result<Option<Arc<Baseline>>, Diagnostic>;

/// A revision the repository does not know.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownRevision(pub String);

impl Baseline {
    /// What the files `HEAD` is named by hold, read through `disk` (a session records each as a ref input): `HEAD`,
    /// the loose ref it names, `packed-refs` and a reftable's list; outside a repository, the missing `.git`
    /// (`git init` changes it). `None` when nothing declares history or no port is installed: nothing is read.
    pub fn ref_stamp(env: &Environment, disk: &dyn specforge_registry::Disk) -> Option<RefStamp> {
        if !declared(env.registries()) {
            return None;
        }
        let history = env.history()?;
        let read = |path: &Path| disk.read_ref(path).map_err(|e| e.kind());
        let Located::Repo(place) = env.place(&*history) else {
            return Some(vec![read(&env.root().join(".git"))]);
        };
        let head = read(&place.git_dir.join("HEAD"));
        let named = head.as_ref().ok().and_then(|bytes| {
            String::from_utf8_lossy(bytes)
                .trim()
                .strip_prefix("ref: ")
                .map(|name| place.common_dir.join(name))
        });
        let mut stamp = vec![head];
        stamp.extend(named.map(|path| read(&path)));
        stamp.push(read(&place.common_dir.join("packed-refs")));
        stamp.push(read(&place.common_dir.join("reftable").join("tables.list")));
        Some(stamp)
    }

    /// The baseline at the merge-base of `revision` and `HEAD` (`check --since`): `Ok(None)` when nothing declares
    /// history or no port is installed, the I204 or W164 that says why there is none, and `UnknownRevision` for a
    /// revision the repository does not name. The revision is resolved even when nothing declares history.
    pub fn since(env: &Environment, revision: &str) -> Result<BaselineRead, UnknownRevision> {
        let Some(history) = env.history() else {
            return Ok(Ok(None));
        };
        let unreadable = |why: String| {
            Diagnostic::new(codes::W164, format!("the baseline cannot be read: {why}"))
        };
        let place = match env.place(&*history) {
            Located::Repo(place) => place,
            Located::NotARepository => {
                return if declared(env.registries()) {
                    Ok(Err(Diagnostic::new(
                        codes::I204,
                        "not in a git repository: nothing is judged against history",
                    )))
                } else {
                    Err(UnknownRevision(revision.to_string()))
                };
            }
            Located::Unavailable(why) => return Ok(Err(unreadable(why))),
        };
        let named = match history.commit(&place, revision) {
            Ok(Some(sha)) => sha,
            Ok(None) => return Err(UnknownRevision(revision.to_string())),
            Err(error) => return Ok(Err(unreadable(error.to_string()))),
        };
        if !declared(env.registries()) {
            return Ok(Ok(None));
        }
        let head = match history.commit(&place, "HEAD") {
            Ok(Some(sha)) => sha,
            Ok(None) => {
                return Ok(Err(Diagnostic::new(
                    codes::I204,
                    "no commit yet: nothing is judged against history",
                )));
            }
            Err(error) => return Ok(Err(unreadable(error.to_string()))),
        };
        let base = match history.merge_base(&place, &named, &head) {
            Ok(Some(sha)) => sha,
            Ok(None) if place.shallow => {
                return Ok(Err(unreadable(format!(
                    "the merge-base of {revision} and HEAD is not in this shallow clone: fetch more history (`fetch-depth: 0`)"
                ))));
            }
            Ok(None) => {
                return Ok(Err(Diagnostic::new(
                    codes::I204,
                    format!(
                        "{revision} and HEAD share no commit: nothing is judged against history"
                    ),
                )));
            }
            Err(error) => return Ok(Err(unreadable(error.to_string()))),
        };
        Ok(Self::at(env, &*history, &place, &base)
            .map(Some)
            .map_err(unreadable))
    }

    /// What a pass that names `kinds` in `baseline` receives as `previous`: the entities of those kinds, each field
    /// as the host reads it, a computed value under its written path.
    pub fn pass_view(&self, kinds: &[String]) -> specforge_protocol_types::PassBaseline {
        specforge_protocol_types::PassBaseline {
            revision: self.revision.as_str().to_string(),
            entities: self
                .entities
                .iter()
                .filter(|(_, e)| kinds.contains(&e.kind))
                .map(|(id, e)| {
                    let mut fields = e.values.clone();
                    fields.extend(e.computed.clone());
                    (
                        id.clone(),
                        specforge_protocol_types::PassBaselineEntity {
                            kind: e.kind.clone(),
                            fields,
                        },
                    )
                })
                .collect(),
        }
    }

    /// This baseline with the move records of `ledger` that the baseline commit's own ledger lacks applied, all at
    /// once (K5 note 1): every entity at or under an old path is at the new one, and every reference value to it is
    /// rewritten. A record that commit already held is from before it. The longest old path wins for a path, and a
    /// path is substituted once, so a renumbering chain (`04` to `05`, `03` to `04`) composes. Only reference-typed
    /// values are rewritten, authored and computed alike.
    pub fn translated(
        &self,
        ledger: &crate::harness::Ledger,
        current: &specforge_graph::Graph,
        forms: &specforge_graph::FieldForms,
    ) -> Baseline {
        let held = crate::harness::Ledger::parse(&self.ledger).ledger;
        let mut moves: Vec<(&str, &str)> = ledger
            .lines()
            .filter(|l| !held.records(&l.path, &l.key, &l.digest))
            .filter_map(|l| l.moved_from().map(|old| (l.path.as_str(), old)))
            .filter(|(new, _)| current.node(new).is_some())
            .collect();
        if moves.is_empty() {
            return self.clone();
        }
        moves.sort_by_key(|(_, old)| std::cmp::Reverse(old.matches('/').count()));
        let rename = |text: &str| -> Option<String> {
            moves.iter().find_map(|(new, old)| {
                if text == *old {
                    Some((*new).to_string())
                } else {
                    text.strip_prefix(old)
                        .and_then(|rest| rest.strip_prefix('/'))
                        .map(|rest| format!("{new}/{rest}"))
                }
            })
        };
        // `value` under `form`, the way `wire::resolved` walks it: only a reference is a path.
        fn rewrite(
            value: &Value,
            form: Option<&specforge_graph::FieldForm>,
            rename: &dyn Fn(&str) -> Option<String>,
        ) -> Value {
            use specforge_protocol_types::FieldType;
            let Some(form) = form else {
                return value.clone();
            };
            let path = |v: &Value| match v {
                Value::String(s) => Value::String(rename(s).unwrap_or_else(|| s.clone())),
                other => other.clone(),
            };
            match (form.field_type(), value) {
                (FieldType::Reference, v) => path(v),
                (FieldType::ReferenceList, Value::Array(items)) => {
                    Value::Array(items.iter().map(path).collect())
                }
                (FieldType::Block, Value::Object(map)) => object(map, form, rename),
                (FieldType::BlockList, Value::Array(items)) => Value::Array(
                    items
                        .iter()
                        .map(|item| match item {
                            Value::Object(map) => object(map, form, rename),
                            other => other.clone(),
                        })
                        .collect(),
                ),
                _ => value.clone(),
            }
        }
        fn object(
            map: &serde_json::Map<String, Value>,
            form: &specforge_graph::FieldForm,
            rename: &dyn Fn(&str) -> Option<String>,
        ) -> Value {
            Value::Object(
                map.iter()
                    .map(|(k, v)| {
                        let member = form.members().and_then(|m| m.get(k));
                        (k.clone(), rewrite(v, member, rename))
                    })
                    .collect(),
            )
        }
        let entities = self
            .entities
            .iter()
            .map(|(path, entity)| {
                let moved = rename(path);
                let mut entity = entity.clone();
                let kind = entity.kind.clone();
                entity.values = entity
                    .values
                    .iter()
                    .map(|(k, v)| {
                        let form = match forms.reading(&kind, k) {
                            specforge_graph::Reading::Declared(f) => Some(f),
                            _ => None,
                        };
                        (k.clone(), rewrite(v, form, &rename))
                    })
                    .collect();
                // a computed value sits under its written path (`checks[0].status`)
                entity.computed = entity
                    .computed
                    .iter()
                    .map(|(k, v)| {
                        let form = match forms
                            .reading_at(&kind, &specforge_graph::field_form::declared_path(k))
                        {
                            specforge_graph::Reading::Declared(f) => Some(f),
                            _ => None,
                        };
                        (k.clone(), rewrite(v, form, &rename))
                    })
                    .collect();
                match moved {
                    Some(new) => (new, entity.moved_from(path)),
                    None => (path.clone(), entity),
                }
            })
            .collect();
        Baseline {
            entities,
            ..self.clone()
        }
    }

    /// The baseline of `env`'s project at `HEAD`: `Ok(None)` when nothing declares history or no history port is
    /// installed, and the I204 or W164 that says why there is none otherwise.
    pub fn of_head(env: &Environment) -> BaselineRead {
        if !declared(env.registries()) {
            return Ok(None);
        }
        let Some(history) = env.history() else {
            return Ok(None);
        };
        let unreadable = |why: String| {
            Diagnostic::new(codes::W164, format!("the baseline cannot be read: {why}"))
                .with_suggestion("nothing is judged against history until git answers")
        };
        let place = match env.place(&*history) {
            Located::Repo(place) => place,
            Located::NotARepository => {
                return Err(Diagnostic::new(
                    codes::I204,
                    "not in a git repository: nothing is judged against history",
                ));
            }
            Located::Unavailable(why) => return Err(unreadable(why)),
        };
        let head = match history.commit(&place, "HEAD") {
            Ok(Some(sha)) => sha,
            Ok(None) => {
                return Err(Diagnostic::new(
                    codes::I204,
                    "no commit yet: nothing is judged against history",
                ));
            }
            Err(error) => return Err(unreadable(error.to_string())),
        };
        Self::at(env, &*history, &place, &head)
            .map(Some)
            .map_err(unreadable)
    }

    /// The baseline at `commit`: from memory or disk when this environment compiled it before, else compiled from
    /// the files the history gives.
    pub fn at(
        env: &Environment,
        history: &dyn History,
        place: &GitPlace,
        commit: &Sha,
    ) -> Result<Arc<Baseline>, String> {
        let env16 = fingerprint(env);
        let key = (env.root().to_path_buf(), commit.clone(), env16.clone());
        if let Some(held) = MEMORY.lock().unwrap_or_else(|e| e.into_inner()).get(&key) {
            return Ok(Arc::clone(held));
        }
        let file = env
            .root()
            .join(CACHE_DIR)
            .join(format!("{}-{env16}.json", commit.as_str()));
        let baseline = match std::fs::read(&file)
            .ok()
            .and_then(|bytes| serde_json::from_slice::<Baseline>(&bytes).ok())
            .filter(|b| &b.revision == commit)
        {
            Some(read) => read,
            None => {
                let root = env.root().canonicalize().map_err(|e| e.to_string())?;
                let within = root.strip_prefix(&place.toplevel).map_err(|_| {
                    "the project is outside the repository's working tree".to_string()
                })?;
                let files = history
                    .files(place, commit, within)
                    .map_err(|e| e.to_string())?;
                let compiled = Self::compile(env, commit.clone(), within, files);
                // shortcut: the cache is best effort and never pruned; prune by age if it grows.
                if let Ok(json) = serde_json::to_vec(&compiled) {
                    let dir = env.root().join(CACHE_DIR);
                    let _ = std::fs::create_dir_all(&dir);
                    // the directory ignores itself, whatever the project's own `.gitignore` says
                    let ignore = dir.join(".gitignore");
                    if !ignore.exists() {
                        let _ = std::fs::write(ignore, "*\n");
                    }
                    let _ = specforge_common::fresh::replace_file(&file, &json);
                }
                compiled
            }
        };
        let baseline = Arc::new(baseline);
        MEMORY
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(key, Arc::clone(&baseline));
        Ok(baseline)
    }

    /// The project of `files` (from the repository's top; `within` is the project's directory there) compiled in
    /// `env`, no history of its own.
    pub fn compile(
        env: &Environment,
        revision: Sha,
        within: &Path,
        files: Vec<CommitFile>,
    ) -> Baseline {
        let mut texts: BTreeMap<String, String> = BTreeMap::new();
        let mut ledger = String::new();
        for (path, bytes) in files {
            let Ok(rest) = path.strip_prefix(within) else {
                continue;
            };
            let key = rest
                .components()
                .map(|c| c.as_os_str().to_string_lossy())
                .collect::<Vec<_>>()
                .join("/");
            if key == crate::harness::LEDGER_FILE {
                ledger = String::from_utf8(bytes).unwrap_or_default();
            } else if key.ends_with(".spec")
                && env.sources().holds_key(&key)
                && let Ok(text) = String::from_utf8(bytes)
            {
                texts.insert(key, text);
            }
        }
        let held: BTreeMap<String, &str> =
            texts.iter().map(|(k, v)| (k.clone(), v.as_str())).collect();
        let keys: Vec<String> = texts.keys().cloned().collect();
        let built = env.build_sources(&keys, &held, |_| Read::Gone);
        let snapshot = env.entity_snapshot(&built.graph);
        let computed = match env.runtime() {
            Some(runtime) if env.registries().compute_passes().next().is_some() => {
                crate::compute::run(
                    env,
                    &snapshot,
                    &**runtime as &dyn WasmRuntime,
                    None,
                    &specforge_graph::Computed::default(),
                    &crate::judged::Judgements::default(),
                )
                .0
            }
            _ => Default::default(),
        };
        let entities = entities_of(
            built.graph.graph(),
            env.registries().field_forms(),
            &computed,
        );
        Baseline {
            revision,
            entities,
            ledger,
        }
    }
}

/// Every entity of `graph`, each field as the host reads it and the values `computed` gave it.
fn entities_of(
    graph: &specforge_graph::Graph,
    forms: &specforge_graph::FieldForms,
    computed: &specforge_graph::Computed,
) -> BTreeMap<String, BaselineEntity> {
    let mut entities = BTreeMap::new();
    for node in graph.nodes() {
        let path = node.id.raw.as_str();
        let mut entity = BaselineEntity {
            kind: node.kind.raw.to_string(),
            ..Default::default()
        };
        for entry in node.fields.entries() {
            let key = entry.key.as_str();
            if let Some(value) = specforge_graph::wire::resolved(graph, forms, node, key) {
                entity
                    .values
                    .insert(key.to_string(), value.as_value().clone());
            }
        }
        for (field, value) in computed.of(path) {
            entity
                .computed
                .insert(field.to_string(), value.as_value().clone());
        }
        entities.insert(path.to_string(), entity);
    }
    entities
}

impl Baseline {
    /// The project `project` as it is on disk, as the baseline a guard pass reads (`revision: "working"`, ADR 0289):
    /// the project before the write being judged.
    pub fn of_project(project: &crate::CompiledProject) -> Baseline {
        Baseline {
            revision: Sha::new("working"),
            entities: entities_of(
                project.graph(),
                project.environment().registries().field_forms(),
                project.computed(),
            ),
            ledger: String::new(),
        }
    }
}

const CACHE_DIR: &str = ".specforge/baselines";

type Key = (PathBuf, Sha, String);
static MEMORY: LazyLock<Mutex<HashMap<Key, Arc<Baseline>>>> = LazyLock::new(Default::default);

/// The first 16 hex of the digest of what a compile reads besides the sources.
fn fingerprint(env: &Environment) -> String {
    let mut bytes = env!("CARGO_PKG_VERSION").as_bytes().to_vec();
    for declaration in env.registries().declarations() {
        bytes.push(0);
        bytes.extend(serde_json::to_vec(declaration).unwrap_or_default());
    }
    // the digest of each installed module: a rebuilt extension with the same name and version is another environment
    for entry in env.installed().lock().entries() {
        bytes.push(0);
        bytes.extend(entry.name.to_string().into_bytes());
        bytes.extend(entry.wasm_hash.as_str().as_bytes());
    }
    let config = env.config();
    bytes.push(0);
    bytes.extend(config.spec_root.clone().unwrap_or_default().into_bytes());
    for entry in config.exclude.entries() {
        bytes.push(0);
        bytes.extend(entry.written().as_bytes());
    }
    Sha256::of(&bytes).to_string()[..16].to_string()
}
