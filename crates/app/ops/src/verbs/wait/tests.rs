use std::cell::{Cell, RefCell};

use super::*;

/// A timer that moves only when paused.
#[derive(Default)]
struct Fake {
    now: Cell<Duration>,
    pauses: RefCell<Vec<Duration>>,
}

impl Timer for Fake {
    fn elapsed(&self) -> Duration {
        self.now.get()
    }

    fn pause(&self, span: Duration) {
        self.now.set(self.now.get() + span);
        self.pauses.borrow_mut().push(span);
    }
}

const MS: fn(u64) -> Duration = Duration::from_millis;

#[test]
fn with_no_condition_it_pauses_its_whole_time_and_checks_nothing() {
    let timer = Fake::default();
    let ended = polled(MS(3000), false, Poll::BACK_OFF, &timer, || {
        panic!("no condition is checked")
    });
    assert_eq!(ended.unwrap(), (Reason::Elapsed, 0));
    assert_eq!(*timer.pauses.borrow(), [MS(3000)]);
}

#[test]
fn it_checks_at_once_then_backs_off_to_the_bound_and_once_at_the_deadline() {
    let timer = Fake::default();
    let mut at = Vec::new();
    let ended = polled(MS(3000), true, Poll::BACK_OFF, &timer, || {
        at.push(millis(timer.elapsed()));
        Ok(None)
    });
    assert_eq!(ended.unwrap(), (Reason::Deadline, 7));
    assert_eq!(at, [0, 100, 300, 700, 1500, 2500, 3000]);
}

#[test]
fn it_ends_at_the_first_check_that_holds() {
    let timer = Fake::default();
    let mut checks = 0;
    let ended = polled(MS(30_000), true, Poll::BACK_OFF, &timer, || {
        checks += 1;
        Ok((checks == 3).then_some(Reason::Answered))
    });
    assert_eq!(ended.unwrap(), (Reason::Answered, 3));
    assert_eq!(timer.elapsed(), MS(300));
}

#[test]
fn a_wait_of_zero_checks_once() {
    let timer = Fake::default();
    let ended = polled(Duration::ZERO, true, Poll::BACK_OFF, &timer, || Ok(None));
    assert_eq!(ended.unwrap(), (Reason::Deadline, 1));
    assert!(timer.pauses.borrow().is_empty());
}

#[test]
fn a_retry_after_is_read_in_every_form_and_held_to_a_day() {
    let now = std::time::UNIX_EPOCH + Duration::from_secs(784_111_777); // Sun, 06 Nov 1994 08:49:37 GMT
    for (value, waits) in [
        ("120", MS(120_000)),
        ("0", Duration::ZERO),
        ("1.5", MS(1500)),
        ("0.0001", MS(1)),
        ("86401", LONGEST),
        ("99999999999999999999", LONGEST),
        ("Sun, 06 Nov 1994 08:50:37 GMT", MS(60_000)),
        ("Sunday, 06-Nov-94 08:50:37 GMT", MS(60_000)),
        ("Sun Nov  6 08:50:37 1994", MS(60_000)),
        ("Sun, 06 Nov 1994 08:48:37 GMT", Duration::ZERO),
        ("Mon, 07 Nov 1994 08:49:38 GMT", LONGEST),
    ] {
        assert_eq!(retry_after(value, now), Ok(waits), "{value}");
    }
    for value in ["", "-5", "1e3", "inf", "1.2.3", "soon", "5m"] {
        assert!(retry_after(value, now).is_err(), "{value}");
    }
}

/// Each probe takes the time the wait has left, held to the poll's first pause at the deadline, and every probe
/// of a URL is found by one record whatever `--for` the wait was given.
#[test]
fn each_probe_takes_the_time_left_and_one_record_serves_any_for() {
    use std::sync::Arc;

    use specforge_effects::Ports;
    use specforge_effects::testing::{FakeHttp, MemoryConsent, ports};
    use specforge_project::testing::TestProject;

    let project = TestProject::new();
    // Every URL is unreachable (`unreachable:dns`): no server yet.
    let http = Arc::new(FakeHttp::new());
    let verbs = Verbs::open(
        &project.project_root(),
        project.root(),
        &Ports {
            http: Arc::clone(&http) as _,
            ..ports()
        },
        &MemoryConsent::approving_everything(),
    );
    let waited = |for_ms| {
        let asked = Wait {
            for_: MS(for_ms),
            url: Some("http://server.test/".to_string()),
            changed: None,
            matching: None,
            mode: EffectsMode::Online,
        };
        wait(&verbs, &asked, Poll::BACK_OFF, &Fake::default()).unwrap()
    };
    let short = waited(3000);
    assert_eq!(short.answer.reason, Reason::Deadline);
    let timeouts: Vec<u64> = http.requests().iter().map(|r| r.timeout_ms).collect();
    assert_eq!(timeouts, [3000, 2900, 2700, 2300, 1500, 500, 100]);
    let long = waited(30_000);
    assert!(short.effect.digest.is_some());
    assert_eq!(short.effect.digest, long.effect.digest);
}

/// The verbs over `project` and the real disk.
fn on_disk(project: &specforge_project::testing::TestProject) -> Verbs {
    Verbs::open(
        &project.project_root(),
        project.root(),
        &specforge_effects::Ports {
            fs: std::sync::Arc::new(specforge_fs::LocalFs),
            ..specforge_effects::testing::ports()
        },
        &specforge_effects::testing::MemoryConsent::approving_everything(),
    )
}

/// A walk stopped at its bound would answer a quiet it never read: it is refused, with the remedy.
#[test]
fn a_walk_that_stops_at_its_bound_is_refused() {
    let project = specforge_project::testing::TestProject::new().files(&[
        ("w/a.txt", "a"),
        ("w/b.txt", "b"),
        ("w/c.txt", "c"),
    ]);
    let verbs = on_disk(&project);
    let changed = Changed {
        glob: "w/*.txt".to_string(),
        since: Some(UnixMillis(0)),
    };
    let mut watched = Watched::judged(&verbs, &changed, UnixMillis(0)).unwrap();
    assert_eq!(watched.changed(&verbs).unwrap().map(|c| c.len()), Some(3));
    watched.max = 2;
    let refused = watched.changed(&verbs).unwrap_err();
    assert_eq!(refused.kind, OpErrorKind::PreconditionFailed);
    assert!(
        refused.message.contains("narrow the glob"),
        "{}",
        refused.message
    );
    // So is a walk that could not enter a directory.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        let locked = project.root().join("w/locked");
        std::fs::create_dir_all(&locked).unwrap();
        std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o000)).unwrap();
        watched.max = files::LIST_MAX;
        let refused = watched.changed(&verbs);
        std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o700)).unwrap();
        let refused = refused.unwrap_err();
        assert!(
            refused.message.contains("w/locked could not be read"),
            "{}",
            refused.message
        );
    }
}

#[test]
fn a_refusal_ends_the_wait_at_once() {
    let timer = Fake::default();
    let ended = polled(MS(30_000), true, Poll::BACK_OFF, &timer, || {
        Err(OpError::new(OpErrorKind::PermissionDenied, "x", "refused"))
    });
    assert_eq!(ended.unwrap_err().code, "x");
    assert_eq!(timer.elapsed(), Duration::ZERO);
}
