//! What each kind of scope means to the host (ADR 0335 D2): the port it is on, what the project's ceiling leaves of
//! it, whether a stored one covers another, its line for a person and the requests it admits. A new scope is one
//! [`ScopeRules`] and one arm of [`rules`], the one dispatch over `PortScope` these answers go through.

use specforge_protocol_types::{
    EffectRequest, FsAccess, FsOp, FsScope, GitScope, Hosts, HttpScope, PortScope, ProcessScope,
};
use url::Url;

use crate::ceiling::{DEFAULT_ROOTS, EffectsCeiling, Narrow, Removed, host_covers};
use crate::gate::Port;

/// What one kind of scope means.
pub trait ScopeRules {
    /// The port it is on.
    fn port(&self) -> Port;
    /// What `ceiling` leaves of it, noting in `removed` what went; `None`: nothing is left.
    fn narrowed(&self, ceiling: &EffectsCeiling, removed: &mut Vec<Removed>) -> Option<PortScope>;
    /// Whether it, stored, allows everything `current` does.
    fn covers(&self, current: &PortScope) -> bool;
    /// One line for a person: `http GET,HEAD https *.example.com`, `process cargo test`.
    fn summary(&self) -> String;
    /// Whether it admits `request`. Under the user's grant an fs scope with no roots reaches any root and a process
    /// scope with no argv runs any command.
    fn admits(&self, request: &EffectRequest, user: bool) -> bool;
}

/// `scope`'s rules.
pub fn rules(scope: &PortScope) -> &dyn ScopeRules {
    match scope {
        PortScope::Http(http) => http,
        PortScope::Fs(fs) => fs,
        PortScope::Process(process) => process,
        PortScope::Git(git) => git,
    }
}

impl ScopeRules for HttpScope {
    fn port(&self) -> Port {
        Port::Http
    }

    fn narrowed(&self, ceiling: &EffectsCeiling, removed: &mut Vec<Removed>) -> Option<PortScope> {
        let ceiling = match &ceiling.http {
            Narrow::Open => return Some(PortScope::Http(self.clone())),
            Narrow::Off => return off("http", removed),
            Narrow::Only(ceiling) => ceiling,
        };
        let mut http = self.clone();
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
                removed,
            );
        }
        if let Some(methods) = &ceiling.methods {
            left &= meet(
                &mut http.methods,
                methods,
                "effects.http.methods",
                "methods",
                removed,
            );
        }
        left.then_some(PortScope::Http(http))
    }

    fn covers(&self, current: &PortScope) -> bool {
        let PortScope::Http(current) = current else {
            return false;
        };
        let hosts = match (&self.hosts, &current.hosts) {
            (Hosts::FromValues, _) => true,
            (Hosts::List(_), Hosts::FromValues) => false,
            (Hosts::List(s), Hosts::List(c)) => {
                c.iter().all(|c| s.iter().any(|s| host_covers(s, c)))
            }
        };
        hosts
            && subset(&self.schemes, &current.schemes)
            && subset(&self.methods, &current.methods)
            && self.secret_header == current.secret_header
    }

    fn summary(&self) -> String {
        let hosts = match &self.hosts {
            Hosts::List(hosts) => hosts.join(","),
            Hosts::FromValues => "the hosts the project's values name".to_string(),
        };
        let methods = if self.methods.is_empty() {
            "any method".to_string()
        } else {
            self.methods
                .iter()
                .map(|m| m.as_str())
                .collect::<Vec<_>>()
                .join(",")
        };
        let schemes = if self.schemes.is_empty() {
            "http,https".to_string()
        } else {
            self.schemes.join(",")
        };
        format!("http {methods} {schemes} {hosts}")
    }

    fn admits(&self, request: &EffectRequest, _user: bool) -> bool {
        // A name's addresses and a handshake are asked of a host the scope lists; a handshake is `https`'s.
        let reached = |host: &str| match &self.hosts {
            Hosts::FromValues => !host.is_empty(),
            Hosts::List(hosts) => hosts.iter().any(|h| host_covers(h, host)),
        };
        let request = match request {
            EffectRequest::Http(request) => request,
            EffectRequest::Resolve(asked) => return reached(&asked.host),
            EffectRequest::Tls(asked) => {
                let https = self.schemes.is_empty()
                    || self.schemes.iter().any(|s| s.eq_ignore_ascii_case("https"));
                return https && reached(&asked.host);
            }
            EffectRequest::Process(_) | EffectRequest::Fs(_) => return false,
        };
        // The parser the adapter requests with (WHATWG): what is judged is what is sent.
        let Ok(url) = Url::parse(&request.url) else {
            return false;
        };
        let host = url.host_str().unwrap_or_default();
        let schemes_ok = if self.schemes.is_empty() {
            matches!(url.scheme(), "http" | "https")
        } else {
            self.schemes
                .iter()
                .any(|s| s.eq_ignore_ascii_case(url.scheme()))
        };
        let method_ok = self.methods.is_empty()
            || self
                .methods
                .iter()
                .any(|m| m.as_str().eq_ignore_ascii_case(&request.method));
        schemes_ok && method_ok && reached(host)
    }
}

impl ScopeRules for FsScope {
    fn port(&self) -> Port {
        Port::Fs
    }

    fn narrowed(&self, ceiling: &EffectsCeiling, removed: &mut Vec<Removed>) -> Option<PortScope> {
        let ceiling = match &ceiling.fs {
            Narrow::Open => return Some(PortScope::Fs(self.clone())),
            Narrow::Off => return off("fs", removed),
            Narrow::Only(ceiling) => ceiling,
        };
        let mut fs = self.clone();
        let mut left = true;
        if let Some(roots) = &ceiling.roots {
            if fs.roots.is_empty() {
                fs.roots = DEFAULT_ROOTS.map(String::from).to_vec();
            }
            left &= meet(&mut fs.roots, roots, "effects.fs.roots", "roots", removed);
        }
        if let Some(ops) = &ceiling.ops {
            left &= meet(&mut fs.ops, ops, "effects.fs.ops", "ops", removed);
        }
        left.then_some(PortScope::Fs(fs))
    }

    fn covers(&self, current: &PortScope) -> bool {
        let PortScope::Fs(current) = current else {
            return false;
        };
        let roots = |roots: &[String]| {
            if roots.is_empty() {
                DEFAULT_ROOTS.map(String::from).to_vec()
            } else {
                roots.to_vec()
            }
        };
        subset(&roots(&self.roots), &roots(&current.roots)) && subset(&self.ops, &current.ops)
    }

    fn summary(&self) -> String {
        format!(
            "fs {} in {}",
            lowercase_list(&self.ops),
            if self.roots.is_empty() {
                "spec,project".to_string()
            } else {
                self.roots.join(",")
            }
        )
    }

    fn admits(&self, request: &EffectRequest, user: bool) -> bool {
        let EffectRequest::Fs(request) = request else {
            return false;
        };
        let root_ok = if self.roots.is_empty() {
            user || DEFAULT_ROOTS.contains(&request.root.as_str())
        } else {
            self.roots.contains(&request.root)
        };
        let op = match request.op {
            FsOp::Stat => FsAccess::Stat,
            FsOp::Read { .. } => FsAccess::Read,
            FsOp::Walk { .. } => FsAccess::Walk,
        };
        root_ok && self.ops.contains(&op)
    }
}

impl ScopeRules for ProcessScope {
    fn port(&self) -> Port {
        Port::Process
    }

    fn narrowed(&self, ceiling: &EffectsCeiling, removed: &mut Vec<Removed>) -> Option<PortScope> {
        match ceiling.process {
            Narrow::Off => off("process", removed),
            // A process is all or nothing: `Only` is never written.
            Narrow::Open | Narrow::Only(()) => Some(PortScope::Process(self.clone())),
        }
    }

    fn covers(&self, current: &PortScope) -> bool {
        matches!(current, PortScope::Process(current) if self == current)
    }

    fn summary(&self) -> String {
        format!("process {}", self.argv.join(" "))
    }

    fn admits(&self, request: &EffectRequest, user: bool) -> bool {
        let EffectRequest::Process(request) = request else {
            return false;
        };
        (user && self.argv.is_empty()) || argv_matches(&self.argv, &request.argv)
    }
}

impl ScopeRules for GitScope {
    fn port(&self) -> Port {
        Port::Git
    }

    fn narrowed(&self, ceiling: &EffectsCeiling, removed: &mut Vec<Removed>) -> Option<PortScope> {
        let ceiling = match &ceiling.git {
            Narrow::Open => return Some(PortScope::Git(self.clone())),
            Narrow::Off => return off("git", removed),
            Narrow::Only(ceiling) => ceiling,
        };
        let mut git = self.clone();
        let left = match &ceiling.ops {
            Some(ops) => meet(&mut git.ops, ops, "effects.git.ops", "ops", removed),
            None => true,
        };
        left.then_some(PortScope::Git(git))
    }

    fn covers(&self, current: &PortScope) -> bool {
        matches!(current, PortScope::Git(current) if subset(&self.ops, &current.ops))
    }

    fn summary(&self) -> String {
        format!("git {}", lowercase_list(&self.ops))
    }

    // `EffectRequest` has no git request.
    fn admits(&self, _request: &EffectRequest, _user: bool) -> bool {
        false
    }
}

/// Whether a stored list allows everything `current` does: empty means any.
fn subset<T: PartialEq>(stored: &[T], current: &[T]) -> bool {
    stored.is_empty() || (!current.is_empty() && current.iter().all(|c| stored.contains(c)))
}

fn lowercase_list<T: std::fmt::Debug>(items: &[T]) -> String {
    if items.is_empty() {
        return "no op".to_string();
    }
    items
        .iter()
        .map(|item| format!("{item:?}").to_lowercase())
        .collect::<Vec<_>>()
        .join(",")
}

/// Whether `argv` is `template` with each `{report}` replaced.
fn argv_matches(template: &[String], argv: &[String]) -> bool {
    template.len() == argv.len()
        && template
            .iter()
            .zip(argv)
            .all(|(t, a)| match t.split_once("{report}") {
                None => t == a,
                Some((before, after)) => {
                    a.len() >= before.len() + after.len()
                        && a.starts_with(before)
                        && a.ends_with(after)
                }
            })
}

fn off(port: &str, removed: &mut Vec<Removed>) -> Option<PortScope> {
    removed.push(Removed {
        key: format!("effects.{port}"),
        what: format!("the {port} port is off"),
    });
    None
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
