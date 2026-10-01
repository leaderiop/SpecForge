use std::sync::Arc;
use tokio::sync::RwLock;

use specforge_lsp::LspState;
use specforge_project::SourceChange;
use specforge_test_macros::test as spec;

/// Helper: create an `Arc<RwLock<LspState>>` pre-loaded with a single document.
fn shared_state_with_document() -> Arc<RwLock<LspState>> {
    let mut state = LspState::new();
    state.open_document("file:///test.spec", "behavior foo \"test\" {}\n");
    Arc::new(RwLock::new(state))
}

// -- concurrent_read_safety ------------------------------------------------------

#[spec(
    invariant = "lsp_state_concurrency_safety",
    verify = "multiple concurrent readers complete without blocking each other"
)]
#[tokio::test]
async fn concurrent_reads_complete() {
    let state = shared_state_with_document();

    // While one reader holds the state, another still gets in — and a
    // writer does not.
    {
        let held = state.read().await;
        let other = state.clone();
        let second = tokio::spawn(async move {
            let s = other.read().await;
            s.document("file:///test.spec")
                .unwrap()
                .content()
                .to_string()
        });
        let content = tokio::time::timeout(std::time::Duration::from_secs(5), second)
            .await
            .expect("a second reader waited on the first")
            .unwrap();
        assert_eq!(content, "behavior foo \"test\" {}\n");
        assert!(state.try_write().is_err(), "a writer must wait for readers");
        drop(held);
    }

    // All twenty readers hold the state at once: each waits, guard in
    // hand, until every other one has its guard too.
    let all_in = Arc::new(tokio::sync::Barrier::new(20));
    let mut handles = Vec::new();
    for i in 0..20 {
        let state = state.clone();
        let all_in = all_in.clone();
        handles.push(tokio::spawn(async move {
            let s = state.read().await;
            all_in.wait().await;
            assert!(
                s.is_open("file:///test.spec"),
                "reader {i} must see open doc"
            );
            let doc = s.document("file:///test.spec").unwrap();
            assert!(
                doc.content().contains("behavior foo"),
                "reader {i} content mismatch"
            );
            s.open_uris().len()
        }));
    }

    for handle in handles {
        let count = tokio::time::timeout(std::time::Duration::from_secs(5), handle)
            .await
            .expect("readers blocked each other")
            .expect("reader task must not panic");
        assert_eq!(count, 1);
    }
}

#[spec(
    invariant = "lsp_state_concurrency_safety",
    verify = "concurrent readers see consistent graph and document state"
)]
#[tokio::test]
async fn concurrent_reads_see_consistent_state() {
    // Version `v` of the document declares the single entity `entity_v`.
    const URI: &str = "file:///p/versioned.spec";
    const PATH: &str = "/p/versioned.spec";
    let text = |v: usize| format!("behavior entity_{v} \"Version {v}\" {{\n}}\n");

    let state = Arc::new(RwLock::new(LspState::new()));
    {
        let mut s = state.write().await;
        s.open_document(URI, &text(1));
        s.session_mut().unwrap().update(SourceChange::Buffer {
            path: PATH,
            text: Some(&text(1)),
        });
    }

    /// What a reader sees: the buffer's version and the versions of the
    /// entities in the graph.
    async fn observe(state: &RwLock<LspState>) -> (usize, Vec<usize>) {
        let s = state.read().await;
        let version = |id: &str| id.trim_start_matches("entity_").parse::<usize>().unwrap();
        let content = s.document(URI).unwrap().content().to_string();
        let doc = version(content.split_whitespace().nth(1).unwrap());
        let graph = s
            .graph()
            .nodes()
            .iter()
            .map(|n| version(n.id.raw.as_str()))
            .collect();
        (doc, graph)
    }
    // Consistent: the graph is one complete build, of the buffer's
    // version or (while a recompile is running) the one before it.
    let assert_consistent = |(doc, graph): (usize, Vec<usize>)| {
        assert_eq!(
            graph.len(),
            1,
            "half-applied graph {graph:?} for version {doc}"
        );
        assert!(
            graph[0] <= doc && graph[0] + 1 >= doc,
            "graph {graph:?}, buffer {doc}"
        );
    };

    // The writer updates the way the server does: the buffer first, then a
    // recompile with the session taken out of the state, then put back.
    let writer = {
        let state = state.clone();
        tokio::spawn(async move {
            for v in 2..=30 {
                let mut session = {
                    let mut s = state.write().await;
                    s.open_document(URI, &text(v));
                    s.take_session().expect("no other update is running")
                };
                // A reader during the recompile.
                assert_consistent(observe(&state).await);
                session.update(SourceChange::Buffer {
                    path: PATH,
                    text: Some(&text(v)),
                });
                tokio::task::yield_now().await;
                state.write().await.set_session(session);
            }
        })
    };
    let mut readers = Vec::new();
    for _ in 0..10 {
        let state = state.clone();
        readers.push(tokio::spawn(async move {
            for _ in 0..30 {
                assert_consistent(observe(&state).await);
                tokio::task::yield_now().await;
            }
        }));
    }
    writer.await.expect("writer panicked");
    for reader in readers {
        reader.await.expect("a reader saw an inconsistent state");
    }
    assert_eq!(observe(&state).await, (30, vec![30]));
}

// -- read_write_interleaving -----------------------------------------------------

#[spec(
    invariant = "lsp_state_concurrency_safety",
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
    behavior = "document_open_close",
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
    invariant = "lsp_state_concurrency_safety",
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
