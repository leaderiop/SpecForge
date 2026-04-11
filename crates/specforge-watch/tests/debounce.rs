use specforge_watch::Debouncer;
use specforge_test_macros::test as spec;
use std::sync::mpsc;
use std::time::Duration;

#[spec(behavior = "debounce_file_changes", verify = "rapid successive changes coalesced into single batch")]
#[test]
fn rapid_successive_changes_coalesced_into_single_batch() {
    let (tx, rx) = mpsc::channel();
    let debouncer = Debouncer::new(Duration::from_millis(50));

    // Send 3 rapid changes
    tx.send("a.spec".to_string()).unwrap();
    tx.send("b.spec".to_string()).unwrap();
    tx.send("a.spec".to_string()).unwrap(); // duplicate

    let batch = debouncer.coalesce(&rx).unwrap();

    // Should be deduplicated and sorted
    assert_eq!(batch, vec!["a.spec", "b.spec"]);
}

#[spec(behavior = "debounce_file_changes", verify = "single isolated change triggers after debounce window")]
#[test]
fn single_isolated_change_triggers_after_debounce_window() {
    let (tx, rx) = mpsc::channel();
    let debouncer = Debouncer::new(Duration::from_millis(20));

    tx.send("only.spec".to_string()).unwrap();

    let batch = debouncer.coalesce(&rx).unwrap();
    assert_eq!(batch, vec!["only.spec"]);
}

#[spec(behavior = "debounce_file_changes")]
#[test]
fn closed_channel_returns_none() {
    let (tx, rx) = mpsc::channel::<String>();
    let debouncer = Debouncer::new(Duration::from_millis(20));

    drop(tx);

    let result = debouncer.coalesce(&rx);
    assert!(result.is_none());
}

#[spec(behavior = "debounce_file_changes", verify = "debounce window prevents redundant recompilation")]
#[test]
fn debounce_window_prevents_redundant_recompilation() {
    let (tx, rx) = mpsc::channel();
    let debouncer = Debouncer::new(Duration::from_millis(50));

    // Send the same file 5 times in rapid succession
    for _ in 0..5 {
        tx.send("same.spec".to_string()).unwrap();
    }

    let batch = debouncer.coalesce(&rx).unwrap();

    // Should produce exactly one entry — meaning one recompilation, not five
    assert_eq!(batch.len(), 1, "5 changes to same file should produce 1 batch entry, not {}", batch.len());
    assert_eq!(batch[0], "same.spec");
}

#[spec(behavior = "debounce_file_changes", verify = "coalesced batch includes union of all changed files")]
#[test]
fn coalesced_batch_includes_union_of_all_changed_files() {
    let (tx, rx) = mpsc::channel();
    let debouncer = Debouncer::new(Duration::from_millis(50));

    tx.send("x.spec".to_string()).unwrap();
    tx.send("y.spec".to_string()).unwrap();
    tx.send("z.spec".to_string()).unwrap();

    let batch = debouncer.coalesce(&rx).unwrap();
    assert_eq!(batch.len(), 3);
    assert!(batch.contains(&"x.spec".to_string()));
    assert!(batch.contains(&"y.spec".to_string()));
    assert!(batch.contains(&"z.spec".to_string()));
}
