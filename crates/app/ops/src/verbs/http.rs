//! `specforge http` (ADR 0339): the `Http` port for people and agents, under the user grant. A request is judged by
//! the project's effects ceiling before it is sent and on every redirect, its answer is read up to a cap, and it goes
//! through the effects journal in the mode asked: `online` performs it, `cached` answers a `GET` from a fresh record,
//! `replay` answers from the journal alone. A declared secret rides as `Authorization: Bearer`, attached by the host
//! as the request leaves: no answer and no record holds its value, the record only the header's name.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use specforge_common::remedy::printable;
use specforge_effects::http::{MAX_REDIRECTS, TIMEOUT_CEILING, host_of};
use specforge_effects::journal::{Answer, Blob, RecordDigest};
use specforge_effects::mode::USER;
use specforge_effects::{
    EffectJournal, EffectRequest, Effects, EffectsMode, FsOp, Outcome, OutcomeClass, Redirects,
    Served, UnixMillis,
};
use specforge_protocol_types::{
    AttachedSecret, ExtensionDeclaration, Header, HttpRequest, Redirected, SettingsItem,
    Unreachable,
};

use super::{Answered, Effect, Verbs};
use crate::{NOT_IN_JOURNAL, OpError, OpErrorKind};

/// The most body bytes an answer keeps unless asked for another cap.
pub const ANSWER_BYTES: u64 = 1024 * 1024;
/// The most bytes a body read from a file may hold.
pub const BODY_FILE_BYTES: u64 = 16 * 1024 * 1024;
/// The response headers an answer keeps.
pub const KEPT_HEADERS: [&str; 3] = ["content-type", "link", "location"];
/// The headers a credential rides in: one comes from a declared secret, never typed.
const CREDENTIAL_HEADERS: [&str; 3] = ["authorization", "proxy-authorization", "cookie"];
/// The code of an answer that never came: no connection, no name, a timeout, a refused redirect.
pub const UNREACHABLE: &str = "unreachable";

/// What `specforge http` asks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ask {
    /// `GET`, `HEAD`, `POST`, `PUT`, `PATCH` or `DELETE`.
    pub method: String,
    pub url: String,
    /// Typed `Name: value` headers.
    pub headers: Vec<String>,
    pub body: Option<Body>,
    /// The declared secret to attach ([`secret`]).
    pub secret: Option<AttachedSecret>,
    /// The whole request's time, at most [`TIMEOUT_CEILING`].
    pub timeout: Duration,
    /// The most body bytes the answer keeps; past it the answer is `truncated`.
    pub max_bytes: u64,
}

/// A request body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Body {
    /// A JSON object, sent as `application/json`: its fields, then each `files` field a string read from a file in
    /// the project's roots (`--field-file`), so a long text needs no JSON escaping in a shell.
    Json {
        object: serde_json::Map<String, serde_json::Value>,
        files: Vec<(String, PathBuf)>,
    },
    /// A file in the project's roots, from the current directory, sent as it is.
    File(PathBuf),
}

/// What a request answered.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HttpAnswer {
    pub method: String,
    pub url: String,
    /// A 2xx answer.
    pub ok: bool,
    pub status: u16,
    /// The URL that answered, after the redirects followed.
    pub final_url: String,
    /// The kept response headers ([`KEPT_HEADERS`]), lowercased.
    pub headers: BTreeMap<String, String>,
    /// The body as text (invalid UTF-8 replaced), at most the asked cap.
    pub body: String,
    /// The body read as JSON, when it is JSON and whole.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub json: Option<serde_json::Value>,
    /// The body passed the cap and was cut there.
    pub truncated: bool,
    /// The header a declared secret was attached as: its name, never its value.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub secret_header: Option<String>,
}

/// The declared secret `named` (`<extension>:<name>`, the extension as `specforge.json` loads it) of `declarations`,
/// attached as `Authorization: Bearer` to `url`'s host alone and only over `https` (ADR 0335 D6): its value is read
/// from the secret's environment variable as the request leaves, so neither the answer nor a record holds it.
pub fn secret(
    declarations: &[ExtensionDeclaration],
    named: &str,
    url: &str,
) -> Result<AttachedSecret, OpError> {
    let invalid = |message: String| {
        OpError::new(
            OpErrorKind::InvalidInput,
            OpErrorKind::InvalidInput.as_str(),
            message,
        )
    };
    let Some((extension, name)) = named.rsplit_once(':') else {
        return Err(invalid(format!(
            "'{}' names no secret: write <extension>:<name>, such as @gsd/core:github_token",
            printable(named)
        )));
    };
    if !url.starts_with("https://") {
        return Err(invalid(format!(
            "a secret rides only an https request, and {} is not one",
            printable(url)
        )));
    }
    let host = host_of(url)
        .filter(|host| !host.is_empty())
        .ok_or_else(|| invalid(format!("{} names no host", printable(url))))?;
    let Some(declaration) = declarations
        .iter()
        .find(|declaration| declaration.handshake.name == extension)
    else {
        return Err(OpError::new(
            OpErrorKind::ExtensionNotFound,
            OpErrorKind::ExtensionNotFound.as_str(),
            format!(
                "the project loads no extension '{}', so it declares no secret '{}'",
                printable(extension),
                printable(name)
            ),
        ));
    };
    let declared: Vec<&str> = declaration
        .settings
        .iter()
        .filter_map(|item| match item {
            SettingsItem::Secret(secret) => Some(secret.name.as_str()),
            _ => None,
        })
        .collect();
    let env = declaration.settings.iter().find_map(|item| match item {
        SettingsItem::Secret(secret) if secret.name == name => Some(secret.env.clone()),
        _ => None,
    });
    let Some(env) = env else {
        let known = if declared.is_empty() {
            "it declares none".to_string()
        } else {
            format!("it declares {}", declared.join(", "))
        };
        return Err(invalid(format!(
            "'{}' declares no secret '{}': {known}",
            printable(extension),
            printable(name)
        )));
    };
    Ok(AttachedSecret {
        header: "Authorization".to_string(),
        env,
        scheme: Some("Bearer".to_string()),
        hosts: vec![host],
    })
}

/// Refused when `secret`'s variable is unset or empty as `read` finds it: a request that would leave without its
/// credential is never sent, and the refusal names the variable (never a value).
pub fn secret_is_set(
    secret: &AttachedSecret,
    read: impl Fn(&str) -> Option<String>,
) -> Result<(), OpError> {
    if read(&secret.env).is_some_and(|value| !value.is_empty()) {
        return Ok(());
    }
    Err(OpError::new(
        OpErrorKind::PreconditionFailed,
        "secret_unset",
        format!(
            "the secret's variable {} is unset or empty, so the request would leave without it",
            printable(&secret.env)
        ),
    )
    .with_suggestion(format!(
        "set {} to the credential, then run it again",
        printable(&secret.env)
    )))
}

/// `ask` sent through the `Http` port in `mode`, judged by the user grant first and on every redirect.
pub fn request(
    verbs: &Verbs,
    ask: &Ask,
    mode: EffectsMode,
) -> Result<Answered<HttpAnswer>, OpError> {
    let request = built(verbs, ask)?;
    let asked = EffectRequest::Http(request.clone());
    verbs.permit(&asked)?;
    let hops = Hops {
        verbs,
        denied: Mutex::new(None),
    };
    let effects = Effects::of(
        mode,
        verbs.ports().clone(),
        Vec::new(),
        EffectJournal::for_root(verbs.root()),
    );
    let served = effects.serve_user(verbs.ports(), &asked, UnixMillis::now_or_end(), |ports| {
        let outcome = ports.http.request(&request, &hops);
        // A server's answer, whole or cut; a refused redirect is no answer, though a status came with it.
        let answers = matches!(
            outcome.class,
            OutcomeClass::Ok | OutcomeClass::Status(_) | OutcomeClass::Capped
        );
        let answered = outcome.status.filter(|_| answers).map(|status| {
            let truncated =
                outcome.class == OutcomeClass::Capped || outcome.body.len() as u64 > ask.max_bytes;
            let mut body = outcome.body.clone();
            body.truncate(usize::try_from(ask.max_bytes).unwrap_or(usize::MAX));
            HttpAnswer {
                method: request.method.clone(),
                url: request.url.clone(),
                ok: (200..300).contains(&status),
                status,
                final_url: outcome.final_url.clone(),
                headers: outcome.headers.iter().cloned().collect(),
                json: (!truncated)
                    .then(|| serde_json::from_slice(&body).ok())
                    .flatten(),
                body: String::from_utf8_lossy(&body).into_owned(),
                truncated,
                secret_header: request.secret.as_ref().map(|secret| secret.header.clone()),
            }
        });
        // An answer is the record's payload; a request no server answered keeps its class alone.
        Answer {
            outcome: Outcome {
                class: outcome.class,
                digest: Blob::of(&outcome.body).sha256,
            },
            payload: answered.and_then(|answer| serde_json::to_vec(&answer).ok()),
        }
    });
    match served {
        Served::Outcome {
            answer,
            from_journal,
            observed_at,
        } => match answer
            .payload
            .as_deref()
            .map(serde_json::from_slice::<HttpAnswer>)
        {
            Some(Ok(kept)) => Ok(Answered {
                answer: kept,
                effect: Effect {
                    digest: Some(RecordDigest::asked_by(USER, &asked)),
                    observed_at,
                    from_journal,
                },
            }),
            Some(Err(_)) => Err(OpError::new(
                OpErrorKind::Internal,
                OpErrorKind::Internal.as_str(),
                "the journal's record of this request holds no answer",
            )),
            None => Err(hops
                .denied
                .into_inner()
                .ok()
                .flatten()
                .unwrap_or_else(|| unanswered(&request.url, &answer.outcome.class))),
        },
        Served::Miss => Err(OpError::new(
            OpErrorKind::PreconditionFailed,
            NOT_IN_JOURNAL,
            format!(
                "{} {} was never sent: replay answers from the effects journal alone",
                request.method,
                printable(&request.url)
            ),
        )),
        Served::NotPerformed(_) | Served::Local => Err(OpError::new(
            OpErrorKind::Internal,
            OpErrorKind::Internal.as_str(),
            "the request was not sent",
        )),
    }
}

/// The request `ask` sends: its typed headers (a credential refused), its body and the secret to attach.
fn built(verbs: &Verbs, ask: &Ask) -> Result<HttpRequest, OpError> {
    let invalid = |message: String| {
        OpError::new(
            OpErrorKind::InvalidInput,
            OpErrorKind::InvalidInput.as_str(),
            message,
        )
    };
    let mut headers = Vec::new();
    for typed in &ask.headers {
        let Some((name, value)) = typed.split_once(':') else {
            return Err(invalid(format!(
                "'{}' is no header: write Name: value",
                printable(typed)
            )));
        };
        let name = name.trim();
        if CREDENTIAL_HEADERS.contains(&name.to_ascii_lowercase().as_str()) {
            return Err(invalid(format!(
                "a {name} header carries a credential, which a typed command line would show: attach a declared \
                 secret instead"
            )));
        }
        headers.push(Header::plain(name, value.trim()));
    }
    let typed_type = headers
        .iter()
        .any(|header| header.name.eq_ignore_ascii_case("content-type"));
    let body = match &ask.body {
        None => None,
        Some(Body::Json { object, files }) => {
            if !typed_type {
                headers.push(Header::plain("Content-Type", "application/json"));
            }
            let mut object = object.clone();
            for (key, path) in files {
                let text = String::from_utf8(read_body(verbs, path)?).map_err(|_| {
                    invalid(format!(
                        "the body field {key} reads {}, which is not UTF-8 text",
                        printable(&path.to_string_lossy())
                    ))
                })?;
                object.insert(key.clone(), serde_json::Value::String(text));
            }
            Some(serde_json::to_vec(&object).map_err(|error| invalid(error.to_string()))?)
        }
        Some(Body::File(path)) => Some(read_body(verbs, path)?),
    };
    Ok(HttpRequest {
        method: ask.method.clone(),
        url: ask.url.clone(),
        headers,
        body,
        body_cap: ask.max_bytes.saturating_add(1),
        timeout_ms: u64::try_from(ask.timeout.min(TIMEOUT_CEILING).as_millis()).unwrap_or(u64::MAX),
        max_redirects: MAX_REDIRECTS,
        // The user typed the address: a server on this machine or the local network is theirs to reach, but not one a
        // redirect to another origin names.
        allow_private: specforge_effects::Private::TypedOrigin,
        keep_headers: KEPT_HEADERS.map(String::from).to_vec(),
        secret: ask.secret.clone(),
    })
}

/// The bytes of the body file `typed`, held to the project's roots.
fn read_body(verbs: &Verbs, typed: &std::path::Path) -> Result<Vec<u8>, OpError> {
    let (located, request) = verbs.held(
        typed,
        FsOp::Read {
            max_bytes: BODY_FILE_BYTES,
        },
    )?;
    let outcome = verbs.ports().fs.run(verbs.roots(), &request);
    if outcome.class != OutcomeClass::Ok || outcome.truncated {
        let why = if outcome.truncated {
            format!("it holds more than {BODY_FILE_BYTES} bytes")
        } else {
            outcome.class.to_string()
        };
        return Err(OpError::new(
            OpErrorKind::FileNotFound,
            OpErrorKind::FileNotFound.as_str(),
            format!("{} cannot be sent: {why}", verbs.shown(&located.at)),
        ));
    }
    Ok(outcome.bytes)
}

/// Why no answer came from `url`.
fn unanswered(url: &str, class: &OutcomeClass) -> OpError {
    let kind = match class {
        OutcomeClass::Unreachable(Unreachable::Timeout) => OpErrorKind::Timeout,
        OutcomeClass::Denied | OutcomeClass::Redirect(Redirected::Refused) => {
            OpErrorKind::PermissionDenied
        }
        _ => OpErrorKind::PreconditionFailed,
    };
    OpError::new(
        kind,
        UNREACHABLE,
        format!("{} did not answer: {class}", printable(url)),
    )
}

/// Every redirect judged by the user grant as the hop it sends: the first one refused keeps its refusal.
struct Hops<'a> {
    verbs: &'a Verbs,
    denied: Mutex<Option<OpError>>,
}

impl Redirects for Hops<'_> {
    fn allow(&self, _from: &str, to: &str, method: &str) -> bool {
        let hop = EffectRequest::Http(HttpRequest {
            method: method.to_string(),
            url: to.to_string(),
            ..HttpRequest::default()
        });
        match self.verbs.permit(&hop) {
            Ok(()) => true,
            Err(error) => {
                if let Ok(mut denied) = self.denied.lock() {
                    denied.get_or_insert(error);
                }
                false
            }
        }
    }
}

#[cfg(test)]
mod tests;
