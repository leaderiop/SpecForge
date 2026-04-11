use specforge_common::{Diagnostic, Severity, SourceSpan, Sym};
use specforge_watch::{
    compute_diagnostics_delta, notify_delta_subscribers, DeltaSubscriber, DiagnosticsDelta,
    GraphDelta,
};
use specforge_test_macros::test as spec;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

fn empty_delta() -> GraphDelta {
    GraphDelta {
        added_nodes: vec![],
        removed_nodes: vec![],
        modified_nodes: vec![],
        added_edges: vec![],
        removed_edges: vec![],
        affected_files: vec![],
    }
}

fn empty_diag_delta() -> DiagnosticsDelta {
    DiagnosticsDelta {
        added: vec![],
        removed: vec![],
    }
}

fn make_diag(code: &str, file: &str, line: usize) -> Diagnostic {
    Diagnostic {
        code: code.to_string(),
        severity: Severity::Error,
        message: format!("{} at {}:{}", code, file, line),
        span: Some(SourceSpan {
            file: Sym::new(file),
            start_line: line,
            start_col: 0,
            end_line: line,
            end_col: 0,
        }),
        suggestion: None,
    }
}

struct CountingSubscriber {
    call_count: Arc<AtomicUsize>,
}

impl DeltaSubscriber for CountingSubscriber {
    fn on_delta(&self, _delta: &GraphDelta, _diag_delta: &DiagnosticsDelta, _affected_files: &[String]) {
        self.call_count.fetch_add(1, Ordering::SeqCst);
    }
}

struct SlowSubscriber {
    done: Arc<AtomicUsize>,
}

impl DeltaSubscriber for SlowSubscriber {
    fn on_delta(&self, _delta: &GraphDelta, _diag_delta: &DiagnosticsDelta, _affected_files: &[String]) {
        std::thread::sleep(std::time::Duration::from_millis(100));
        self.done.fetch_add(1, Ordering::SeqCst);
    }
}

struct AffectedFilesRecorder {
    files: Arc<Mutex<Vec<String>>>,
}

impl DeltaSubscriber for AffectedFilesRecorder {
    fn on_delta(&self, _delta: &GraphDelta, _diag_delta: &DiagnosticsDelta, affected_files: &[String]) {
        let mut f = self.files.lock().unwrap();
        f.extend(affected_files.iter().cloned());
    }
}

// ── subscriber receives delta notification ────────────────────

#[spec(behavior = "notify_delta_subscribers")]
#[test]
fn subscriber_receives_delta_notification() {
    let count = Arc::new(AtomicUsize::new(0));
    let subscriber = CountingSubscriber {
        call_count: count.clone(),
    };

    let subscribers: Vec<Box<dyn DeltaSubscriber>> = vec![Box::new(subscriber)];
    let delta = empty_delta();
    let diag_delta = empty_diag_delta();

    notify_delta_subscribers(&subscribers, &delta, &diag_delta);

    assert_eq!(count.load(Ordering::SeqCst), 1);
}

// ── multiple subscribers all notified ─────────────────────────

#[spec(behavior = "notify_delta_subscribers")]
#[test]
fn multiple_subscribers_all_notified() {
    let count = Arc::new(AtomicUsize::new(0));
    let subscribers: Vec<Box<dyn DeltaSubscriber>> = (0..3)
        .map(|_| {
            Box::new(CountingSubscriber {
                call_count: count.clone(),
            }) as Box<dyn DeltaSubscriber>
        })
        .collect();

    let delta = empty_delta();
    let diag_delta = empty_diag_delta();

    notify_delta_subscribers(&subscribers, &delta, &diag_delta);

    assert_eq!(count.load(Ordering::SeqCst), 3);
}

// ── slow subscriber does not block pipeline ───────────────────

#[spec(behavior = "notify_delta_subscribers", verify = "slow subscriber does not block pipeline")]
#[test]
fn slow_subscriber_does_not_block_other_subscribers() {
    let fast_count = Arc::new(AtomicUsize::new(0));
    let slow_done = Arc::new(AtomicUsize::new(0));

    let subscribers: Vec<Box<dyn DeltaSubscriber>> = vec![
        Box::new(SlowSubscriber {
            done: slow_done.clone(),
        }),
        Box::new(CountingSubscriber {
            call_count: fast_count.clone(),
        }),
    ];

    let delta = empty_delta();
    let diag_delta = empty_diag_delta();

    notify_delta_subscribers(&subscribers, &delta, &diag_delta);

    assert_eq!(fast_count.load(Ordering::SeqCst), 1);
    assert_eq!(slow_done.load(Ordering::SeqCst), 1);
}

// ── LSP receives semantic token updates for affected files ────

#[spec(behavior = "notify_delta_subscribers", verify = "LSP receives semantic token updates for affected files")]
#[test]
fn lsp_subscriber_receives_affected_files() {
    let files = Arc::new(Mutex::new(Vec::new()));
    let subscriber = AffectedFilesRecorder {
        files: files.clone(),
    };

    let subscribers: Vec<Box<dyn DeltaSubscriber>> = vec![Box::new(subscriber)];
    let delta = GraphDelta {
        added_nodes: vec![],
        removed_nodes: vec![],
        modified_nodes: vec![],
        added_edges: vec![],
        removed_edges: vec![],
        affected_files: vec!["a.spec".to_string(), "b.spec".to_string()],
    };
    let diag_delta = empty_diag_delta();

    notify_delta_subscribers(&subscribers, &delta, &diag_delta);

    let received = files.lock().unwrap();
    assert!(received.contains(&"a.spec".to_string()));
    assert!(received.contains(&"b.spec".to_string()));
}

// ── diagnostics delta includes added and removed ──────────────

#[spec(behavior = "notify_delta_subscribers", verify = "diagnostics delta includes added and removed")]
#[test]
fn diagnostics_delta_includes_added_and_removed() {
    let old_diags = vec![
        make_diag("E001", "a.spec", 1),
        make_diag("E002", "a.spec", 5),
    ];

    let new_diags = vec![
        make_diag("E001", "a.spec", 1), // same — not in delta
        make_diag("E003", "b.spec", 10), // new — added
    ];

    let delta = compute_diagnostics_delta(&old_diags, &new_diags);

    assert_eq!(delta.added.len(), 1);
    assert_eq!(delta.added[0].code, "E003");

    assert_eq!(delta.removed.len(), 1);
    assert_eq!(delta.removed[0].code, "E002");
}

#[spec(behavior = "notify_delta_subscribers")]
#[test]
fn no_changes_produces_empty_diagnostics_delta() {
    let diags = vec![make_diag("E001", "a.spec", 1)];
    let delta = compute_diagnostics_delta(&diags, &diags);

    assert_eq!(delta.added.len(), 0);
    assert_eq!(delta.removed.len(), 0);
}
