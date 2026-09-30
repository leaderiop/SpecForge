use crate::delta::GraphDelta;
use serde::Serialize;
use specforge_common::Diagnostic;
use std::sync::Arc;

/// Delta of diagnostics between two compilation cycles.
#[derive(Debug, Clone, Serialize)]
pub struct DiagnosticsDelta {
    pub added: Vec<Diagnostic>,
    pub removed: Vec<Diagnostic>,
}

/// Compute the diagnostics delta between old and new diagnostic sets.
pub fn compute_diagnostics_delta(
    old_diagnostics: &[Diagnostic],
    new_diagnostics: &[Diagnostic],
) -> DiagnosticsDelta {
    // Use (code, file, start_line) as a rough identity for diagnostics
    fn diag_key(d: &Diagnostic) -> (String, String, usize) {
        let file = d
            .span
            .as_ref()
            .map(|s| s.file.to_string())
            .unwrap_or_default();
        let line = d.span.as_ref().map(|s| s.start_line).unwrap_or(0);
        (d.code.clone(), file, line)
    }

    let old_keys: std::collections::HashSet<_> = old_diagnostics.iter().map(diag_key).collect();
    let new_keys: std::collections::HashSet<_> = new_diagnostics.iter().map(diag_key).collect();

    let added: Vec<Diagnostic> = new_diagnostics
        .iter()
        .filter(|d| !old_keys.contains(&diag_key(d)))
        .cloned()
        .collect();

    let removed: Vec<Diagnostic> = old_diagnostics
        .iter()
        .filter(|d| !new_keys.contains(&diag_key(d)))
        .cloned()
        .collect();

    DiagnosticsDelta { added, removed }
}

/// Subscriber that receives delta notifications after incremental rebuilds.
pub trait DeltaSubscriber: Send + Sync {
    fn on_delta(
        &self,
        delta: &GraphDelta,
        diagnostics_delta: &DiagnosticsDelta,
        affected_files: &[String],
    );
}

/// Dispatch delta to all subscribers. Non-blocking: each subscriber runs on
/// its own thread and this returns as soon as they are spawned, so a slow
/// subscriber neither delays the pipeline nor the other subscribers.
///
/// The returned handles let a caller wait for delivery when it needs to
/// (tests, shutdown); dropping them detaches the notifications.
pub fn notify_delta_subscribers(
    subscribers: &[Arc<dyn DeltaSubscriber>],
    delta: &GraphDelta,
    diagnostics_delta: &DiagnosticsDelta,
) -> Vec<std::thread::JoinHandle<()>> {
    let payload = Arc::new((delta.clone(), diagnostics_delta.clone()));
    subscribers
        .iter()
        .map(|subscriber| {
            let subscriber = Arc::clone(subscriber);
            let payload = Arc::clone(&payload);
            std::thread::spawn(move || {
                let (delta, diagnostics_delta) = &*payload;
                subscriber.on_delta(delta, diagnostics_delta, &delta.affected_files);
            })
        })
        .collect()
}
