//! What a lent record's payload holds (ADR 0335 D4): one line of JSON saying what the guest was answered, then the
//! answer's bytes, so a replay answers what the live call did. The journal holds no secret: a response header that
//! carries one is not kept, so a replay answers without it.

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use specforge_effects::http::HttpOutcome;
use specforge_effects::process::ProcessClass;

/// What an HTTP answer says besides its body.
#[derive(Serialize, Deserialize)]
pub(super) struct HttpHead {
    pub status: Option<u16>,
    pub headers: Vec<(String, String)>,
    pub final_url: String,
    pub detail: Option<String>,
}

impl HttpHead {
    pub fn of(outcome: &HttpOutcome) -> Self {
        HttpHead {
            status: outcome.status,
            headers: outcome
                .headers
                .iter()
                .filter(|(name, _)| !carries_a_secret(name))
                .cloned()
                .collect(),
            final_url: outcome.final_url.clone(),
            detail: outcome.detail.clone(),
        }
    }
}

/// What a program's run says besides its output: how it ended, and how much of the bytes is its stderr.
#[derive(Serialize, Deserialize)]
pub(super) struct ProcessHead {
    pub class: ProcessClass,
    pub stderr: u64,
}

/// Whether a response header's value may be a credential.
fn carries_a_secret(name: &str) -> bool {
    let name = name.to_ascii_lowercase();
    ["cookie", "auth", "token", "secret", "key", "credential"]
        .iter()
        .any(|word| name.contains(word))
}

/// `head`, then `bytes`.
pub(super) fn sealed<H: Serialize>(head: &H, bytes: &[&[u8]]) -> Vec<u8> {
    let mut payload = serde_json::to_vec(head).expect("a record's head is plain data");
    payload.push(b'\n');
    payload.extend(bytes.iter().copied().flatten());
    payload
}

/// A payload's head and bytes; none when it is not one [`sealed`] wrote, which answers nothing rather than an empty
/// success.
pub(super) fn opened<H: DeserializeOwned>(payload: &[u8]) -> Option<(H, &[u8])> {
    let line = payload.iter().position(|byte| *byte == b'\n')?;
    let head = serde_json::from_slice(&payload[..line]).ok()?;
    Some((head, &payload[line + 1..]))
}
