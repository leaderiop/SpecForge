//! The `http` port: one request, through the call's effects, under its grant.

use std::sync::Arc;

use specforge_effects::http::{self, guest_body_cap};
use specforge_effects::{EffectRequest, OutcomeClass, Served};
use specforge_protocol_types::{HostError, HttpRequest, HttpResponse};

use super::record::{HttpHead, opened, sealed};
use super::{
    PortSession, answered, asked_cap, failed, held, lent_record, millis, not_in_journal,
    not_performed, now, out_of_scope,
};

impl PortSession {
    /// One HTTP request, through the call's effects, under its grant (which attaches its secret on the host side).
    pub fn http(&mut self, request: HttpRequest) -> Result<HttpResponse, HostError> {
        let (kind, call) = self.lent()?;
        // What only the host may set, whatever a guest (in process, a Rust value) put there.
        let request = HttpRequest {
            allow_private: false,
            secret: None,
            body_cap: asked_cap(request.body_cap, guest_body_cap(u64::from(self.memory_mb))),
            timeout_ms: asked_cap(request.timeout_ms, millis(http::TIMEOUT_CEILING)),
            max_redirects: request.max_redirects.min(http::MAX_REDIRECTS),
            ..request
        };
        let asked = EffectRequest::Http(request.clone());
        let lent = lent_record(call, Self::granted(kind, call, &asked)?);
        // Only what runs is held to what the budget left: a record is found by what was asked.
        let run = HttpRequest {
            timeout_ms: request.timeout_ms.min(self.budget_left()),
            ..request.clone()
        };
        let mut live = None;
        let served = call.effects.serve_lent(lent, &asked, now(), |ports| {
            let granted = call.grant.http(kind, Arc::clone(&ports.http));
            let outcome = granted.request(&run, &http::Any);
            let payload = sealed(&HttpHead::of(&outcome), &[&outcome.body]);
            let answer = answered(outcome.class, &outcome.body, payload);
            live = Some(outcome);
            answer
        });
        let (class, status, headers, body, final_url, detail) = match (served, live) {
            (Served::Outcome { .. }, Some(outcome)) => (
                outcome.class,
                outcome.status,
                outcome.headers,
                outcome.body,
                outcome.final_url,
                outcome.detail,
            ),
            (Served::Outcome { answer, .. }, None) => {
                let Some((head, body)) = answer.payload.as_deref().and_then(opened::<HttpHead>)
                else {
                    return Err(not_in_journal(&asked));
                };
                (
                    answer.outcome.class,
                    head.status,
                    head.headers,
                    body.to_vec(),
                    head.final_url,
                    head.detail,
                )
            }
            (Served::Miss, _) => return Err(not_in_journal(&asked)),
            (_, _) => return Err(not_performed(&asked)),
        };
        let detail = detail.unwrap_or_else(|| class.to_string());
        match (class, status) {
            (OutcomeClass::Denied, _) => Err(out_of_scope(detail)),
            (OutcomeClass::Ok | OutcomeClass::Status(_) | OutcomeClass::Capped, Some(status)) => {
                let (body, cut) = held(body, self.answer_cap);
                Ok(HttpResponse {
                    status,
                    headers,
                    body,
                    truncated: cut || class == OutcomeClass::Capped,
                    final_url,
                })
            }
            _ => Err(failed(class, detail)),
        }
    }
}
