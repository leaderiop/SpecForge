//! `specforge http` (ADR 0339): the `Http` port for people and agents, under the project's effects ceiling and
//! through its effects journal.

use std::path::PathBuf;
use std::time::Duration;

use specforge_effects::http::DEFAULT_TIMEOUT;
use specforge_effects::{EffectsMode, Ports};
use specforge_ops::OpError;
use specforge_ops::verbs::http::{self as sent, ANSWER_BYTES, Ask, Body, HttpAnswer};

use super::{VerbArgs, shown};
use crate::outcome::Exit;
use crate::terminal::{Terminal, out};

#[derive(clap::Subcommand)]
pub(crate) enum Action {
    /// GET a URL
    Get(AskArgs),
    /// HEAD a URL: its status and headers
    Head(AskArgs),
    /// POST a body to a URL
    Post(AskArgs),
    /// PUT a body to a URL
    Put(AskArgs),
    /// PATCH a URL with a body
    Patch(AskArgs),
    /// DELETE a URL
    Delete(AskArgs),
}

#[derive(clap::Args)]
pub(crate) struct AskArgs {
    /// The URL, `https://` or `http://`
    url: String,

    /// A request header, `Name: value` (repeatable); a credential comes from --secret
    #[arg(long = "header", short = 'H', value_name = "NAME: VALUE")]
    headers: Vec<String>,

    /// A string field of a JSON body, `key=value` (repeatable)
    #[arg(long = "field", short = 'f', value_name = "KEY=VALUE")]
    fields: Vec<String>,

    /// A field of a JSON body given as JSON, `key=<json>` (repeatable): a number, a boolean, an array, an object
    #[arg(long = "field-json", short = 'F', value_name = "KEY=JSON")]
    json_fields: Vec<String>,

    /// The body, read from a file in the project's roots
    #[arg(long, value_name = "PATH", conflicts_with_all = ["fields", "json_fields"])]
    body_file: Option<PathBuf>,

    /// A secret an extension declares, `<extension>:<name>` (e.g. `@gsd/core:github_token`), sent as
    /// `Authorization: Bearer <its value>` to the URL's host over https; its value is never printed or recorded
    #[arg(long, value_name = "EXTENSION:NAME")]
    secret: Option<String>,

    /// Give up after this long: `10s`, `500ms`, `1m` (at most 60s)
    #[arg(long, value_name = "DURATION", value_parser = super::parse_duration)]
    timeout: Option<Duration>,

    /// The most body bytes the answer keeps; past it, `truncated` is true
    #[arg(long, value_name = "BYTES", default_value_t = ANSWER_BYTES)]
    max_bytes: u64,

    /// Whether the request is sent (online, the default; auto is online), a GET answered from a fresh record of the
    /// effects journal (cached), or every request answered from the journal alone (replay)
    #[arg(long, value_enum)]
    effects: Option<EffectsMode>,

    #[command(flatten)]
    verb: VerbArgs,
}

impl Action {
    pub(crate) fn run(self, terminal: &Terminal, ports: &Ports) -> Exit {
        let (method, args) = match self {
            Action::Get(args) => ("GET", args),
            Action::Head(args) => ("HEAD", args),
            Action::Post(args) => ("POST", args),
            Action::Put(args) => ("PUT", args),
            Action::Patch(args) => ("PATCH", args),
            Action::Delete(args) => ("DELETE", args),
        };
        args.run(method, terminal, ports)
    }
}

impl AskArgs {
    fn run(self, method: &str, terminal: &Terminal, ports: &Ports) -> Exit {
        let format = match self.verb.format(terminal) {
            Ok(format) => format,
            Err(exit) => return exit,
        };
        let mode = self.effects.unwrap_or(EffectsMode::Online);
        let answer = self.verb.open(ports).and_then(|verbs| {
            let ask = self.ask(method, &verbs)?;
            sent::request(&verbs, &ask, mode)
        });
        self.verb
            .print(terminal, format, answer, |terminal, answer: &HttpAnswer| {
                // The body as the server sent it, made safe for a terminal; a HEAD has none.
                out!(terminal, "{}", shown(&answer.body));
            })
    }

    /// What the arguments ask: a JSON body of the fields, or the body file, and the declared secret.
    fn ask(&self, method: &str, verbs: &specforge_ops::verbs::Verbs) -> Result<Ask, OpError> {
        let mut object = serde_json::Map::new();
        for field in &self.fields {
            let (key, value) = split_field(field)?;
            object.insert(
                key.to_string(),
                serde_json::Value::String(value.to_string()),
            );
        }
        for field in &self.json_fields {
            let (key, value) = split_field(field)?;
            let value = serde_json::from_str(value).map_err(|error| {
                usage(format!(
                    "--field-json {}: the value is no JSON ({error})",
                    shown(field)
                ))
            })?;
            object.insert(key.to_string(), value);
        }
        let body = match &self.body_file {
            Some(path) => Some(Body::File(path.clone())),
            None if object.is_empty() => None,
            None => Some(Body::Json(object)),
        };
        let secret = match &self.secret {
            Some(named) => {
                // The declarations alone: the project's extensions are loaded, its sources not compiled.
                let env = crate::project::load_dir(verbs.root().dir());
                Some(sent::secret(
                    env.registries().declarations(),
                    named,
                    &self.url,
                )?)
            }
            None => None,
        };
        Ok(Ask {
            method: method.to_string(),
            url: self.url.clone(),
            headers: self.headers.clone(),
            body,
            secret,
            timeout: self.timeout.unwrap_or(DEFAULT_TIMEOUT),
            max_bytes: self.max_bytes,
        })
    }
}

/// `key=value`, split at its first `=`.
fn split_field(field: &str) -> Result<(&str, &str), OpError> {
    field
        .split_once('=')
        .filter(|(key, _)| !key.is_empty())
        .ok_or_else(|| usage(format!("{} is no field: write key=value", shown(field))))
}

fn usage(message: String) -> OpError {
    OpError::new(
        specforge_ops::OpErrorKind::InvalidInput,
        specforge_ops::OpErrorKind::InvalidInput.as_str(),
        message,
    )
}
