//! The effect plane's wire types (ADR 0335): what a host effect asks for, and the class of what it answered. The
//! host's `specforge-effects` performs them and digests them; the types here are serde only, so a guest that never
//! names them carries none of their code.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// A program run: its argv, environment changes, stdin, deadline and output cap.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProcessRequest {
    /// The program, then its arguments. The runner resolves the program (PATH, and PATHEXT on Windows).
    pub argv: Vec<String>,
    /// Applied after the runner's environment policy, in order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub env: Vec<EnvVar>,
    /// The bytes written to the program's stdin; `None` is a null stdin.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stdin: Option<Vec<u8>>,
    /// The run and its output end by then, held to the runner's ceiling.
    pub deadline_ms: u64,
    /// The most bytes kept of each of stdout and stderr; more stops the run.
    pub output_cap: u64,
    /// The caller reads the output as text (UTF-8, `\r\n` read as `\n`).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub text: bool,
    /// What the program inherits of the host's environment. None: all of it but a secret, as a program the host itself
    /// runs. Some: only what a program needs to work and these names, as a program a guest runs (ADR 0335 D4). Only the
    /// host sets it: the WIT request has no field for it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inherit: Option<Vec<String>>,
}

/// One environment change: set `name` to `value`, or remove it when `value` is `None`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnvVar {
    pub name: String,
    #[serde(default)]
    pub value: Option<String>,
}

impl EnvVar {
    /// `name=value`.
    pub fn set(name: impl Into<String>, value: impl Into<String>) -> Self {
        EnvVar {
            name: name.into(),
            value: Some(value.into()),
        }
    }

    /// `name` removed.
    pub fn removed(name: impl Into<String>) -> Self {
        EnvVar {
            name: name.into(),
            value: None,
        }
    }
}

/// An HTTP request.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct HttpRequest {
    pub method: String,
    pub url: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub headers: Vec<Header>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body: Option<Vec<u8>>,
    /// The most response bytes read; a longer body is cut there and the outcome is `capped`.
    pub body_cap: u64,
    /// The whole request, redirects and body included, held to the adapter's ceiling.
    pub timeout_ms: u64,
    /// The most redirects followed; one more is `redirect:limit`.
    pub max_redirects: u32,
    /// A host that resolves to a loopback, private or link-local address may be reached.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub allow_private: bool,
    /// The response headers the outcome keeps, by name (case-insensitive).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub keep_headers: Vec<String>,
    /// The secret the host attaches as a header, set by the grant that permitted the request (ADR 0335 D6). Never
    /// serialised: no guest names or sees it, and it is outside the request's digest.
    #[serde(skip)]
    pub secret: Option<AttachedSecret>,
}

/// A secret the host attaches to a granted request as `header: <scheme> <value>` (or the bare value), its value
/// read from `env` when the request is sent, and only on an `https` hop to one of `hosts` (ADR 0335 D6). It holds
/// the variable's name, never its value.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AttachedSecret {
    pub header: String,
    /// The environment variable the declared secret names.
    pub env: String,
    pub scheme: Option<String>,
    /// The hosts the granted capability lists.
    pub hosts: Vec<String>,
}

/// A request header. A sensitive one (a credential) is written `<redacted>` in `Debug` and in the canonical form a
/// digest is taken of, and is dropped on a redirect to another origin.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Header {
    pub name: String,
    pub value: String,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub sensitive: bool,
}

impl Header {
    /// A header anyone may see.
    pub fn plain(name: impl Into<String>, value: impl Into<String>) -> Self {
        Header {
            name: name.into(),
            value: value.into(),
            sensitive: false,
        }
    }

    /// A credential.
    pub fn sensitive(name: impl Into<String>, value: impl Into<String>) -> Self {
        Header {
            sensitive: true,
            ..Header::plain(name, value)
        }
    }
}

impl fmt::Debug for Header {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let value = if self.sensitive {
            REDACTED
        } else {
            &self.value
        };
        f.debug_struct("Header")
            .field("name", &self.name)
            .field("value", &value)
            .field("sensitive", &self.sensitive)
            .finish()
    }
}

/// What a sensitive header's value is written as.
pub const REDACTED: &str = "<redacted>";

/// A file effect under a named root.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FsRequest {
    /// `spec`, `project` or a named root.
    pub root: String,
    /// Relative to the root, `/` separated; a `<root>/` prefix names another root.
    pub path: String,
    pub op: FsOp,
}

/// What a file effect does.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum FsOp {
    /// Its kind, size and modification time.
    Stat,
    /// At most `max_bytes` of its bytes.
    Read { max_bytes: u64 },
    /// The files under it matching `glob` (every file when `None`), at most `max_entries`.
    Walk {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        glob: Option<String>,
        max_entries: u32,
    },
}

/// The class of an effect's answer: what decides a verdict, the same on every operating system. It is written as
/// one string (`ok`, `status:404`, `exit:1`, `unreachable:tls`, …).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum OutcomeClass {
    Ok,
    /// An HTTP answer outside 2xx.
    Status(u16),
    /// A program exited with a code other than 0.
    Exit(i32),
    /// A program was ended by a signal it did not ask for.
    Signal,
    /// The deadline passed.
    Deadline,
    /// The output passed its cap.
    Capped,
    /// The file does not exist.
    Missing,
    /// The path names a withheld file, or the effect is not granted.
    Denied,
    /// The path leaves its root.
    Outside,
    /// The program is not found.
    NotFound,
    /// The program was found and could not be started.
    CannotStart,
    Unreachable(Unreachable),
    Redirect(Redirected),
}

/// Why a host could not be reached.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Unreachable {
    Dns,
    Connect,
    Tls,
    Timeout,
    Reset,
    Proxy,
    /// Every address the name resolved to is loopback, private, link-local or unspecified.
    RefusedAddress,
}

/// Why a redirect was not followed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Redirected {
    /// More than the request's `max_redirects`.
    Limit,
    /// From `https` to `http`.
    Downgrade,
    /// The caller's redirect policy refused the target.
    Refused,
}

impl Unreachable {
    const ALL: [(Unreachable, &'static str); 7] = [
        (Unreachable::Dns, "dns"),
        (Unreachable::Connect, "connect"),
        (Unreachable::Tls, "tls"),
        (Unreachable::Timeout, "timeout"),
        (Unreachable::Reset, "reset"),
        (Unreachable::Proxy, "proxy"),
        (Unreachable::RefusedAddress, "refused_address"),
    ];
}

impl Redirected {
    const ALL: [(Redirected, &'static str); 3] = [
        (Redirected::Limit, "limit"),
        (Redirected::Downgrade, "downgrade"),
        (Redirected::Refused, "refused"),
    ];
}

const PLAIN: [(OutcomeClass, &str); 9] = [
    (OutcomeClass::Ok, "ok"),
    (OutcomeClass::Signal, "signal"),
    (OutcomeClass::Deadline, "deadline"),
    (OutcomeClass::Capped, "capped"),
    (OutcomeClass::Missing, "missing"),
    (OutcomeClass::Denied, "denied"),
    (OutcomeClass::Outside, "outside"),
    (OutcomeClass::NotFound, "not_found"),
    (OutcomeClass::CannotStart, "cannot_start"),
];

impl fmt::Display for OutcomeClass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OutcomeClass::Status(code) => write!(f, "status:{code}"),
            OutcomeClass::Exit(code) => write!(f, "exit:{code}"),
            OutcomeClass::Unreachable(why) => {
                let name = Unreachable::ALL
                    .iter()
                    .find(|(w, _)| w == why)
                    .map(|(_, n)| n);
                write!(f, "unreachable:{}", name.expect("every reason is named"))
            }
            OutcomeClass::Redirect(why) => {
                let name = Redirected::ALL
                    .iter()
                    .find(|(w, _)| w == why)
                    .map(|(_, n)| n);
                write!(f, "redirect:{}", name.expect("every reason is named"))
            }
            plain => {
                let name = PLAIN.iter().find(|(c, _)| c == plain).map(|(_, n)| n);
                f.write_str(name.expect("every plain class is named"))
            }
        }
    }
}

/// A string that names no outcome class.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownClass(pub String);

impl fmt::Display for UnknownClass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "'{}' is no outcome class", self.0)
    }
}

impl FromStr for OutcomeClass {
    type Err = UnknownClass;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let unknown = || UnknownClass(text.to_string());
        if let Some((_, class)) = PLAIN
            .iter()
            .map(|(c, n)| (*n, *c))
            .find(|(n, _)| *n == text)
        {
            return Ok(class);
        }
        let (head, tail) = text.split_once(':').ok_or_else(unknown)?;
        match head {
            "status" => tail
                .parse()
                .map(OutcomeClass::Status)
                .map_err(|_| unknown()),
            "exit" => tail.parse().map(OutcomeClass::Exit).map_err(|_| unknown()),
            "unreachable" => Unreachable::ALL
                .iter()
                .find(|(_, n)| *n == tail)
                .map(|(w, _)| OutcomeClass::Unreachable(*w))
                .ok_or_else(unknown),
            "redirect" => Redirected::ALL
                .iter()
                .find(|(_, n)| *n == tail)
                .map(|(w, _)| OutcomeClass::Redirect(*w))
                .ok_or_else(unknown),
            _ => Err(unknown()),
        }
    }
}

impl Serialize for OutcomeClass {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for OutcomeClass {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        text.parse().map_err(serde::de::Error::custom)
    }
}

/// What an effect answered: its class, and the digest of its content (a body, an output, a file).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Outcome {
    pub class: OutcomeClass,
    pub digest: String,
}

/// An effect a guest or the host asks for: what the effects journal keys its record by (ADR 0337 D3).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "port", rename_all = "snake_case")]
pub enum EffectRequest {
    Process(ProcessRequest),
    Http(HttpRequest),
    Fs(FsRequest),
    Resolve(ResolveRequest),
    Tls(TlsRequest),
}

/// A name's addresses, asked of the Http port: there is no Dns port and no `dns` capability.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResolveRequest {
    pub host: String,
    /// A loopback, private or link-local address may be answered.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub allow_private: bool,
    pub timeout_ms: u64,
}

/// A TLS handshake with a host, asked of the Http port: what its leaf certificate says, and whether its chain verifies.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TlsRequest {
    pub host: String,
    /// `tcp_port`: an [`EffectRequest`]'s tag is its `port`.
    pub tcp_port: u16,
    /// A host at a loopback, private or link-local address may be reached.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub allow_private: bool,
    pub timeout_ms: u64,
}
