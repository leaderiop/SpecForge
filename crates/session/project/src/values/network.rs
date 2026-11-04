//! The requests a checker's probes make: on [`WORKERS`] threads, at most [`PER_HOST`] to one host at once, each through
//! the run's effects under its extension's grant. Their answers are journaled as a guest reads them.

use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Condvar, Mutex};
use std::time::Instant;

use specforge_effects::http::Any;
use specforge_effects::journal::{Answer, Blob};
use specforge_effects::{
    Asker, EffectRequest, Effects, HttpOutcome, HttpRequest, Outcome, OutcomeClass, Ports, Served,
    UnixMillis,
};
use specforge_protocol_types::{
    ByteRange, Header, HttpMethod, Operation, OutcomeData, Probe, ProbeOutcome,
};

use super::budget;

/// Calls on the network at once, overall and to one host.
const WORKERS: usize = 32;
const PER_HOST: usize = 8;
/// What a journaled answer's head may add to its body: a request's cap holds both.
pub(super) const HEAD_ROOM: u64 = 8 * 1024;

/// One request on the network, of any number of probes.
pub(super) struct Job<'a> {
    pub extension: &'a str,
    pub scopes: &'a [String],
    /// What a not-performed one counts under (I206): its checker.
    pub scope: &'a str,
    pub probe: &'a Probe,
    pub request: EffectRequest,
    pub host: String,
    /// The most body a checker reads of it.
    pub max_bytes: u64,
}

/// What became of a [`Job`].
#[derive(Debug, Clone)]
pub(super) enum Performed {
    Answer(ProbeOutcome),
    /// Asked, and this run's effects did not perform it: they counted it under its scope.
    Declined,
    /// Not asked: the run's budget was spent.
    OverBudget,
}

/// What the network may still spend.
pub(super) struct Spending {
    pub deadline: Instant,
    /// The most an answer's body may add up to; what the run read before is already spent.
    pub bytes: u64,
    pub spent: u64,
}

/// The jobs, on [`WORKERS`] threads, at most [`PER_HOST`] to one host at once. None starts once `spending` is used up.
pub(super) fn perform_all(
    effects: &Effects,
    now: UnixMillis,
    jobs: &[Job<'_>],
    spending: Spending,
) -> Vec<Performed> {
    struct Queue<'a> {
        pending: VecDeque<usize>,
        busy: HashMap<&'a str, usize>,
    }
    let queue = Mutex::new(Queue {
        pending: (0..jobs.len()).collect(),
        busy: HashMap::new(),
    });
    let freed = Condvar::new();
    let results = Mutex::new(vec![Performed::OverBudget; jobs.len()]);
    let spent = AtomicU64::new(spending.spent);
    std::thread::scope(|scope| {
        for _ in 0..WORKERS.min(jobs.len()) {
            scope.spawn(|| {
                loop {
                    let mut held = lock_of(&queue);
                    let index = loop {
                        if held.pending.is_empty() {
                            return;
                        }
                        let free = held.pending.iter().position(|&index| {
                            held.busy
                                .get(jobs[index].host.as_str())
                                .copied()
                                .unwrap_or(0)
                                < PER_HOST
                        });
                        match free {
                            Some(at) => {
                                let index = held.pending.remove(at).expect("a position it found");
                                *held.busy.entry(jobs[index].host.as_str()).or_default() += 1;
                                break index;
                            }
                            None => {
                                held = freed.wait(held).unwrap_or_else(|e| e.into_inner());
                            }
                        }
                    };
                    drop(held);
                    let spare = spent.load(Ordering::Relaxed) < spending.bytes
                        && Instant::now() < spending.deadline;
                    if spare {
                        let outcome = perform_one(effects, now, &jobs[index]);
                        // Before its slot frees: the most it overruns by is what is on the wire to its host now.
                        if let Performed::Answer(outcome) = &outcome {
                            spent.fetch_add(budget::cost(&outcome.data), Ordering::Relaxed);
                        }
                        results.lock().unwrap_or_else(|e| e.into_inner())[index] = outcome;
                    }
                    *lock_of(&queue)
                        .busy
                        .entry(jobs[index].host.as_str())
                        .or_default() -= 1;
                    freed.notify_all();
                }
            });
        }
    });
    results.into_inner().unwrap_or_else(|e| e.into_inner())
}

fn lock_of<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|e| e.into_inner())
}

fn perform_one(effects: &Effects, now: UnixMillis, job: &Job<'_>) -> Performed {
    let asker = Asker {
        extension: job.extension,
        op: Operation::CheckValues,
        scopes: job.scopes,
    };
    match effects.serve_for(&asker, job.scope, &job.request, now, |ports| {
        answer_of(job, ports)
    }) {
        Served::Outcome {
            answer,
            from_journal,
            observed_at,
        } => Performed::Answer(ProbeOutcome {
            probe: job.probe.clone(),
            class: answer.outcome.class,
            data: answer.payload.as_deref().map_or(OutcomeData::None, decode),
            observed_at: observed_at.0,
            from_cache: from_journal,
        }),
        Served::NotPerformed(_) | Served::Miss | Served::Local => Performed::Declined,
    }
}

/// `job` performed through `ports`, which hold its grant: the answer a record keeps and a guest reads.
fn answer_of(job: &Job<'_>, ports: &Ports) -> Answer {
    let (class, data) = match &job.request {
        EffectRequest::Http(request) => {
            let mut outcome = ports.http.request(request, &Any);
            // A server that does not serve HEAD is asked once as a GET, through the same grant.
            if request.method == HttpMethod::Head.as_str()
                && matches!(outcome.status, Some(405 | 501))
            {
                let get = HttpRequest {
                    method: HttpMethod::Get.as_str().to_string(),
                    ..request.clone()
                };
                outcome = ports.http.request(&get, &Any);
            }
            let range = match job.probe {
                Probe::HttpGet { range, .. } => Some(*range),
                _ => None,
            };
            let keep = usize::try_from(job.max_bytes).unwrap_or(usize::MAX);
            let body = served(range, &outcome, keep);
            (
                body.class,
                OutcomeData::Http {
                    status: outcome.status,
                    final_url: outcome.final_url,
                    headers: outcome
                        .headers
                        .into_iter()
                        .map(|(name, value)| Header::plain(name, value))
                        .collect(),
                    body: body.bytes,
                    truncated: body.truncated,
                    start: body.start,
                    total_len: body.total_len,
                },
            )
        }
        EffectRequest::Resolve(request) => {
            let outcome = ports.http.resolve(request);
            (
                outcome.class,
                OutcomeData::Resolve {
                    addresses: outcome.addresses,
                },
            )
        }
        EffectRequest::Tls(request) => {
            let outcome = ports.http.tls(request);
            (
                outcome.class,
                OutcomeData::Tls {
                    not_after: outcome.not_after,
                    issuer: outcome.issuer,
                    chain_valid: outcome.chain_valid,
                },
            )
        }
        EffectRequest::Process(_) | EffectRequest::Fs(_) => {
            (OutcomeClass::Denied, OutcomeData::None)
        }
    };
    let payload = encode(&data);
    Answer {
        outcome: Outcome {
            class,
            digest: Blob::of(&payload).sha256,
        },
        payload: Some(payload),
    }
}

/// What an answer holds of the bytes a GET asked for.
#[derive(Debug, PartialEq, Eq)]
struct Body {
    class: OutcomeClass,
    bytes: Vec<u8>,
    /// Where they start in the whole, and the whole's length when the server said it.
    start: u64,
    total_len: Option<u64>,
    /// The body stopped at its cap.
    truncated: bool,
}

/// The body of `outcome` as `range` asked for it, at most `keep` bytes. A 206 says where its bytes start in
/// `Content-Range` (one with none is `range:ignored`); a 200 to a prefix is its first bytes, and a 200 to a suffix or a
/// span is the whole, sliced when it was read to its end and `range:ignored` when it was not; a 416 says the length.
fn served(range: Option<ByteRange>, outcome: &HttpOutcome, keep: usize) -> Body {
    let header = |name: &str| {
        outcome
            .headers
            .iter()
            .find(|(n, _)| n.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.trim())
    };
    let length = header("content-length").and_then(|v| v.parse::<u64>().ok());
    let capped = outcome.class == OutcomeClass::Capped || outcome.body.len() > keep;
    let mut body = Body {
        class: outcome.class,
        bytes: outcome.body[..outcome.body.len().min(keep)].to_vec(),
        start: 0,
        total_len: length,
        truncated: capped,
    };
    let ignored = |total_len| Body {
        class: OutcomeClass::RangeIgnored,
        bytes: Vec::new(),
        start: 0,
        total_len,
        truncated: false,
    };
    let Some(range) = range.filter(|range| !matches!(range, ByteRange::Whole { .. })) else {
        return body;
    };
    match (outcome.status, range) {
        (Some(206), _) => match header("content-range").and_then(content_range) {
            Some((Some(start), total_len)) => {
                let len = usize::try_from(range.len()).unwrap_or(usize::MAX);
                body.truncated = outcome.body.len() > len;
                body.bytes.truncate(len);
                Body {
                    start,
                    total_len,
                    ..body
                }
            }
            _ => ignored(None),
        },
        (Some(200), ByteRange::Prefix { .. }) => body,
        (Some(200), _) if capped => ignored(length),
        (Some(200), _) => {
            let total = outcome.body.len() as u64;
            let within = range.within(total);
            let slice = within.start as usize..within.end as usize;
            Body {
                bytes: outcome.body[slice].to_vec(),
                start: within.start,
                total_len: Some(total),
                ..body
            }
        }
        (Some(416), _) => Body {
            bytes: Vec::new(),
            total_len: header("content-range")
                .and_then(content_range)
                .and_then(|(_, total)| total),
            ..body
        },
        _ => body,
    }
}

/// A `Content-Range` value's first byte and the whole's length (`bytes 0-99/1234`, `bytes 0-99/*`, `bytes */1234`).
fn content_range(value: &str) -> Option<(Option<u64>, Option<u64>)> {
    let (span, total) = value.strip_prefix("bytes ")?.split_once('/')?;
    let start = match span.trim() {
        "*" => None,
        span => Some(span.split_once('-')?.0.trim().parse().ok()?),
    };
    let total = match total.trim() {
        "*" => None,
        total => Some(total.parse().ok()?),
    };
    Some((start, total))
}

/// A probe's data as a record keeps it: its JSON, on one line; a body's bytes follow it as they came, which a
/// request's cap holds with the [`HEAD_ROOM`] (bytes in JSON take up to four times their size).
fn encode(data: &OutcomeData) -> Vec<u8> {
    match data {
        OutcomeData::Http {
            status,
            final_url,
            headers,
            body,
            truncated,
            start,
            total_len,
        } => {
            let head = OutcomeData::Http {
                status: *status,
                final_url: final_url.clone(),
                headers: headers.clone(),
                body: Vec::new(),
                truncated: *truncated,
                start: *start,
                total_len: *total_len,
            };
            let mut bytes = serde_json::to_vec(&head).expect("outcome data serialises");
            bytes.push(b'\n');
            bytes.extend_from_slice(body);
            bytes
        }
        other => serde_json::to_vec(other).expect("outcome data serialises"),
    }
}

/// What [`encode`] wrote; none for a record of another shape, whose class says all it can.
fn decode(payload: &[u8]) -> OutcomeData {
    let (head, body) = match payload.iter().position(|byte| *byte == b'\n') {
        Some(at) => (&payload[..at], &payload[at + 1..]),
        None => (payload, &[][..]),
    };
    match serde_json::from_slice(head) {
        Ok(OutcomeData::Http {
            status,
            final_url,
            headers,
            truncated,
            start,
            total_len,
            ..
        }) => OutcomeData::Http {
            status,
            final_url,
            headers,
            body: body.to_vec(),
            truncated,
            start,
            total_len,
        },
        Ok(other) => other,
        Err(_) => OutcomeData::None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_record_keeps_a_probes_data_as_it_was() {
        for data in [
            OutcomeData::Http {
                status: Some(200),
                final_url: "https://a.example/".into(),
                headers: vec![Header::plain("etag", "x")],
                body: b"line one\nline two".to_vec(),
                truncated: true,
                start: 7,
                total_len: Some(99),
            },
            OutcomeData::Resolve {
                addresses: vec!["192.0.2.1".into()],
            },
            OutcomeData::Tls {
                not_after: Some("2030-01-01T00:00:00Z".into()),
                issuer: Some("Test CA".into()),
                chain_valid: true,
            },
        ] {
            assert_eq!(decode(&encode(&data)), data);
        }
        assert_eq!(decode(b"not json"), OutcomeData::None);
    }
}
