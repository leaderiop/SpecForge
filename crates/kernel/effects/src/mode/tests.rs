use std::cell::Cell;

use super::*;
use crate::journal::HTTP_SUCCESS_TTL;
use crate::testing::ports;
use crate::{FsOp, FsRequest, HttpRequest, Outcome, OutcomeClass};

fn http(url: &str) -> EffectRequest {
    EffectRequest::Http(HttpRequest {
        method: "GET".into(),
        url: url.into(),
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

/// A journal under a fresh directory, holding `request` answered `ok` at `at`.
fn journal_with(request: &EffectRequest, when: u64) -> (tempfile::TempDir, EffectJournal) {
    let dir = tempfile::tempdir().unwrap();
    let journal = EffectJournal::at(dir.path());
    journal
        .keep(request, &answer(OutcomeClass::Ok), at(when))
        .unwrap();
    (dir, journal)
}

/// Serve `request` at `now`, counting how often the ports were asked; they answer `class`.
fn serve_as(
    effects: &Effects,
    request: &EffectRequest,
    now: u64,
    performed: &Cell<u32>,
    class: OutcomeClass,
) -> Served {
    effects.serve("links", request, at(now), |_| {
        performed.set(performed.get() + 1);
        answer(class)
    })
}

/// The user's grant, which permits every request: what a test of a mode's order is judged by.
fn user_grant() -> Grant {
    Grant::user(&crate::ceiling::EffectsCeiling::default())
}

/// As [`serve_as`], asked under [`user_grant`].
fn serve_judged(
    effects: &Effects,
    request: &EffectRequest,
    now: u64,
    performed: &Cell<u32>,
    class: OutcomeClass,
) -> Served {
    let asker = Asker {
        extension: "the user",
        op: Operation::Collect,
        scopes: &["http".to_string()],
        queries: false,
    };
    effects.serve_for(&asker, "links", request, at(now), |_| {
        performed.set(performed.get() + 1);
        answer(class)
    })
}

fn serve(effects: &Effects, request: &EffectRequest, now: u64, performed: &Cell<u32>) -> Served {
    serve_as(effects, request, now, performed, OutcomeClass::Status(500))
}

#[test]
fn only_a_request_that_asks_for_nothing_to_happen_is_answered_again() {
    let post = EffectRequest::Http(HttpRequest {
        method: "POST".into(),
        url: "https://api.osv.dev/v1/query".into(),
        body: Some(b"{}".to_vec()),
        ..HttpRequest::default()
    });
    let dir = tempfile::tempdir().unwrap();
    let journal = EffectJournal::at(dir.path());
    let digest = RecordDigest::asked_by("@t/a", &post);
    journal
        .keep_as(&digest, &post, &answer(OutcomeClass::Ok), at(0))
        .unwrap();
    let scopes = ["registries".to_string()];
    let served = |effects: &Effects, queries| {
        let asker = Asker {
            extension: "@t/a",
            op: Operation::CheckValues,
            scopes: &scopes,
            queries,
        };
        effects.serve_for(&asker, "registries", &post, at(1), |_| {
            answer(OutcomeClass::Status(500))
        })
    };
    let cached = Effects::cached(journal.clone());
    assert_eq!(
        served(&cached, false),
        Served::NotPerformed(Why::NoFreshRecord),
        "a POST no capability declares a query asked for something to happen"
    );
    assert!(
        matches!(
            served(&cached, true),
            Served::Outcome {
                from_journal: true,
                ..
            }
        ),
        "a declared query reads: its fresh record answers it"
    );
    let replay = Effects::replay(journal);
    assert!(matches!(
        served(&replay, false),
        Served::Outcome {
            from_journal: true,
            ..
        }
    ));
}

#[test]
fn cached_answers_a_fresh_record_and_performs_nothing() {
    let request = http("https://example.org/");
    let (_dir, journal) = journal_with(&request, 0);
    let effects = Effects::cached(journal);
    assert!(effects.ports().is_none(), "cached holds no port");
    let performed = Cell::new(0);
    assert_eq!(
        serve(&effects, &request, 1, &performed),
        Served::Outcome {
            answer: answer(OutcomeClass::Ok),
            from_journal: true,
            observed_at: at(0),
        }
    );
    let other = http("https://example.org/other");
    assert_eq!(
        serve(&effects, &other, 1, &performed),
        Served::NotPerformed(Why::NoFreshRecord)
    );
    let expired = HTTP_SUCCESS_TTL.as_millis() as u64;
    assert_eq!(
        serve(&effects, &request, expired, &performed),
        Served::NotPerformed(Why::NoFreshRecord)
    );
    assert_eq!(performed.get(), 0);
    let log = effects.log();
    assert_eq!(log.not_run.get("links"), Some(&2));
    assert_eq!(log.consulted.len(), 2, "each record file looked up");
    // Served expired last: the clock changes its answer no more, so it carries no expiry (ADR 0337 D5).
    assert!(log.consulted.values().all(Option::is_none));
    let fresh = Effects::cached(effects.journal().clone());
    serve(&fresh, &request, 1, &performed);
    assert_eq!(
        fresh.take_log().consulted.into_values().collect::<Vec<_>>(),
        vec![Some(at(expired))],
        "a fresh record expires with its TTL"
    );
    assert!(
        fresh.log().consulted.is_empty(),
        "taken: the next run starts empty"
    );
}

#[test]
fn auto_performs_only_an_approved_effect_and_journals_it() {
    let dir = tempfile::tempdir().unwrap();
    let journal = EffectJournal::at(dir.path());
    let effects = Effects::auto(ports(), vec![user_grant()], journal.clone());
    let performed = Cell::new(0);
    let refused = http("https://example.org/");
    assert_eq!(
        serve(&effects, &refused, 1, &performed),
        Served::NotPerformed(Why::NotApproved),
        "`serve` judges nothing, so `auto` performs nothing through it"
    );
    let asked = http("https://example.org/approved");
    assert_eq!(
        serve_judged(&effects, &asked, 1, &performed, OutcomeClass::Status(500)),
        Served::Outcome {
            answer: answer(OutcomeClass::Status(500)),
            from_journal: false,
            observed_at: at(1),
        }
    );
    assert_eq!(performed.get(), 1);
    assert!(matches!(
        journal.lookup(&RecordDigest::asked_by("the user", &asked), at(2)),
        Lookup::Fresh(_)
    ));
    // Its fresh record answers the next run without the port.
    assert!(matches!(
        serve_judged(&effects, &asked, 2, &performed, OutcomeClass::Status(500)),
        Served::Outcome {
            from_journal: true,
            ..
        }
    ));
    assert_eq!(performed.get(), 1);
}

#[test]
fn a_denied_answer_is_not_served_after_a_grant() {
    let dir = tempfile::tempdir().unwrap();
    let journal = EffectJournal::at(dir.path());
    let effects = Effects::auto(ports(), vec![user_grant()], journal.clone());
    let request = http("https://example.org/");
    let performed = Cell::new(0);
    // The port refuses it: not granted yet. No record is kept.
    assert!(matches!(
        serve_judged(&effects, &request, 1, &performed, OutcomeClass::Denied),
        Served::Outcome {
            from_journal: false,
            ..
        }
    ));
    assert_eq!(
        journal.lookup(&RecordDigest::asked_by("the user", &request), at(2)),
        Lookup::Missing
    );
    // Granted since: the next run performs it at once.
    assert_eq!(
        serve_judged(&effects, &request, 2, &performed, OutcomeClass::Ok),
        Served::Outcome {
            answer: answer(OutcomeClass::Ok),
            from_journal: false,
            observed_at: at(2),
        }
    );
    assert_eq!(performed.get(), 2);
}

#[test]
fn a_record_answers_only_the_extension_that_asked() {
    let dir = tempfile::tempdir().unwrap();
    let journal = EffectJournal::at(dir.path());
    let request = http("https://example.org/");
    let scopes = ["http".to_string()];
    let asker = |extension| Asker {
        extension,
        op: Operation::Collect,
        scopes: &scopes,
        queries: false,
    };
    let live = Effects::auto(
        ports(),
        vec![Grant::user(&Default::default())],
        journal.clone(),
    );
    let performed = Cell::new(0);
    let ask = |effects: &Effects, extension| {
        effects.serve_for(&asker(extension), "links", &request, at(1), |_| {
            performed.set(performed.get() + 1);
            answer(OutcomeClass::Ok)
        })
    };
    assert!(matches!(ask(&live, "the user"), Served::Outcome { .. }));
    assert_eq!(performed.get(), 1);
    for effects in [
        Effects::cached(journal.clone()),
        Effects::replay(journal.clone()),
    ] {
        assert!(matches!(
            ask(&effects, "the user"),
            Served::Outcome {
                from_journal: true,
                ..
            }
        ));
        assert!(
            !matches!(ask(&effects, "@other/ext"), Served::Outcome { .. }),
            "{:?}",
            effects.mode()
        );
    }
    assert_eq!(performed.get(), 1, "neither mode performs");
}

#[test]
fn online_performs_past_a_fresh_record() {
    let request = http("https://example.org/");
    let (_dir, journal) = journal_with(&request, 0);
    let effects = Effects::online(ports(), journal.clone());
    let performed = Cell::new(0);
    assert!(matches!(
        serve(&effects, &request, 1, &performed),
        Served::Outcome {
            from_journal: false,
            ..
        }
    ));
    assert_eq!(performed.get(), 1);
    let Lookup::Fresh(kept) = journal.lookup(&RecordDigest::of(&request), at(2)) else {
        panic!("the new answer replaced the record");
    };
    assert_eq!(kept.record.outcome.class, OutcomeClass::Status(500));
}

#[test]
fn replay_serves_an_expired_record_and_logs_a_miss() {
    let request = http("https://example.org/");
    let (_dir, journal) = journal_with(&request, 0);
    let effects = Effects::replay(journal);
    let performed = Cell::new(0);
    let late = HTTP_SUCCESS_TTL.as_millis() as u64 * 10;
    assert!(matches!(
        serve(&effects, &request, late, &performed),
        Served::Outcome {
            from_journal: true,
            ..
        }
    ));
    let other = http("https://example.org/other");
    assert_eq!(serve(&effects, &other, late, &performed), Served::Miss);
    let misses = effects.log().misses;
    assert_eq!(misses.len(), 1);
    assert_eq!(
        misses[0].to_string(),
        format!(
            "GET https://example.org/other ({})",
            RecordDigest::of(&other)
        )
    );
    assert_eq!(performed.get(), 0);
}

#[test]
fn fs_and_git_are_performed_in_every_mode() {
    // An fs effect is local: every mode hands it back to the caller, unjournaled. A git question is no
    // `EffectRequest`: the session's Git port answers it in every mode.
    let dir = tempfile::tempdir().unwrap();
    let read = EffectRequest::Fs(FsRequest {
        root: "project".into(),
        path: "a.txt".into(),
        op: FsOp::Read { max_bytes: 1 },
    });
    let journal = EffectJournal::at(dir.path());
    let performed = Cell::new(0);
    for mode in EffectsMode::ALL {
        let effects = Effects::of(mode, ports(), Vec::new(), journal.clone());
        assert_eq!(effects.mode(), mode);
        assert_eq!(effects.ports().is_some(), mode.is_live());
        assert_eq!(
            serve(&effects, &read, 0, &performed),
            Served::Local,
            "{mode:?}"
        );
        assert_eq!(effects.log(), Log::default());
    }
    assert_eq!(performed.get(), 0);
    assert!(!dir.path().join(journal::JOURNAL_DIR).exists());
    assert_eq!(EffectsMode::parse("replay"), Some(EffectsMode::Replay));
    assert_eq!(
        EffectsMode::NAMES.map(|n| EffectsMode::parse(n).unwrap().name()),
        EffectsMode::NAMES
    );
}
