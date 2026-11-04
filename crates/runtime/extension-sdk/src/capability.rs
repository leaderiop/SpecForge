//! Declaring capabilities (ADR 0335 D2): what an extension may make the host do, on one port, under a scope name the
//! machine approves. The builders don't validate: the host's registry does (W021), and a capability with a problem
//! grants nothing.

use specforge_protocol_types::{
    CapabilityDescriptor, FsAccess, FsScope, GitOp, GitScope, Hosts, HttpMethod, HttpScope,
    Operation, PortScope, ProcessScope, SecretHeader,
};

use crate::ContributionsBuilder;

impl ContributionsBuilder {
    /// Declare a capability, named by its scope (`[a-z][a-z0-9-]*`, unique in the extension).
    ///
    /// ```
    /// # use specforge_extension_sdk::prelude::*;
    /// # let mut c = ContributionsBuilder::default();
    /// c.capability(
    ///     Capability::http("links")
    ///         .hosts(&["*.example.com"])
    ///         .methods(&[HttpMethod::Head])
    ///         .why("check that the links the specs write answer"),
    /// );
    /// ```
    pub fn capability(&mut self, capability: Capability) -> &mut Self {
        self.decl.capabilities.push(capability.0);
        self
    }
}

/// Builder for a [`CapabilityDescriptor`].
pub struct Capability(CapabilityDescriptor);

impl Capability {
    fn on(scope: &str, port: PortScope) -> Self {
        Capability(CapabilityDescriptor {
            name: scope.to_string(),
            why: String::new(),
            call_kinds: Vec::new(),
            port,
        })
    }

    /// HTTP requests to the hosts [`Capability::hosts`] lists (or [`Capability::from_values`]).
    pub fn http(scope: &str) -> Self {
        Self::on(scope, PortScope::Http(HttpScope::default()))
    }

    /// File effects in the roots [`Capability::roots`] lists (`spec` and `project` when none), doing [`Capability::ops`].
    pub fn fs(scope: &str) -> Self {
        Self::on(scope, PortScope::Fs(FsScope::default()))
    }

    /// Running the command [`Capability::argv`] gives (a collector's).
    pub fn process(scope: &str) -> Self {
        Self::on(scope, PortScope::Process(ProcessScope::default()))
    }

    /// Reading (or writing, [`Capability::git_ops`]) the project's git repository.
    pub fn git(scope: &str) -> Self {
        Self::on(
            scope,
            PortScope::Git(GitScope {
                ops: vec![GitOp::Read],
            }),
        )
    }

    /// Why the extension needs it: shown when asking and listing.
    pub fn why(mut self, why: &str) -> Self {
        self.0.why = why.to_string();
        self
    }

    /// The call kinds it is for; none: every kind the host's gate allows for its port.
    pub fn call_kinds(mut self, kinds: &[Operation]) -> Self {
        self.0.call_kinds = kinds.to_vec();
        self
    }

    /// An http capability's hosts: `example.com`, or `*.example.com` for any host under it.
    pub fn hosts(mut self, hosts: &[&str]) -> Self {
        if let PortScope::Http(http) = &mut self.0.port {
            http.hosts = Hosts::List(hosts.iter().map(|h| h.to_string()).collect());
        }
        self
    }

    /// An http capability reaches the hosts the project's values name (a value checker's URLs).
    pub fn from_values(mut self) -> Self {
        if let PortScope::Http(http) = &mut self.0.port {
            http.hosts = Hosts::FromValues;
        }
        self
    }

    /// An http capability's schemes; none: `http` and `https`.
    pub fn schemes(mut self, schemes: &[&str]) -> Self {
        if let PortScope::Http(http) = &mut self.0.port {
            http.schemes = schemes.iter().map(|s| s.to_string()).collect();
        }
        self
    }

    /// An http capability's methods; none: any.
    pub fn methods(mut self, methods: &[HttpMethod]) -> Self {
        if let PortScope::Http(http) = &mut self.0.port {
            http.methods = methods.to_vec();
        }
        self
    }

    /// An https capability's secret, which the host attaches as `header: <scheme> <value>` (ADR 0335 D6): `secret`
    /// is a secret the extension declares; its value never reaches the extension.
    pub fn secret_header(mut self, header: &str, secret: &str, scheme: Option<&str>) -> Self {
        if let PortScope::Http(http) = &mut self.0.port {
            http.secret_header = Some(SecretHeader {
                header: header.to_string(),
                secret: secret.to_string(),
                scheme: scheme.map(str::to_string),
            });
        }
        self
    }

    /// An fs capability's roots: `spec`, `project` or a root `specforge.json` names.
    pub fn roots(mut self, roots: &[&str]) -> Self {
        if let PortScope::Fs(fs) = &mut self.0.port {
            fs.roots = roots.iter().map(|r| r.to_string()).collect();
        }
        self
    }

    /// An fs capability's ops.
    pub fn ops(mut self, ops: &[FsAccess]) -> Self {
        if let PortScope::Fs(fs) = &mut self.0.port {
            fs.ops = ops.to_vec();
        }
        self
    }

    /// A git capability's ops.
    pub fn git_ops(mut self, ops: &[GitOp]) -> Self {
        if let PortScope::Git(git) = &mut self.0.port {
            git.ops = ops.to_vec();
        }
        self
    }

    /// A process capability's command: the program, then its arguments; `{report}` in an argument is the
    /// collector's report location.
    pub fn argv(mut self, argv: &[&str]) -> Self {
        if let PortScope::Process(process) = &mut self.0.port {
            process.argv = argv.iter().map(|a| a.to_string()).collect();
        }
        self
    }

    /// A process capability's deadline, held to the host's ceiling.
    pub fn deadline_ms(mut self, deadline_ms: u32) -> Self {
        if let PortScope::Process(process) = &mut self.0.port {
            process.deadline_ms = Some(deadline_ms);
        }
        self
    }
}

#[cfg(test)]
mod tests;
