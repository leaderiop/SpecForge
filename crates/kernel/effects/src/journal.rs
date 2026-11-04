//! The effects journal (ADR 0337 D3): one [`EffectRecord`] per request digest under a project's
//! `.specforge/effects/journal/` (a loose root's under the user's), per machine and gitignored. It is the cache an
//! effects mode answers from: a fresh record answers its request again, an unreadable one is a miss the next record
//! overwrites. Not the write journal.
//!
//! A journal record holds no secret, and its files are private (ADR 0288): a record shows the request through
//! [`RecordedRequest`], which has no place for a sensitive header's value, an environment value, a body or stdin,
//! and every file and directory is written `0600`/`0700`.

use std::fmt;
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use specforge_common::ProjectRoot;
use specforge_package::Sha256;

use crate::{EffectRequest, Outcome, OutcomeClass, UnixMillis};

/// Where a project's journal lives, from its root; a loose root's, from the user's home.
pub const JOURNAL_DIR: &str = ".specforge/effects/journal";

/// How long an HTTP success answers again.
pub const HTTP_SUCCESS_TTL: Duration = Duration::from_secs(24 * 60 * 60);
/// How long an HTTP failure (`status:4xx`, `status:5xx`, `unreachable:*`) answers again.
pub const HTTP_FAILURE_TTL: Duration = Duration::from_secs(60 * 60);
/// The most payload bytes a record keeps beside it, whatever the request's own cap.
pub const PAYLOAD_CAP: u64 = 4 * 1024 * 1024;

/// The digest a record is keyed and found by: `sha256:` and 64 lowercase hex characters, so a record's path never
/// leaves its shard.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct RecordDigest(Sha256);

impl RecordDigest {
    /// The digest of `request` ([`crate::digest`]).
    pub fn of(request: &EffectRequest) -> Self {
        Self::parse(&crate::digest(request)).expect("a request digest is sha256 and hex")
    }

    /// `text` when it is `sha256:` and 64 lowercase hex characters.
    pub fn parse(text: &str) -> Option<Self> {
        let hex = text.strip_prefix("sha256:")?;
        Sha256::parse(hex).ok().map(RecordDigest)
    }

    fn hex(&self) -> &str {
        self.0.as_str()
    }
}

impl TryFrom<String> for RecordDigest {
    type Error = String;

    fn try_from(text: String) -> Result<Self, String> {
        Self::parse(&text)
            .ok_or_else(|| format!("{text:?} is not sha256: and 64 lowercase hex characters"))
    }
}

impl From<RecordDigest> for String {
    fn from(digest: RecordDigest) -> String {
        digest.to_string()
    }
}

impl fmt::Display for RecordDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "sha256:{}", self.0)
    }
}

/// Bytes as a record shows them: their digest and length, never the bytes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Blob {
    pub sha256: String,
    pub len: u64,
}

impl Blob {
    pub fn of(bytes: &[u8]) -> Self {
        Blob {
            sha256: format!("sha256:{}", Sha256::of(bytes)),
            len: bytes.len() as u64,
        }
    }
}

/// A request header as a record shows it: its name, and its value only when it is not sensitive.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecordedHeader {
    name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    value: Option<String>,
}

impl RecordedHeader {
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The value; none for a sensitive header, whose value no record holds.
    pub fn value(&self) -> Option<&str> {
        self.value.as_deref()
    }
}

/// What a record shows of its request: what identifies it and is safe to show. The method, URL and argv; header and
/// environment names; a non-sensitive header's value; a body or stdin as a [`Blob`]. Built by [`Self::of`] only.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "port", rename_all = "snake_case")]
pub enum RecordedRequest {
    Http {
        method: String,
        url: String,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        headers: Vec<RecordedHeader>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        body: Option<Blob>,
    },
    Process {
        argv: Vec<String>,
        /// The names of the environment changes, in order.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        env: Vec<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        stdin: Option<Blob>,
    },
}

impl RecordedRequest {
    /// What a record shows of `request`; none for an fs request, which is never journaled.
    pub fn of(request: &EffectRequest) -> Option<Self> {
        Some(match request {
            EffectRequest::Http(http) => RecordedRequest::Http {
                method: http.method.clone(),
                url: http.url.clone(),
                headers: http
                    .headers
                    .iter()
                    .map(|header| RecordedHeader {
                        name: header.name.clone(),
                        value: (!header.sensitive).then(|| header.value.clone()),
                    })
                    .collect(),
                body: http.body.as_deref().map(Blob::of),
            },
            EffectRequest::Process(process) => RecordedRequest::Process {
                argv: process.argv.clone(),
                env: process.env.iter().map(|var| var.name.clone()).collect(),
                stdin: process.stdin.as_deref().map(Blob::of),
            },
            EffectRequest::Fs(_) => return None,
        })
    }
}

/// `GET https://…` for an HTTP request, the argv for a process.
impl fmt::Display for RecordedRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RecordedRequest::Http { method, url, .. } => write!(f, "{method} {url}"),
            RecordedRequest::Process { argv, .. } => f.write_str(&argv.join(" ")),
        }
    }
}

/// One performed effect as the journal keeps it (ADR 0335 D3, ADR 0337 D3): the request as it may be shown, its
/// digest, the outcome, when it was observed, how long it answers again, and the payload kept beside it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EffectRecord {
    pub request: RecordedRequest,
    pub request_digest: RecordDigest,
    pub outcome: Outcome,
    pub observed_at: UnixMillis,
    /// How long the record answers for its request; `None`: it is not served fresh again (kept for audit).
    #[serde(
        default,
        rename = "ttl_s",
        skip_serializing_if = "Option::is_none",
        with = "seconds"
    )]
    pub ttl: Option<Duration>,
    /// The payload kept beside the record (`<hex>.payload`), checked against this when read.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payload: Option<Blob>,
}

/// A time to live written in whole seconds.
mod seconds {
    use std::time::Duration;

    use serde::{Deserialize, Deserializer, Serializer};

    pub(super) fn serialize<S: Serializer>(
        ttl: &Option<Duration>,
        s: S,
    ) -> Result<S::Ok, S::Error> {
        match ttl {
            Some(ttl) => s.serialize_some(&ttl.as_secs()),
            None => s.serialize_none(),
        }
    }

    pub(super) fn deserialize<'de, D: Deserializer<'de>>(
        d: D,
    ) -> Result<Option<Duration>, D::Error> {
        Ok(Option::<u64>::deserialize(d)?.map(Duration::from_secs))
    }
}

/// What a port answered: the outcome, and the payload when the request reads one (an HTTP body, a process's
/// output). What leaves the host toward a guest is this, never the request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Answer {
    pub outcome: Outcome,
    pub payload: Option<Vec<u8>>,
}

/// A record the journal holds, and its payload when it kept one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Kept {
    pub record: EffectRecord,
    pub payload: Option<Vec<u8>>,
}

impl Kept {
    /// The answer it gives again.
    pub fn answer(self) -> Answer {
        Answer {
            outcome: self.record.outcome,
            payload: self.payload,
        }
    }
}

/// What the journal holds for a request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Lookup {
    /// A record within its time to live.
    Fresh(Kept),
    /// A record past it, or one never served fresh again (`ttl` none).
    Expired(Kept),
    /// No record, one that does not read, or one whose payload is missing or not the one it names.
    Missing,
}

/// Whether, and how long, an answer is journaled.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Retention {
    /// Not journaled: an fs read, a refusal (`denied`, so a grant takes effect at once), and any answer the TTL
    /// table does not name.
    Never,
    /// Journaled for audit, never served fresh again (a process run).
    Audit,
    /// Journaled, and served again for this long.
    For(Duration),
}

/// The TTL table (ADR 0337 D3): an HTTP success 24 h; an HTTP failure (`status:4xx`, `status:5xx`,
/// `unreachable:*`) 1 h; a process run for audit; a `denied` answer and every other one never.
///
/// A git question is no `EffectRequest`: the Git port's answers are local reads, never journaled.
pub fn retention(request: &EffectRequest, class: OutcomeClass) -> Retention {
    match (request, class) {
        (_, OutcomeClass::Denied) | (EffectRequest::Fs(_), _) => Retention::Never,
        (EffectRequest::Http(_), OutcomeClass::Ok) => Retention::For(HTTP_SUCCESS_TTL),
        (
            EffectRequest::Http(_),
            OutcomeClass::Status(400..=599) | OutcomeClass::Unreachable(_),
        ) => Retention::For(HTTP_FAILURE_TTL),
        (EffectRequest::Http(_), _) => Retention::Never,
        (EffectRequest::Process(_), _) => Retention::Audit,
    }
}

/// The record of `request` answered `answer` at `observed_at`; none when the answer is not journaled. A payload is
/// kept when it is within the request's cap and [`PAYLOAD_CAP`].
pub fn record_of(
    request: &EffectRequest,
    answer: &Answer,
    observed_at: UnixMillis,
) -> Option<EffectRecord> {
    let ttl = match retention(request, answer.outcome.class) {
        Retention::Never => return None,
        Retention::Audit => None,
        Retention::For(ttl) => Some(ttl),
    };
    let cap = match request {
        EffectRequest::Http(http) => http.body_cap,
        EffectRequest::Process(process) => process.output_cap,
        EffectRequest::Fs(_) => 0,
    }
    .min(PAYLOAD_CAP);
    Some(EffectRecord {
        request: RecordedRequest::of(request)?,
        request_digest: RecordDigest::of(request),
        outcome: answer.outcome.clone(),
        observed_at,
        ttl,
        payload: answer
            .payload
            .as_deref()
            .filter(|payload| payload.len() as u64 <= cap)
            .map(Blob::of),
    })
}

/// Whether `record` still answers its request at `now`.
pub fn fresh(record: &EffectRecord, now: UnixMillis) -> bool {
    expires(record).is_some_and(|expires| now < expires)
}

/// When `record` stops answering its request; none when it is never served fresh again.
pub fn expires(record: &EffectRecord) -> Option<UnixMillis> {
    record.ttl.map(|ttl| record.observed_at.plus(ttl))
}

/// A project's effects journal; a detached one holds nothing and writes nothing.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EffectJournal {
    dir: Option<PathBuf>,
}

impl EffectJournal {
    /// The journal under `base` (a project root, or the user's home for a loose root).
    pub fn at(base: &Path) -> Self {
        EffectJournal {
            dir: Some(base.join(JOURNAL_DIR)),
        }
    }

    /// The journal of `root`: under it when it holds `specforge.json`; a loose root's under `home` (none: detached),
    /// so a run in any directory writes nothing into it.
    pub fn of(root: &ProjectRoot, home: Option<&Path>) -> Self {
        if root.is_configured() {
            Self::at(root.dir())
        } else {
            home.map_or_else(Self::detached, Self::at)
        }
    }

    /// [`Self::of`] under the user's home (`HOME`, `USERPROFILE` on Windows).
    pub fn for_root(root: &ProjectRoot) -> Self {
        let home = std::env::var_os("HOME")
            .or_else(|| std::env::var_os("USERPROFILE"))
            .map(PathBuf::from);
        Self::of(root, home.as_deref())
    }

    /// A journal of no project: every lookup is a miss and nothing is written.
    pub fn detached() -> Self {
        EffectJournal::default()
    }

    /// The directory records are written under; none for a detached journal.
    pub fn dir(&self) -> Option<&Path> {
        self.dir.as_deref()
    }

    /// The file of the record keyed `digest`: `<dir>/<aa>/<hex>.json`. None for a detached journal.
    pub(crate) fn path(&self, digest: &RecordDigest) -> Option<PathBuf> {
        let hex = digest.hex();
        self.dir
            .as_ref()
            .map(|dir| dir.join(&hex[..2]).join(format!("{hex}.json")))
    }

    /// Keep what `request` answered at `observed_at`, replacing the request's last record atomically and privately.
    /// `Ok(None)` when the answer is not journaled ([`retention`]) or the journal is detached; else the record file
    /// written and the record.
    pub fn keep(
        &self,
        request: &EffectRequest,
        answer: &Answer,
        observed_at: UnixMillis,
    ) -> std::io::Result<Option<(PathBuf, EffectRecord)>> {
        let Some(record) = record_of(request, answer, observed_at) else {
            return Ok(None);
        };
        let Some(path) = self.path(&record.request_digest) else {
            return Ok(None);
        };
        if let (Some(_), Some(payload)) = (&record.payload, &answer.payload) {
            specforge_common::fresh::replace_private(&path.with_extension("payload"), payload)?;
        }
        let bytes = serde_json::to_vec_pretty(&record).map_err(std::io::Error::other)?;
        specforge_common::fresh::replace_private(&path, &bytes)?;
        Ok(Some((path, record)))
    }

    /// What the journal holds for the request keyed `digest`, at `now`.
    pub fn lookup(&self, digest: &RecordDigest, now: UnixMillis) -> Lookup {
        let Some(path) = self.path(digest) else {
            return Lookup::Missing;
        };
        let read = std::fs::read(&path)
            .ok()
            .and_then(|bytes| serde_json::from_slice::<EffectRecord>(&bytes).ok())
            .filter(|record| record.request_digest == *digest);
        let Some(record) = read else {
            return Lookup::Missing;
        };
        let payload = match &record.payload {
            None => None,
            Some(blob) => match std::fs::read(path.with_extension("payload")) {
                Ok(bytes) if Blob::of(&bytes) == *blob => Some(bytes),
                _ => return Lookup::Missing,
            },
        };
        let kept = Kept { record, payload };
        if fresh(&kept.record, now) {
            Lookup::Fresh(kept)
        } else {
            Lookup::Expired(kept)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        AttachedSecret, EnvVar, FsOp, FsRequest, Header, HttpRequest, ProcessRequest, Unreachable,
    };

    fn http(url: &str) -> EffectRequest {
        EffectRequest::Http(HttpRequest {
            method: "GET".into(),
            url: url.into(),
            body_cap: 1024,
            ..HttpRequest::default()
        })
    }

    fn answer(class: OutcomeClass) -> Answer {
        Answer {
            outcome: Outcome {
                class,
                digest: String::new(),
            },
            payload: None,
        }
    }

    fn at(ms: u64) -> UnixMillis {
        UnixMillis(ms)
    }

    #[test]
    fn a_record_is_written_atomically_and_read_back_by_its_request_digest() {
        let dir = tempfile::tempdir().unwrap();
        let journal = EffectJournal::at(dir.path());
        let request = http("https://example.org/");
        let (path, record) = journal
            .keep(&request, &answer(OutcomeClass::Ok), at(1_000))
            .unwrap()
            .expect("journaled");
        assert!(path.is_file());
        assert!(!path.with_extension("json.tmp").exists(), "no temp left");
        assert_eq!(record.request_digest, RecordDigest::of(&request));
        assert_eq!(
            journal.lookup(&record.request_digest, at(2_000)),
            Lookup::Fresh(Kept {
                record: record.clone(),
                payload: None
            })
        );
        assert_eq!(record.ttl, Some(HTTP_SUCCESS_TTL));
    }

    #[test]
    fn only_an_http_failure_the_table_names_is_journaled_an_hour() {
        let request = http("https://example.org/");
        let ttl = |class| record_of(&request, &answer(class), at(0)).map(|record| record.ttl);
        assert_eq!(ttl(OutcomeClass::Status(404)), Some(Some(HTTP_FAILURE_TTL)));
        assert_eq!(ttl(OutcomeClass::Status(503)), Some(Some(HTTP_FAILURE_TTL)));
        assert_eq!(
            ttl(OutcomeClass::Unreachable(Unreachable::Dns)),
            Some(Some(HTTP_FAILURE_TTL))
        );
        for class in [
            OutcomeClass::Denied,
            OutcomeClass::Deadline,
            OutcomeClass::Capped,
            OutcomeClass::Status(304),
            OutcomeClass::Redirect(crate::Redirected::Limit),
        ] {
            assert_eq!(ttl(class), None, "{class} is not journaled");
        }
        let run = EffectRequest::Process(ProcessRequest::default());
        assert_eq!(
            record_of(&run, &answer(OutcomeClass::Exit(1)), at(0)).map(|r| r.ttl),
            Some(None),
            "a process run is kept for audit"
        );
        assert_eq!(record_of(&run, &answer(OutcomeClass::Denied), at(0)), None);
    }

    #[test]
    fn a_record_past_its_ttl_is_expired_not_fresh() {
        let dir = tempfile::tempdir().unwrap();
        let journal = EffectJournal::at(dir.path());
        let request = http("https://example.org/");
        let (_, record) = journal
            .keep(&request, &answer(OutcomeClass::Ok), at(0))
            .unwrap()
            .unwrap();
        let end = at(HTTP_SUCCESS_TTL.as_millis() as u64);
        assert_eq!(expires(&record), Some(end));
        assert!(matches!(
            journal.lookup(&record.request_digest, at(end.0 - 1)),
            Lookup::Fresh(_)
        ));
        assert!(matches!(
            journal.lookup(&record.request_digest, end),
            Lookup::Expired(_)
        ));
        // A process run is journaled for audit but never answers fresh again.
        let run = EffectRequest::Process(ProcessRequest::default());
        let (_, record) = journal
            .keep(&run, &answer(OutcomeClass::Ok), at(0))
            .unwrap()
            .expect("journaled");
        assert!(matches!(
            journal.lookup(&record.request_digest, at(0)),
            Lookup::Expired(_)
        ));
    }

    #[test]
    fn an_undecodable_record_is_missing_and_the_next_record_replaces_it() {
        let dir = tempfile::tempdir().unwrap();
        let journal = EffectJournal::at(dir.path());
        let request = http("https://example.org/");
        let digest = RecordDigest::of(&request);
        let path = journal.path(&digest).unwrap();
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, b"{ not json").unwrap();
        assert_eq!(journal.lookup(&digest, at(0)), Lookup::Missing);
        journal
            .keep(&request, &answer(OutcomeClass::Ok), at(0))
            .unwrap();
        assert!(matches!(journal.lookup(&digest, at(0)), Lookup::Fresh(_)));
    }

    #[test]
    fn fs_and_git_requests_are_never_journaled() {
        let dir = tempfile::tempdir().unwrap();
        let journal = EffectJournal::at(dir.path());
        let read = EffectRequest::Fs(FsRequest {
            root: "project".into(),
            path: "a.txt".into(),
            op: FsOp::Read { max_bytes: 1 },
        });
        assert_eq!(retention(&read, OutcomeClass::Ok), Retention::Never);
        assert_eq!(
            journal
                .keep(&read, &answer(OutcomeClass::Ok), at(0))
                .unwrap(),
            None
        );
        assert!(!dir.path().join(JOURNAL_DIR).exists());
        // A detached journal writes nothing either, and every lookup misses.
        let request = http("https://example.org/");
        let detached = EffectJournal::detached();
        assert_eq!(
            detached
                .keep(&request, &answer(OutcomeClass::Ok), at(0))
                .unwrap(),
            None
        );
        assert_eq!(
            detached.lookup(&RecordDigest::of(&request), at(0)),
            Lookup::Missing
        );
    }

    #[test]
    fn the_journal_lives_under_specforge_effects() {
        let journal = EffectJournal::at(Path::new("/p"));
        let hex = "ab".repeat(32);
        let digest = RecordDigest::parse(&format!("sha256:{hex}")).unwrap();
        assert_eq!(
            journal.path(&digest).unwrap(),
            Path::new("/p/.specforge/effects/journal/ab").join(format!("{hex}.json"))
        );
    }

    #[test]
    fn a_record_digest_is_sha256_and_hex_only() {
        let hex = "0f".repeat(32);
        assert!(RecordDigest::parse(&format!("sha256:{hex}")).is_some());
        for text in [
            "../x",
            "sha256:../x",
            hex.as_str(),
            &format!("sha256:{}", "0F".repeat(32)),
            &format!("sha256:{}/..", "0f".repeat(31)),
            &format!("sha256:{hex}0"),
        ] {
            assert_eq!(RecordDigest::parse(text), None, "{text}");
        }
        // A record file that names another digest does not decode.
        assert!(serde_json::from_str::<RecordDigest>("\"../x\"").is_err());
    }

    #[test]
    fn a_record_names_neither_the_secret_a_grant_attached_nor_its_header() {
        let dir = tempfile::tempdir().unwrap();
        let request = EffectRequest::Http(HttpRequest {
            method: "GET".into(),
            url: "https://example.org/api".into(),
            secret: Some(AttachedSecret {
                header: "X-Attached-Key".into(),
                env: "ATTACHED_ENV".into(),
                scheme: Some("Bearer".into()),
                hosts: vec!["example.org".into()],
            }),
            ..HttpRequest::default()
        });
        let (path, _) = EffectJournal::at(dir.path())
            .keep(&request, &answer(OutcomeClass::Ok), at(0))
            .unwrap()
            .unwrap();
        let written = std::fs::read_to_string(&path).unwrap();
        for name in ["ATTACHED_ENV", "X-Attached-Key"] {
            assert!(!written.contains(name), "{name} in {written}");
        }
    }

    #[test]
    fn a_record_holds_no_secret() {
        let dir = tempfile::tempdir().unwrap();
        let journal = EffectJournal::at(dir.path());
        let request = EffectRequest::Http(HttpRequest {
            method: "POST".into(),
            url: "https://example.org/api".into(),
            headers: vec![
                Header::sensitive("Authorization", "Bearer s3cr3t-t0ken"),
                Header::plain("Accept", "application/json"),
            ],
            body: Some(b"password=hunter2".to_vec()),
            body_cap: 1024,
            ..HttpRequest::default()
        });
        let (path, _) = journal
            .keep(&request, &answer(OutcomeClass::Ok), at(0))
            .unwrap()
            .unwrap();
        let written = std::fs::read_to_string(&path).unwrap();
        for secret in ["s3cr3t-t0ken", "hunter2"] {
            assert!(!written.contains(secret), "{secret} in {written}");
        }
        assert!(written.contains("Authorization") && written.contains("application/json"));
        assert!(written.contains(&Blob::of(b"password=hunter2").sha256));
        let run = EffectRequest::Process(ProcessRequest {
            argv: vec!["deploy".into()],
            env: vec![EnvVar::set("API_TOKEN", "env-s3cr3t")],
            stdin: Some(b"stdin-s3cr3t".to_vec()),
            ..ProcessRequest::default()
        });
        let (path, _) = journal
            .keep(&run, &answer(OutcomeClass::Ok), at(0))
            .unwrap()
            .unwrap();
        let written = std::fs::read_to_string(&path).unwrap();
        assert!(!written.contains("env-s3cr3t") && !written.contains("stdin-s3cr3t"));
        assert!(written.contains("API_TOKEN"));
    }

    #[cfg(unix)]
    #[test]
    fn journal_files_and_directories_are_private() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let journal = EffectJournal::at(dir.path());
        let request = http("https://example.org/");
        let ok = Answer {
            payload: Some(b"<html>".to_vec()),
            ..answer(OutcomeClass::Ok)
        };
        let (path, _) = journal.keep(&request, &ok, at(0)).unwrap().unwrap();
        let mode = |path: &Path| std::fs::metadata(path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode(&path), 0o600);
        assert_eq!(mode(&path.with_extension("payload")), 0o600);
        assert_eq!(mode(path.parent().unwrap()), 0o700);
        assert_eq!(mode(&dir.path().join(JOURNAL_DIR)), 0o700);
    }

    #[test]
    fn a_payload_is_kept_beside_its_record_and_checked_when_read() {
        let dir = tempfile::tempdir().unwrap();
        let journal = EffectJournal::at(dir.path());
        let request = http("https://example.org/");
        let ok = Answer {
            payload: Some(b"{\"ok\":true}".to_vec()),
            ..answer(OutcomeClass::Ok)
        };
        let (path, record) = journal.keep(&request, &ok, at(0)).unwrap().unwrap();
        let Lookup::Fresh(kept) = journal.lookup(&record.request_digest, at(1)) else {
            panic!("fresh");
        };
        assert_eq!(kept.answer(), ok);
        // A payload that is not the one the record names is a miss.
        std::fs::write(path.with_extension("payload"), b"tampered").unwrap();
        assert_eq!(
            journal.lookup(&record.request_digest, at(1)),
            Lookup::Missing
        );
        // A payload over the request's cap is not kept; the record is.
        let big = Answer {
            payload: Some(vec![b'x'; 2048]),
            ..answer(OutcomeClass::Ok)
        };
        let (path, record) = journal.keep(&request, &big, at(0)).unwrap().unwrap();
        assert_eq!(record.payload, None);
        let Lookup::Fresh(kept) = journal.lookup(&record.request_digest, at(1)) else {
            panic!("fresh");
        };
        assert_eq!(kept.payload, None);
        assert!(path.is_file());
    }

    #[test]
    fn a_loose_roots_journal_is_the_users() {
        let project = tempfile::tempdir().unwrap();
        let home = tempfile::tempdir().unwrap();
        let loose = ProjectRoot::of(project.path()).unwrap();
        assert!(!loose.is_configured());
        assert_eq!(
            EffectJournal::of(&loose, Some(home.path())),
            EffectJournal::at(home.path())
        );
        assert_eq!(EffectJournal::of(&loose, None), EffectJournal::detached());
        std::fs::write(project.path().join("specforge.json"), "{}").unwrap();
        let configured = ProjectRoot::of(project.path()).unwrap();
        assert_eq!(
            EffectJournal::of(&configured, Some(home.path())),
            EffectJournal::at(project.path())
        );
        // A loose root's run writes into the user's journal, never into the directory it ran in.
        let journal = EffectJournal::of(&loose, Some(home.path()));
        journal
            .keep(
                &http("https://example.org/"),
                &answer(OutcomeClass::Ok),
                at(0),
            )
            .unwrap()
            .unwrap();
        assert!(home.path().join(JOURNAL_DIR).is_dir());
        assert!(!project.path().join(".specforge").exists());
    }
}
