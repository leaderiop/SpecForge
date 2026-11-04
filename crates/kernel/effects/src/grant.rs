//! The grant (ADR 0335 D2): what one extension may make the host do, the intersection of what it declares, what
//! the call kind's gate allows, what the project's effects ceiling leaves and what this machine approved. It is
//! decided once per extension and environment, pure over its inputs; [`Grant::permits`] answers each request, and
//! [`Grant::needs`] the approvals a run must settle before its first effect (ADR 0125 D5).

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use specforge_common::remedy::printable;
use specforge_common::{Diagnostic, codes};
use specforge_package::Sha256;
use specforge_protocol_types::{
    AttachedSecret, CapabilityDescriptor, EffectRequest, ExtensionDeclaration, FsAccess, FsOp,
    GitOp, GitScope, Hosts, HttpScope, Operation, PortScope, ProcessScope, SettingsItem,
};

use crate::ceiling::EffectsCeiling;
use crate::consent::{
    Approval, ApprovedAt, CapabilityApproval, Consent, ConsentError, KeyedBy, ProjectKey,
    RootApproval, approval_line,
};
use crate::gate::{Port, gate};
use crate::scope::rules;
use crate::{Provenance, Roots};

/// What a run will exercise: each call kind and the capability it uses (O11: only these are asked for).
pub type Uses = BTreeSet<(Operation, String)>;

/// One declared capability, as granted here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Granted {
    pub declared: CapabilityDescriptor,
    /// What the ceiling left; `None`: it grants nothing.
    pub scope: Option<PortScope>,
    /// What the ceiling removed, each with its key.
    pub removed: Vec<crate::ceiling::Removed>,
    /// Using it needs this machine's approval (http, process, a write).
    pub needs_approval: bool,
    /// This machine approved it, its granted scope covered.
    pub approved: bool,
    /// The secret the host attaches to the requests it permits (ADR 0335 D6): an `http` scope's `secret_header`,
    /// resolved to the declared secret's environment variable, to the hosts the ceiling left.
    pub secret: Option<AttachedSecret>,
}

impl Granted {
    /// What the ceiling left of it (what it declares when it left nothing), in the line a person approves.
    pub fn summary(&self) -> String {
        approval_line(
            self.scope.as_ref().unwrap_or(&self.declared.port),
            self.secret.as_ref().map(|secret| secret.env.as_str()),
        )
    }
}

/// A named root that resolves outside the project.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutsideRoot {
    pub name: String,
    /// Its canonical target.
    pub target: String,
    pub approved: bool,
}

/// What one extension may make the host do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Grant {
    extension: String,
    provenance: Option<Provenance>,
    project: ProjectKey,
    /// The user's own act (`specforge http`, …): only the ceiling narrows it.
    user: bool,
    capabilities: Vec<Granted>,
    outside: Vec<OutsideRoot>,
    /// Why the consent store could not be read (W186): nothing in it counts.
    store: Option<ConsentError>,
}

/// One approval a run needs and does not have.
#[expect(clippy::large_enum_variant, reason = "a handful per run, built once")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Need {
    /// A capability: what to store once approved, and what to show when asking.
    Capability {
        approval: CapabilityApproval,
        why: String,
        summary: String,
    },
    /// A named root outside the project.
    Root(RootApproval),
}

impl Need {
    /// The approval to store once approved.
    pub fn approval(&self) -> Approval {
        match self {
            Need::Capability { approval, .. } => Approval::Capability(approval.clone()),
            Need::Root(root) => Approval::Root(root.clone()),
        }
    }
}

/// Why a request is refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Denied {
    /// The call kind may not use the port at all (the gate).
    CallKind,
    /// No declared capability covers the request.
    Undeclared,
    /// A declared capability covers it, but not what the project's ceiling left of it.
    Ceiling,
    /// The capability needs this machine's approval and has none.
    NotApproved,
    /// The request reaches a named root outside the project that this machine did not approve.
    Root,
}

/// A refused request: who asked, what, under which scope and why. `CallKind`, `Undeclared` and `Ceiling` are E090
/// ([`Denial::diagnostic`]); `NotApproved` and `Root` are the caller's to report (collect's E059, a check's W170).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Denial {
    pub extension: String,
    /// The request, in one line.
    pub request: String,
    /// The capability that came closest, when one did.
    pub capability: Option<String>,
    pub why: Denied,
    /// What a person reads for `why`.
    pub detail: String,
}

impl Denial {
    /// E090: an effect outside the granted scope.
    pub fn diagnostic(&self) -> Diagnostic {
        let capability = self
            .capability
            .as_deref()
            .map(|s| format!(" (capability '{s}')"))
            .unwrap_or_default();
        Diagnostic::new(
            codes::E090,
            format!(
                "extension '{}': `{}` is outside its granted scope{capability}: {}",
                self.extension, self.request, self.detail
            ),
        )
    }
}

/// The current project's key: its canonical root.
pub fn project_key(project: &Path) -> ProjectKey {
    ProjectKey(
        project
            .canonicalize()
            .unwrap_or_else(|_| project.to_path_buf())
            .display()
            .to_string(),
    )
}

impl Grant {
    /// `extension`'s grant: its sound capabilities (one with a W021 grants nothing), each narrowed by `ceiling`,
    /// and what `consent` approved for it, loaded as `provenance`, in `roots`' project.
    pub fn decide(
        declaration: &ExtensionDeclaration,
        provenance: &Provenance,
        ceiling: &EffectsCeiling,
        roots: &Roots,
        consent: &dyn Consent,
    ) -> Grant {
        let project = project_key(roots.project());
        // An unusable store approves nothing (W186); its error is kept once.
        let mut store = None;
        let mut approved = |wanted: Approval| match consent.approved(&wanted) {
            Ok(approved) => approved,
            Err(error) => {
                store.get_or_insert(error);
                false
            }
        };
        let mut grant = Grant {
            extension: declaration.name().to_string(),
            provenance: Some(provenance.clone()),
            project: project.clone(),
            user: false,
            capabilities: Vec::new(),
            outside: outside_roots(roots, &project),
            store: None,
        };
        for declared in declaration.sound_capabilities() {
            let narrowed = ceiling.narrow(&declared.port);
            let needs_approval =
                matches!(declared.port, PortScope::Http(_) | PortScope::Process(_))
                    || declared.port.writes();
            let secret = narrowed
                .scope
                .as_ref()
                .and_then(|scope| attached(declaration, &declared.port, scope));
            let mut granted = Granted {
                declared: declared.clone(),
                scope: narrowed.scope,
                removed: narrowed.removed,
                needs_approval,
                approved: false,
                secret,
            };
            granted.approved = needs_approval
                && grant
                    .key(&granted)
                    .is_some_and(|key| approved(Approval::Capability(key)));
            grant.capabilities.push(granted);
        }
        for root in &mut grant.outside {
            root.approved = approved(Approval::Root(RootApproval {
                project: project.clone(),
                root: root.name.clone(),
                target: root.target.clone(),
                approved_at: ApprovedAt::default(),
            }));
        }
        grant.store = store;
        grant
    }

    /// The user's own act (`specforge http`, `files`, `git`, `run`): every port, every request, approved, with
    /// only `ceiling` narrowing it. No gate applies: no call kind is asking.
    pub fn user(ceiling: &EffectsCeiling) -> Grant {
        let open = [
            PortScope::Http(HttpScope {
                hosts: Hosts::FromValues,
                ..HttpScope::default()
            }),
            PortScope::Fs(specforge_protocol_types::FsScope {
                roots: Vec::new(),
                ops: vec![
                    FsAccess::Stat,
                    FsAccess::Read,
                    FsAccess::Walk,
                    FsAccess::Write,
                ],
            }),
            PortScope::Process(ProcessScope::default()),
            PortScope::Git(GitScope {
                ops: vec![GitOp::Read, GitOp::Write],
            }),
        ];
        Grant {
            extension: "the user".to_string(),
            provenance: None,
            project: ProjectKey::default(),
            user: true,
            capabilities: open
                .into_iter()
                .map(|port| {
                    let narrowed = ceiling.narrow(&port);
                    Granted {
                        declared: CapabilityDescriptor {
                            name: port.port().to_string(),
                            why: "your own command".to_string(),
                            call_kinds: Vec::new(),
                            port,
                        },
                        scope: narrowed.scope,
                        removed: narrowed.removed,
                        needs_approval: false,
                        approved: true,
                        secret: None,
                    }
                })
                .collect(),
            outside: Vec::new(),
            store: None,
        }
    }

    pub fn extension(&self) -> &str {
        &self.extension
    }

    pub fn provenance(&self) -> Option<&Provenance> {
        self.provenance.as_ref()
    }

    /// Every declared capability, as granted.
    pub fn capabilities(&self) -> &[Granted] {
        &self.capabilities
    }

    /// The named roots outside the project, and whether each is approved.
    pub fn outside_roots(&self) -> &[OutsideRoot] {
        &self.outside
    }

    /// Count what `need` asks for approved in this grant: a yes, stored or for this run only.
    pub fn allow(&mut self, need: &Need) {
        match need {
            Need::Capability { approval, .. } => self
                .capabilities
                .iter_mut()
                .filter(|g| g.declared.name == approval.scope)
                .for_each(|g| g.approved = true),
            Need::Root(root) => self
                .outside
                .iter_mut()
                .filter(|r| r.name == root.root && r.target == root.target)
                .for_each(|r| r.approved = true),
        }
    }

    /// Why the consent store could not be read, when it could not (W186).
    pub fn store_problem(&self) -> Option<&ConsentError> {
        self.store.as_ref()
    }

    /// The approval `granted` is stored under; `None` when it grants nothing.
    pub fn key(&self, granted: &Granted) -> Option<CapabilityApproval> {
        let provenance = self.provenance.as_ref()?;
        Some(CapabilityApproval {
            project: self.project.clone(),
            extension: self.extension.clone(),
            provenance: KeyedBy::of(provenance),
            scope: granted.declared.name.clone(),
            granted: granted.scope.clone()?,
            call_kinds: granted.declared.call_kinds.clone(),
            secret_env: granted.secret.as_ref().map(|secret| secret.env.clone()),
            digest: Sha256::parse(provenance.digest()).ok(),
            specforge: env!("CARGO_PKG_VERSION").to_string(),
            approved_at: ApprovedAt::default(),
        })
    }

    /// The approvals `uses` needs that this machine does not hold. A used capability the ceiling narrowed out, or
    /// that the gate refuses for its call kind, needs none: it is refused ([`Grant::permits`]).
    pub fn needs(&self, uses: &Uses) -> Vec<Need> {
        let mut needs = Vec::new();
        let mut roots = BTreeSet::new();
        for (op, name) in uses {
            let Some(granted) = self.capabilities.iter().find(|g| &g.declared.name == name) else {
                continue;
            };
            let Some(scope) = &granted.scope else {
                continue;
            };
            if !self.allows_kind(granted, *op, scope) {
                continue;
            }
            if granted.needs_approval
                && !granted.approved
                && let Some(approval) = self.key(granted)
                && !needs.iter().any(|n| matches!(n, Need::Capability { approval: a, .. } if a.scope == approval.scope))
            {
                needs.push(Need::Capability {
                    approval,
                    why: granted.declared.why.clone(),
                    summary: granted.summary(),
                });
            }
            if let PortScope::Fs(fs) = scope {
                for root in self
                    .outside
                    .iter()
                    .filter(|r| !r.approved && fs.roots.contains(&r.name))
                {
                    if roots.insert(root.name.clone()) {
                        needs.push(Need::Root(RootApproval {
                            project: self.project.clone(),
                            root: root.name.clone(),
                            target: root.target.clone(),
                            approved_at: ApprovedAt::default(),
                        }));
                    }
                }
            }
        }
        needs
    }

    /// W170: the scopes and roots `uses` needs that this machine has not approved, and the command that approves
    /// them; `None` when nothing is missing.
    pub fn unapproved_summary(&self, uses: &Uses) -> Option<Diagnostic> {
        let needs = self.needs(uses);
        if needs.is_empty() {
            return None;
        }
        let mut scopes = Vec::new();
        let mut roots = Vec::new();
        for need in &needs {
            match need {
                Need::Capability {
                    approval, summary, ..
                } => scopes.push((approval.scope.clone(), summary.clone())),
                Need::Root(root) => roots.push((root.root.clone(), root.target.clone())),
            }
        }
        let mut what: Vec<String> = scopes
            .iter()
            .map(|(scope, summary)| format!("'{scope}' ({summary})"))
            .collect();
        what.extend(
            roots
                .iter()
                .map(|(root, target)| format!("root '{root}' ({target})")),
        );
        let mut commands = Vec::new();
        if !scopes.is_empty() {
            let names: Vec<&str> = scopes.iter().map(|(s, _)| s.as_str()).collect();
            commands.push(format!(
                "specforge grants approve {} {}",
                self.extension,
                names.join(" ")
            ));
        }
        commands.extend(
            roots.iter().map(|(root, _)| {
                format!("specforge grants approve {} --root {root}", self.extension)
            }),
        );
        Some(
            Diagnostic::new(
                codes::W170,
                printable(&format!(
                    "extension '{}': effects not approved on this machine: {}",
                    self.extension,
                    what.join(", ")
                ))
                .into_owned(),
            )
            .with_suggestion(
                printable(&format!(
                    "approve with `{}`, or allow them for one run when asked",
                    commands.join("` and `")
                ))
                .into_owned(),
            ),
        )
    }

    /// Whether a call of kind `op` may make `request`: `Err` names why not.
    pub fn permits(&self, op: Operation, request: &EffectRequest) -> Result<(), Denial> {
        self.permitted(op, request).map(|_| ())
    }

    /// The capability that permits `request` for a call of kind `op` ([`Grant::permits`]).
    pub fn permitted(&self, op: Operation, request: &EffectRequest) -> Result<&Granted, Denial> {
        let (port, write) = port_of(request);
        let root = match request {
            EffectRequest::Fs(fs) => Some(fs.root.as_str()),
            _ => None,
        };
        self.decided(op, (port, write), one_line(request), root, &|scope| {
            rules(scope).admits(request, self.user)
        })
    }

    /// Whether a call of kind `op` may ask git `question`: git is read-only and no [`EffectRequest`], so its scope
    /// is judged by the port alone.
    pub fn permits_git(&self, op: Operation, question: &str) -> Result<(), Denial> {
        let reads = |scope: &PortScope| matches!(scope, PortScope::Git(git) if git.ops.contains(&GitOp::Read));
        self.decided(
            op,
            (Port::Git, false),
            format!("git {question}"),
            None,
            &reads,
        )
        .map(|_| ())
    }

    /// The capability `admits` keeps for a call of kind `op` on `port`, or why there is none.
    fn decided(
        &self,
        op: Operation,
        (port, write): (Port, bool),
        request: String,
        root: Option<&str>,
        admits: &dyn Fn(&PortScope) -> bool,
    ) -> Result<&Granted, Denial> {
        let deny = |why: Denied, capability: Option<&str>, detail: String| Denial {
            extension: self.extension.clone(),
            request: request.clone(),
            capability: capability.map(str::to_string),
            why,
            detail,
        };
        if !self.user && !gate(op, port, write) {
            return Err(deny(
                Denied::CallKind,
                None,
                format!("a {op} may not use the {} port", port.name()),
            ));
        }
        let declared: Vec<&Granted> = self
            .capabilities
            .iter()
            .filter(|g| {
                (self.user
                    || g.declared.call_kinds.is_empty()
                    || g.declared.call_kinds.contains(&op))
                    && admits(&g.declared.port)
            })
            .collect();
        let Some(first) = declared.first() else {
            return Err(deny(
                Denied::Undeclared,
                None,
                "no capability it declares covers it".to_string(),
            ));
        };
        let left: Vec<&&Granted> = declared
            .iter()
            .filter(|g| g.scope.as_ref().is_some_and(admits))
            .collect();
        let Some(usable) = left
            .iter()
            .find(|g| !g.needs_approval || g.approved)
            .or(left.first())
        else {
            return Err(deny(
                Denied::Ceiling,
                Some(&first.declared.name),
                "the project's effects ceiling (`effects` in specforge.json) narrows it out"
                    .to_string(),
            ));
        };
        let name = usable.declared.name.as_str();
        if usable.needs_approval && !usable.approved {
            return Err(deny(
                Denied::NotApproved,
                Some(name),
                format!(
                    "it is not approved on this machine (`specforge grants approve {} {name}`)",
                    self.extension
                ),
            ));
        }
        if let Some(asked) = root
            && let Some(root) = self.outside.iter().find(|r| r.name == asked && !r.approved)
        {
            return Err(deny(
                Denied::Root,
                Some(name),
                format!(
                    "root '{}' lies outside the project ({}) and is not approved on this machine",
                    root.name, root.target
                ),
            ));
        }
        Ok(**usable)
    }

    fn allows_kind(&self, granted: &Granted, op: Operation, scope: &PortScope) -> bool {
        (granted.declared.call_kinds.is_empty() || granted.declared.call_kinds.contains(&op))
            && gate(op, rules(scope).port(), false)
    }
}

fn port_of(request: &EffectRequest) -> (Port, bool) {
    match request {
        EffectRequest::Http(_) => (Port::Http, false),
        EffectRequest::Fs(_) => (Port::Fs, false),
        EffectRequest::Process(_) => (Port::Process, false),
    }
}

/// `request` in one line, with no header or body: `GET https://…`, `read spec/a.md`, `cargo test`.
pub fn one_line(request: &EffectRequest) -> String {
    match request {
        EffectRequest::Http(http) => format!("{} {}", http.method, http.url),
        EffectRequest::Fs(fs) => {
            let op = match fs.op {
                FsOp::Stat => "stat",
                FsOp::Read { .. } => "read",
                FsOp::Walk { .. } => "walk",
            };
            format!("{op} {}/{}", fs.root, fs.path)
        }
        EffectRequest::Process(process) => process.argv.join(" "),
    }
}

/// The secret `declared`'s `secret_header` names, attached to the hosts `granted` lists (ADR 0335 D6); none for a
/// capability whose hosts come from values, or a secret the extension does not declare (both W021).
fn attached(
    declaration: &ExtensionDeclaration,
    declared: &PortScope,
    granted: &PortScope,
) -> Option<AttachedSecret> {
    let (PortScope::Http(declared), PortScope::Http(granted)) = (declared, granted) else {
        return None;
    };
    let header = declared.secret_header.as_ref()?;
    let Hosts::List(hosts) = &granted.hosts else {
        return None;
    };
    let env = declaration.settings.iter().find_map(|item| match item {
        SettingsItem::Secret(secret) if secret.name == header.secret => Some(secret.env.clone()),
        _ => None,
    })?;
    Some(AttachedSecret {
        header: header.header.clone(),
        env,
        scheme: header.scheme.clone(),
        hosts: hosts.clone(),
    })
}

/// The named roots of `roots` that resolve outside `project` (canonical), by name.
fn outside_roots(roots: &Roots, project: &str) -> Vec<OutsideRoot> {
    let project = Path::new(project);
    let mut outside: BTreeMap<String, String> = BTreeMap::new();
    for (name, path) in roots.named() {
        let target = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        if !target.starts_with(project) {
            outside.insert(name.to_string(), target.display().to_string());
        }
    }
    outside
        .into_iter()
        .map(|(name, target)| OutsideRoot {
            name,
            target,
            approved: false,
        })
        .collect()
}

#[cfg(test)]
mod tests;
