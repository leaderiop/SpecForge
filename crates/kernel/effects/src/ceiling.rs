//! The project's effects ceiling (ADR 0335 D2.3): what `specforge.json` `effects` leaves of each port. It can only
//! narrow a declared capability, never widen one, and it is never an approval: the file is committed.

use specforge_protocol_types::{FsAccess, GitOp, Hosts, HttpMethod, PortScope};

/// What the ceiling leaves of one port.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum Narrow<T> {
    /// Not narrowed.
    #[default]
    Open,
    /// The port is switched off.
    Off,
    /// Only what it lists.
    Only(T),
}

/// `effects.http`: each list kept, when written, is all that is left.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HttpCeiling {
    pub hosts: Option<Vec<String>>,
    pub schemes: Option<Vec<String>>,
    pub methods: Option<Vec<HttpMethod>>,
}

/// `effects.fs`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FsCeiling {
    pub roots: Option<Vec<String>>,
    pub ops: Option<Vec<FsAccess>>,
}

/// `effects.git`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GitCeiling {
    pub ops: Option<Vec<GitOp>>,
}

/// The project's effects ceiling. The default narrows nothing.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EffectsCeiling {
    pub http: Narrow<HttpCeiling>,
    pub fs: Narrow<FsCeiling>,
    pub git: Narrow<GitCeiling>,
    /// A process is all or nothing: `Only` is never written.
    pub process: Narrow<()>,
}

/// What the ceiling left of a declared scope.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Narrowed {
    /// `None`: nothing is left, the capability grants nothing.
    pub scope: Option<PortScope>,
    /// What it removed, each with the `effects` key that removed it (`effects.http.hosts`).
    pub removed: Vec<Removed>,
}

/// One thing the ceiling removed from a scope.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Removed {
    /// The ceiling's key: `effects.process`, `effects.http.hosts`.
    pub key: String,
    /// What it took, for a person.
    pub what: String,
}

/// The roots a file scope that names none reaches.
pub const DEFAULT_ROOTS: [&str; 2] = ["spec", "project"];

impl EffectsCeiling {
    /// What it leaves of `scope`.
    pub fn narrow(&self, scope: &PortScope) -> Narrowed {
        let mut removed = Vec::new();
        let off = |port: &str, removed: &mut Vec<Removed>| {
            removed.push(Removed {
                key: format!("effects.{port}"),
                what: format!("the {port} port is off"),
            });
        };
        let scope = match (scope, self) {
            (
                PortScope::Http(_),
                EffectsCeiling {
                    http: Narrow::Off, ..
                },
            )
            | (
                PortScope::Fs(_),
                EffectsCeiling {
                    fs: Narrow::Off, ..
                },
            )
            | (
                PortScope::Git(_),
                EffectsCeiling {
                    git: Narrow::Off, ..
                },
            )
            | (
                PortScope::Process(_),
                EffectsCeiling {
                    process: Narrow::Off,
                    ..
                },
            ) => {
                off(scope.port(), &mut removed);
                None
            }
            (
                PortScope::Http(http),
                EffectsCeiling {
                    http: Narrow::Only(ceiling),
                    ..
                },
            ) => {
                let mut http = http.clone();
                let mut left = true;
                if let Some(hosts) = &ceiling.hosts {
                    let kept = match &http.hosts {
                        Hosts::FromValues => hosts.clone(),
                        Hosts::List(declared) => meet_hosts(declared, hosts),
                    };
                    if http.hosts != Hosts::List(kept.clone()) {
                        removed.push(Removed {
                            key: "effects.http.hosts".into(),
                            what: format!("hosts other than {}", or_none(&kept)),
                        });
                    }
                    left &= !kept.is_empty();
                    http.hosts = Hosts::List(kept);
                }
                if let Some(schemes) = &ceiling.schemes {
                    left &= meet(
                        &mut http.schemes,
                        schemes,
                        "effects.http.schemes",
                        "schemes",
                        &mut removed,
                    );
                }
                if let Some(methods) = &ceiling.methods {
                    left &= meet(
                        &mut http.methods,
                        methods,
                        "effects.http.methods",
                        "methods",
                        &mut removed,
                    );
                }
                left.then_some(PortScope::Http(http))
            }
            (
                PortScope::Fs(fs),
                EffectsCeiling {
                    fs: Narrow::Only(ceiling),
                    ..
                },
            ) => {
                let mut fs = fs.clone();
                let mut left = true;
                if let Some(roots) = &ceiling.roots {
                    if fs.roots.is_empty() {
                        fs.roots = DEFAULT_ROOTS.map(String::from).to_vec();
                    }
                    left &= meet(
                        &mut fs.roots,
                        roots,
                        "effects.fs.roots",
                        "roots",
                        &mut removed,
                    );
                }
                if let Some(ops) = &ceiling.ops {
                    left &= meet(&mut fs.ops, ops, "effects.fs.ops", "ops", &mut removed);
                }
                left.then_some(PortScope::Fs(fs))
            }
            (
                PortScope::Git(git),
                EffectsCeiling {
                    git: Narrow::Only(ceiling),
                    ..
                },
            ) => {
                let mut git = git.clone();
                let left = match &ceiling.ops {
                    Some(ops) => meet(&mut git.ops, ops, "effects.git.ops", "ops", &mut removed),
                    None => true,
                };
                left.then_some(PortScope::Git(git))
            }
            _ => Some(scope.clone()),
        };
        Narrowed { scope, removed }
    }
}

/// Keep of `declared` what `ceiling` lists (all of `ceiling` when `declared` is empty, which means any); note what
/// went. False when nothing is left.
fn meet<T: Clone + PartialEq + std::fmt::Debug>(
    declared: &mut Vec<T>,
    ceiling: &[T],
    key: &str,
    what: &str,
    removed: &mut Vec<Removed>,
) -> bool {
    let kept: Vec<T> = if declared.is_empty() {
        ceiling.to_vec()
    } else {
        declared
            .iter()
            .filter(|d| ceiling.contains(d))
            .cloned()
            .collect()
    };
    if kept != *declared {
        let gone: Vec<String> = if declared.is_empty() {
            vec![format!("{what} other than {}", or_none(&debug_list(&kept)))]
        } else {
            debug_list(
                &declared
                    .iter()
                    .filter(|d| !kept.contains(d))
                    .cloned()
                    .collect::<Vec<_>>(),
            )
        };
        removed.push(Removed {
            key: key.to_string(),
            what: gone.join(", "),
        });
    }
    *declared = kept;
    !declared.is_empty()
}

fn debug_list<T: std::fmt::Debug>(items: &[T]) -> Vec<String> {
    items
        .iter()
        .map(|item| format!("{item:?}").trim_matches('"').to_lowercase())
        .collect()
}

fn or_none(items: &[String]) -> String {
    if items.is_empty() {
        "none".to_string()
    } else {
        items.join(", ")
    }
}

/// The host patterns both `declared` and `ceiling` allow: a declared pattern a ceiling one covers, or a ceiling
/// pattern a declared one covers.
fn meet_hosts(declared: &[String], ceiling: &[String]) -> Vec<String> {
    let mut kept: Vec<String> = Vec::new();
    for d in declared {
        for c in ceiling {
            let both = if host_covers(c, d) {
                Some(d)
            } else if host_covers(d, c) {
                Some(c)
            } else {
                None
            };
            if let Some(host) = both.filter(|h| !kept.contains(h)) {
                kept.push(host.clone());
            }
        }
    }
    kept
}

/// Whether host pattern `pattern` allows every host `other` (a host or a pattern) allows, in any case:
/// `*.example.com` covers `a.example.com` and `*.a.example.com`, not `example.com`.
pub fn host_covers(pattern: &str, other: &str) -> bool {
    let (pattern, other) = (pattern.to_ascii_lowercase(), other.to_ascii_lowercase());
    match pattern.strip_prefix("*.") {
        Some(suffix) => match other.strip_prefix("*.") {
            Some(under) => under == suffix || under.ends_with(&format!(".{suffix}")),
            None => other.ends_with(&format!(".{suffix}")),
        },
        None => pattern == other,
    }
}
