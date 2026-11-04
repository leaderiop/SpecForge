//! What an extension declares it may make the host do (ADR 0335 D2): a capability names a scope, says why, the call
//! kinds it is for and what it reaches on one port. The host gates it by call kind, the project's effects ceiling
//! narrows it and the machine approves it; a capability with a problem ([`capability_problems`], W021) grants
//! nothing.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::{ExtensionDeclaration, Operation, SettingsItem};

/// One declared capability. On the wire it is one flat object, its port's keys beside `port` (a [`Wire`]): a
/// flattened, internally tagged enum would cost every guest about 40 KB of serde code.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "Wire", into = "Wire")]
pub struct CapabilityDescriptor {
    /// The scope name: `[a-z][a-z0-9-]*`, unique in the extension. Approvals and the CLI name it.
    pub name: String,
    /// Why the extension needs it: shown when asking and listing.
    pub why: String,
    /// The call kinds it is for; empty: every kind the host's gate allows for its port.
    pub call_kinds: Vec<Operation>,
    pub port: PortScope,
}

/// A [`CapabilityDescriptor`] as the wire carries it.
#[derive(Clone, Serialize, Deserialize)]
struct Wire {
    name: String,
    why: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    call_kinds: Vec<Operation>,
    port: PortName,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    schemes: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    methods: Vec<HttpMethod>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    hosts: Option<Hosts>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    secret_header: Option<SecretHeader>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    roots: Vec<String>,
    /// An fs capability's [`FsAccess`]es, or a git one's `read` and `write`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    ops: Option<Vec<FsAccess>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    argv: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    deadline_ms: Option<u32>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    env: Vec<String>,
}

#[derive(Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum PortName {
    Http,
    Fs,
    Process,
    Git,
}

impl TryFrom<Wire> for CapabilityDescriptor {
    type Error = String;

    fn try_from(wire: Wire) -> Result<Self, String> {
        let port = match wire.port {
            PortName::Http => PortScope::Http(HttpScope {
                schemes: wire.schemes,
                methods: wire.methods,
                hosts: wire.hosts.unwrap_or_default(),
                secret_header: wire.secret_header,
            }),
            PortName::Fs => PortScope::Fs(FsScope {
                roots: wire.roots,
                ops: wire.ops.unwrap_or_default(),
            }),
            PortName::Process => PortScope::Process(ProcessScope {
                argv: wire.argv.unwrap_or_default(),
                deadline_ms: wire.deadline_ms,
                env: wire.env,
            }),
            PortName::Git => PortScope::Git(GitScope {
                ops: wire
                    .ops
                    .unwrap_or_default()
                    .into_iter()
                    .map(|op| match op {
                        FsAccess::Read => Ok(GitOp::Read),
                        FsAccess::Write => Ok(GitOp::Write),
                        other => Err(format!("git op `{other:?}` is neither `read` nor `write`")),
                    })
                    .collect::<Result<_, _>>()?,
            }),
        };
        Ok(CapabilityDescriptor {
            name: wire.name,
            why: wire.why,
            call_kinds: wire.call_kinds,
            port,
        })
    }
}

impl From<CapabilityDescriptor> for Wire {
    fn from(capability: CapabilityDescriptor) -> Self {
        let mut wire = Wire {
            name: capability.name,
            why: capability.why,
            call_kinds: capability.call_kinds,
            port: PortName::Http,
            schemes: Vec::new(),
            methods: Vec::new(),
            hosts: None,
            secret_header: None,
            roots: Vec::new(),
            ops: None,
            argv: None,
            deadline_ms: None,
            env: Vec::new(),
        };
        match capability.port {
            PortScope::Http(http) => {
                wire.schemes = http.schemes;
                wire.methods = http.methods;
                wire.hosts = Some(http.hosts);
                wire.secret_header = http.secret_header;
            }
            PortScope::Fs(fs) => {
                wire.port = PortName::Fs;
                wire.roots = fs.roots;
                wire.ops = Some(fs.ops);
            }
            PortScope::Process(process) => {
                wire.port = PortName::Process;
                wire.argv = Some(process.argv);
                wire.deadline_ms = process.deadline_ms;
                wire.env = process.env;
            }
            PortScope::Git(git) => {
                wire.port = PortName::Git;
                let ops = git.ops.into_iter().map(|op| match op {
                    GitOp::Read => FsAccess::Read,
                    GitOp::Write => FsAccess::Write,
                });
                wire.ops = Some(ops.collect());
            }
        }
        wire
    }
}

/// The keys of a [`CapabilityDescriptor`] beside its port's: the unknown-key walk takes them out first.
pub const CAPABILITY_KEYS: [&str; 4] = ["name", "why", "call_kinds", "port"];

/// What a capability reaches, on one port. Its serde form (tagged by `port`) is the consent store's; the wire's is
/// [`CapabilityDescriptor`]'s.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "port", rename_all = "snake_case")]
pub enum PortScope {
    Http(HttpScope),
    Fs(FsScope),
    Process(ProcessScope),
    Git(GitScope),
}

impl PortScope {
    /// The port's wire name.
    pub fn port(&self) -> &'static str {
        match self {
            PortScope::Http(_) => "http",
            PortScope::Fs(_) => "fs",
            PortScope::Process(_) => "process",
            PortScope::Git(_) => "git",
        }
    }

    /// Whether it writes: an fs `write` or a git `write`.
    pub fn writes(&self) -> bool {
        match self {
            PortScope::Fs(fs) => fs.ops.contains(&FsAccess::Write),
            PortScope::Git(git) => git.ops.contains(&GitOp::Write),
            PortScope::Http(_) | PortScope::Process(_) => false,
        }
    }
}

/// An HTTP capability: the schemes, methods and hosts it may request.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct HttpScope {
    /// `http`, `https`; empty: both.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub schemes: Vec<String>,
    /// Empty: any method.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub methods: Vec<HttpMethod>,
    pub hosts: Hosts,
    /// A secret the host attaches as a header (ADR 0335 D6); the guest never sees its value.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub secret_header: Option<SecretHeader>,
}

/// The hosts an HTTP capability may reach.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Hosts {
    /// These hosts: `example.com`, or `*.example.com` for any host under it.
    List(Vec<String>),
    /// The hosts the project's values name (a value checker's URLs).
    FromValues,
}

impl Default for Hosts {
    fn default() -> Self {
        Hosts::List(Vec::new())
    }
}

/// An HTTP method a capability may use.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum HttpMethod {
    Get,
    Head,
    Post,
    Put,
    Patch,
    Delete,
    Options,
}

impl HttpMethod {
    pub const ALL: [HttpMethod; 7] = [
        HttpMethod::Get,
        HttpMethod::Head,
        HttpMethod::Post,
        HttpMethod::Put,
        HttpMethod::Patch,
        HttpMethod::Delete,
        HttpMethod::Options,
    ];

    /// `GET`, `HEAD`, …
    pub fn as_str(self) -> &'static str {
        match self {
            HttpMethod::Get => "GET",
            HttpMethod::Head => "HEAD",
            HttpMethod::Post => "POST",
            HttpMethod::Put => "PUT",
            HttpMethod::Patch => "PATCH",
            HttpMethod::Delete => "DELETE",
            HttpMethod::Options => "OPTIONS",
        }
    }

    /// The method `name` names, in any case.
    pub fn parse(name: &str) -> Option<HttpMethod> {
        HttpMethod::ALL
            .into_iter()
            .find(|m| m.as_str().eq_ignore_ascii_case(name))
    }
}

/// A secret the host attaches to a granted HTTPS request as `header: <scheme> <value>` (or the bare value).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecretHeader {
    pub header: String,
    /// A secret the extension declares in `settings`: its environment variable holds the value.
    pub secret: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scheme: Option<String>,
}

/// A file capability: the roots it reaches and what it does there.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FsScope {
    /// `spec`, `project` or a root `specforge.json` names; empty: `spec` and `project`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub roots: Vec<String>,
    pub ops: Vec<FsAccess>,
}

/// What a file capability does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FsAccess {
    Stat,
    Read,
    Walk,
    Write,
}

/// A process capability: the command it runs.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProcessScope {
    /// The program, then its arguments; `{report}` in an argument is the collector's report location.
    pub argv: Vec<String>,
    /// The run's deadline, held to the host's ceiling.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deadline_ms: Option<u32>,
    /// The environment variables, by name, the program inherits or a guest may set beside what any program needs to
    /// work. A person approves them with the command: another name asks again.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub env: Vec<String>,
}

/// A git capability.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitScope {
    pub ops: Vec<GitOp>,
}

/// What a git capability does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GitOp {
    Read,
    Write,
}

/// The call kinds that may write: a fs or git `write` capability names only these.
pub const WRITING_KINDS: [Operation; 2] = [Operation::Collect, Operation::Command];

/// Whether `name` is a scope name: `[a-z][a-z0-9-]*`.
pub fn is_scope_name(name: &str) -> bool {
    let mut chars = name.chars();
    chars.next().is_some_and(|c| c.is_ascii_lowercase())
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

/// Whether `pattern` is a host (`example.com`) or a `*.`-prefixed host (`*.example.com`).
pub fn is_host_pattern(pattern: &str) -> bool {
    let host = pattern.strip_prefix("*.").unwrap_or(pattern);
    !host.is_empty()
        && host.len() <= 253
        && host.split('.').all(|label| {
            !label.is_empty()
                && label.len() <= 63
                && !label.starts_with('-')
                && !label.ends_with('-')
                && label.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
        })
}

/// Every problem of `declaration`'s capabilities and of its collectors' `capability`, by the capability's (or the
/// collector's) index and why: what the registry reports (W021) and what grants nothing.
pub fn capability_problems(declaration: &ExtensionDeclaration) -> Vec<CapabilityProblem> {
    let mut problems = Vec::new();
    let mut seen = BTreeSet::new();
    for (index, capability) in declaration.capabilities.iter().enumerate() {
        let mut problem = |why: String| {
            problems.push(CapabilityProblem {
                owner: Owner::Capability(index),
                why,
            })
        };
        if !is_scope_name(&capability.name) {
            problem(format!(
                "its name '{}' is not a scope name [a-z][a-z0-9-]*",
                capability.name
            ));
        } else if !seen.insert(capability.name.as_str()) {
            problem(format!("its name '{}' is declared twice", capability.name));
        }
        if capability.why.trim().is_empty() {
            problem("it says no `why`".to_string());
        }
        if capability.port.writes()
            && (capability.call_kinds.is_empty()
                || capability
                    .call_kinds
                    .iter()
                    .any(|kind| !WRITING_KINDS.contains(kind)))
        {
            problem(
                "it writes, so its `call_kinds` must name only `collect` or `command`".to_string(),
            );
        }
        match &capability.port {
            PortScope::Process(process) => {
                if process.argv.is_empty() {
                    problem("its `argv` is empty".to_string());
                }
                for name in process
                    .env
                    .iter()
                    .filter(|name| name.is_empty() || name.contains(['=', '\0']))
                {
                    problem(format!("its `env` name '{name}' is no variable's name"));
                }
            }
            PortScope::Http(http) => {
                if let Hosts::List(hosts) = &http.hosts {
                    if hosts.is_empty() {
                        problem("it lists no host".to_string());
                    }
                    for host in hosts.iter().filter(|h| !is_host_pattern(h)) {
                        problem(format!(
                            "its host '{host}' is not a host or a `*.`-prefixed host"
                        ));
                    }
                }
                for scheme in http
                    .schemes
                    .iter()
                    .filter(|s| !matches!(s.as_str(), "http" | "https"))
                {
                    problem(format!(
                        "its scheme '{scheme}' is neither `http` nor `https`"
                    ));
                }
                if let Some(secret) = &http.secret_header {
                    for why in secret_header_problems(declaration, http, secret) {
                        problem(why);
                    }
                }
            }
            _ => {}
        }
    }
    for (index, collector) in declaration.collectors.iter().enumerate() {
        let Some(name) = &collector.capability else {
            continue;
        };
        let names_process = declaration
            .capabilities
            .iter()
            .any(|c| &c.name == name && matches!(c.port, PortScope::Process(_)));
        if !names_process {
            problems.push(CapabilityProblem {
                owner: Owner::Collector(index),
                why: format!("its capability '{name}' is no `process` capability it declares"),
            });
        }
    }
    problems
}

/// The headers the host never lets a secret set: the ones the HTTP stack owns, and the hop-by-hop ones.
const HOST_HEADERS: [&str; 11] = [
    "host",
    "cookie",
    "content-length",
    "connection",
    "keep-alive",
    "proxy-authenticate",
    "proxy-authorization",
    "te",
    "trailer",
    "transfer-encoding",
    "upgrade",
];

/// Why an http capability's `secret_header` cannot be attached (ADR 0335 D6): the secret is no secret the extension
/// declares, the hosts come from values, a scheme is not only `https`, or the header is no token the host may set.
fn secret_header_problems(
    declaration: &ExtensionDeclaration,
    http: &HttpScope,
    secret: &SecretHeader,
) -> Vec<String> {
    let mut problems = Vec::new();
    let declared = declaration
        .settings
        .iter()
        .any(|item| matches!(item, SettingsItem::Secret(s) if s.name == secret.secret));
    if !declared {
        problems.push(format!(
            "its `secret_header` names the secret '{}', which it does not declare in `settings`",
            secret.secret
        ));
    }
    if http.hosts == Hosts::FromValues {
        problems
            .push("its `secret_header` goes only to hosts it lists, not `from_values`".to_string());
    }
    if http.schemes != ["https"] {
        problems.push("its `secret_header` needs `schemes` to be only `https`".to_string());
    }
    let token = !secret.header.is_empty()
        && secret
            .header
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "!#$%&'*+-.^_`|~".contains(c));
    if !token || HOST_HEADERS.contains(&secret.header.to_ascii_lowercase().as_str()) {
        problems.push(format!(
            "its `secret_header` header '{}' is not a header the host lets a secret set",
            secret.header
        ));
    }
    problems
}

/// One problem [`capability_problems`] found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityProblem {
    pub owner: Owner,
    pub why: String,
}

/// What a [`CapabilityProblem`] is about.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Owner {
    /// The capability at this index.
    Capability(usize),
    /// The collector at this index (its `capability`).
    Collector(usize),
}

impl ExtensionDeclaration {
    /// Its capabilities that have no problem: the ones a grant reads.
    pub fn sound_capabilities(&self) -> Vec<&CapabilityDescriptor> {
        let bad: BTreeSet<usize> = capability_problems(self)
            .into_iter()
            .filter_map(|p| match p.owner {
                Owner::Capability(index) => Some(index),
                Owner::Collector(_) => None,
            })
            .collect();
        self.capabilities
            .iter()
            .enumerate()
            .filter(|(index, _)| !bad.contains(index))
            .map(|(_, c)| c)
            .collect()
    }
}

#[cfg(test)]
mod tests;
