use std::sync::Arc;
use tokio::sync::RwLock;

use specforge_lsp::LspState;
use specforge_test_macros::test as spec;

/// Helper: create an `Arc<RwLock<LspState>>` pre-loaded with a single document.
fn shared_state_with_document() -> Arc<RwLock<LspState>> {
    let mut state = LspState::new();
    state.open_document("file:///test.spec", "behavior foo \"test\" {}\n");
    Arc::new(RwLock::new(state))
}

// -- concurrent_read_safety ------------------------------------------------------

#[spec(
    behavior = "concurrent_read_safety",
    verify = "multiple concurrent readers complete without blocking each other"
)]
#[tokio::test]
async fn concurrent_reads_complete() {
    let state = shared_state_with_document();

    let mut handles = Vec::new();
    for i in 0..20 {
        let state = state.clone();
        handles.push(tokio::spawn(async move {
            let s = state.read().await;
            assert!(s.is_open("file:///test.spec"), "reader {i} must see open doc");
            let doc = s.document("file:///test.spec").unwrap();
            assert!(doc.content().contains("behavior foo"), "reader {i} content mismatch");
            s.open_uris().len()
        }));
    }

    for handle in handles {
        let count = handle.await.expect("reader task must not panic");
        assert_eq!(count, 1);
    }
}

#[spec(
    behavior = "concurrent_read_safety",
    verify = "concurrent readers see consistent graph and document state"
)]
#[tokio::test]
async fn concurrent_reads_see_consistent_state() {
    let state = shared_state_with_document();

    // Pre-populate with multiple documents
    {
        let mut s = state.write().await;
        s.open_document("file:///a.spec", "behavior a \"Alpha\" {}\n");
        s.open_document("file:///b.spec", "behavior b \"Beta\" {}\n");
        s.open_document("file:///c.spec", "behavior c \"Gamma\" {}\n");
    }

    let mut handles = Vec::new();
    for _ in 0..20 {
        let state = state.clone();
        handles.push(tokio::spawn(async move {
            let s = state.read().await;
            let uris = s.open_uris();
            // Must see all 4 documents (test.spec + a/b/c)
            assert_eq!(uris.len(), 4, "all readers must see exactly 4 open documents");
            assert!(s.is_open("file:///a.spec"));
            assert!(s.is_open("file:///b.spec"));
            assert!(s.is_open("file:///c.spec"));
            assert_eq!(s.graph().node_count(), 0, "graph starts empty");
        }));
    }

    for handle in handles {
        handle.await.expect("reader task must not panic");
    }
}

// -- read_write_interleaving -----------------------------------------------------

#[spec(
    behavior = "read_write_interleaving",
    verify = "interleaved read and write operations do not deadlock"
)]
#[tokio::test]
async fn read_write_interleaving_no_deadlock() {
    let state = Arc::new(RwLock::new(LspState::new()));

    let mut handles = Vec::new();

    // Spawn writer tasks that open documents
    for i in 0..10 {
        let state = state.clone();
        handles.push(tokio::spawn(async move {
            let uri = format!("file:///doc_{i}.spec");
            let content = format!("behavior doc_{i} \"Doc {i}\" {{}}\n");
            let mut s = state.write().await;
            s.open_document(&uri, &content);
        }));
    }

    // Spawn reader tasks that query state (some docs may or may not be open yet)
    for _ in 0..10 {
        let state = state.clone();
        handles.push(tokio::spawn(async move {
            let s = state.read().await;
            // We don't assert exact counts since writes happen concurrently.
            // The key assertion: we got the lock and didn't deadlock.
            let _count = s.open_uris().len();
        }));
    }

    // All tasks must complete (no deadlock)
    for handle in handles {
        handle.await.expect("task must not panic or deadlock");
    }

    // After all tasks, verify final state is consistent
    let s = state.read().await;
    assert_eq!(s.open_uris().len(), 10, "all 10 documents must be open");
    for i in 0..10 {
        let uri = format!("file:///doc_{i}.spec");
        assert!(s.is_open(&uri), "document {uri} must be open");
    }
}

// -- rapid_open_close_stress -----------------------------------------------------

#[spec(
    behavior = "rapid_open_close_stress",
    verify = "rapid open and close cycles do not corrupt state"
)]
#[tokio::test]
async fn rapid_open_close_no_corruption() {
    let state = Arc::new(RwLock::new(LspState::new()));

    let mut handles = Vec::new();

    // Each task opens a document, reads it, modifies it, reads again, then closes it
    for i in 0..20 {
        let state = state.clone();
        handles.push(tokio::spawn(async move {
            let uri = format!("file:///rapid_{i}.spec");
            let content = format!("behavior rapid_{i} \"Rapid {i}\" {{}}\n");

            // Open
            {
                let mut s = state.write().await;
                s.open_document(&uri, &content);
            }

            // Read and verify
            {
                let s = state.read().await;
                assert!(s.is_open(&uri), "document must be open after open_document");
                let doc = s.document(&uri).unwrap();
                assert!(doc.content().contains(&format!("rapid_{i}")));
            }

            // Apply a change
            {
                let mut s = state.write().await;
                // Replace the title text (starts after the first quote)
                s.apply_change(&uri, 0, 0, 0, 0, "// edited\n");
            }

            // Read the change
            {
                let s = state.read().await;
                let doc = s.document(&uri).unwrap();
                assert!(
                    doc.content().starts_with("// edited\n"),
                    "change must be reflected in buffer"
                );
            }

            // Close
            {
                let mut s = state.write().await;
                s.close_document(&uri);
            }

            // Verify closed
            {
                let s = state.read().await;
                assert!(!s.is_open(&uri), "document must not be open after close");
            }
        }));
    }

    for handle in handles {
        handle.await.expect("rapid open/close task must not panic");
    }

    // Final state: all documents closed
    let s = state.read().await;
    assert_eq!(s.open_uris().len(), 0, "no documents should remain open");
}

#[spec(
    behavior = "rapid_open_close_stress",
    verify = "concurrent writes to different documents do not interfere"
)]
#[tokio::test]
async fn concurrent_writes_to_different_documents() {
    let state = Arc::new(RwLock::new(LspState::new()));

    // Phase 1: open all documents concurrently
    let mut handles = Vec::new();
    for i in 0..10 {
        let state = state.clone();
        handles.push(tokio::spawn(async move {
            let uri = format!("file:///cw_{i}.spec");
            let content = format!("behavior cw_{i} \"CW {i}\" {{}}\n");
            let mut s = state.write().await;
            s.open_document(&uri, &content);
        }));
    }
    for handle in handles {
        handle.await.expect("open task must not panic");
    }

    // Phase 2: apply changes to each document concurrently
    let mut handles = Vec::new();
    for i in 0..10 {
        let state = state.clone();
        handles.push(tokio::spawn(async move {
            let uri = format!("file:///cw_{i}.spec");
            let mut s = state.write().await;
            s.apply_change(&uri, 0, 0, 0, 0, "// header\n");
        }));
    }
    for handle in handles {
        handle.await.expect("change task must not panic");
    }

    // Phase 3: verify all documents got their changes
    let s = state.read().await;
    assert_eq!(s.open_uris().len(), 10);
    for i in 0..10 {
        let uri = format!("file:///cw_{i}.spec");
        let doc = s.document(&uri).expect("document must exist");
        assert!(
            doc.content().starts_with("// header\n"),
            "document {uri} must have header prepended"
        );
        assert!(
            doc.content().contains(&format!("cw_{i}")),
            "document {uri} must retain original content"
        );
    }
}
