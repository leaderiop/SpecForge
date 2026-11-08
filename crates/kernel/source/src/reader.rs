//! The reader: files loaded, digested, looked up in the cache and parsed on scoped threads, the answer in the order
//! asked and the work counted.

use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use serde::Serialize;
use sha2::Digest as _;
use specforge_common::shape::Shape;
use specforge_protocol_types::{NotParsed, SyntaxFile, SyntaxOutcome};

use crate::cache::{Entry, SyntaxCache};
use crate::parse::{Deadline, PARSE_DEADLINE, parse, parser_for};
use crate::query::digest_of;
use crate::{Language, QueryFault, SourceQuery};

/// A file as its loader gives it: its bytes, or why there are none.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Loaded {
    Bytes(Vec<u8>),
    NotParsed(NotParsed),
}

/// What a read did: files parsed now, files answered from the cache, bytes parsed now, cache writes that failed.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Shape)]
pub struct SyntaxWork {
    pub parsed: usize,
    pub cached: usize,
    pub bytes: u64,
    pub unwritten: usize,
}

impl std::ops::Add for SyntaxWork {
    type Output = SyntaxWork;

    fn add(self, other: SyntaxWork) -> SyntaxWork {
        SyntaxWork {
            parsed: self.parsed + other.parsed,
            cached: self.cached + other.cached,
            bytes: self.bytes + other.bytes,
            unwritten: self.unwritten + other.unwritten,
        }
    }
}

impl std::ops::AddAssign for SyntaxWork {
    fn add_assign(&mut self, other: SyntaxWork) {
        *self = *self + other;
    }
}

/// A read's answer: one file per path asked, in that order, and the work it took.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Read {
    pub files: Vec<SyntaxFile>,
    pub work: SyntaxWork,
}

/// The host's source reader over one cache, its compiled queries memoised.
#[derive(Debug)]
pub struct SourceReader {
    cache: SyntaxCache,
    deadline: Deadline,
    queries: Mutex<HashMap<(Language, String), Arc<SourceQuery>>>,
}

impl SourceReader {
    pub fn new(cache: SyntaxCache) -> Self {
        SourceReader {
            cache,
            deadline: PARSE_DEADLINE,
            queries: Mutex::default(),
        }
    }

    /// The same reader, each file held to `deadline` rather than [`PARSE_DEADLINE`].
    pub fn with_deadline(mut self, deadline: Deadline) -> Self {
        self.deadline = deadline;
        self
    }

    /// Its cache.
    pub fn cache(&self) -> &SyntaxCache {
        &self.cache
    }

    /// `text` compiled for `language`, once per reader.
    pub fn query(&self, language: Language, text: &str) -> Result<Arc<SourceQuery>, QueryFault> {
        let key = (language, digest_of(text));
        if let Some(query) = self.queries().get(&key) {
            return Ok(Arc::clone(query));
        }
        let query = Arc::new(SourceQuery::compile_digested(
            language,
            text,
            key.1.clone(),
        )?);
        self.queries().insert(key, Arc::clone(&query));
        Ok(query)
    }

    fn queries(&self) -> std::sync::MutexGuard<'_, HashMap<(Language, String), Arc<SourceQuery>>> {
        self.queries
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// Each of `paths` loaded by `load`, then answered from the cache or parsed with `query`: on scoped workers (at
    /// most one per file), one parser each, the answer in the order asked.
    pub fn read(
        &self,
        query: &SourceQuery,
        paths: &[String],
        load: &(dyn Fn(&str) -> Loaded + Sync),
    ) -> Read {
        let workers = std::thread::available_parallelism()
            .map_or(1, usize::from)
            .min(paths.len());
        let next = AtomicUsize::new(0);
        let mut answered: Vec<(usize, SyntaxFile, SyntaxWork)> = std::thread::scope(|scope| {
            let handles: Vec<_> = (0..workers)
                .map(|_| {
                    scope.spawn(|| {
                        let mut parser = parser_for(query);
                        let mut done = Vec::new();
                        loop {
                            let at = next.fetch_add(1, Ordering::Relaxed);
                            let Some(path) = paths.get(at) else {
                                return done;
                            };
                            let (file, work) = self.one(&mut parser, query, path, load);
                            done.push((at, file, work));
                        }
                    })
                })
                .collect();
            handles
                .into_iter()
                .flat_map(|handle| {
                    handle
                        .join()
                        .unwrap_or_else(|panic| std::panic::resume_unwind(panic))
                })
                .collect()
        });
        answered.sort_by_key(|(at, ..)| *at);
        let mut work = SyntaxWork::default();
        let files = answered
            .into_iter()
            .map(|(_, file, done)| {
                work += done;
                file
            })
            .collect();
        Read { files, work }
    }

    /// One file: loaded, digested, looked up, parsed on a miss and kept (an outcome past the deadline is not).
    fn one(
        &self,
        parser: &mut tree_sitter::Parser,
        query: &SourceQuery,
        path: &str,
        load: &(dyn Fn(&str) -> Loaded + Sync),
    ) -> (SyntaxFile, SyntaxWork) {
        let mut work = SyntaxWork::default();
        let bytes = match load(path) {
            Loaded::Bytes(bytes) => bytes,
            Loaded::NotParsed(reason) => {
                let file = SyntaxFile {
                    path: path.to_string(),
                    digest: None,
                    outcome: SyntaxOutcome::NotParsed { reason },
                };
                return (file, work);
            }
        };
        let sha256 = format!("{:x}", sha2::Sha256::digest(&bytes));
        let key = SyntaxCache::key(query.language().grammar(), query.digest(), &sha256);
        let outcome = match self.cache.get(&key) {
            Some(Entry { matches, errors }) => {
                work.cached = 1;
                SyntaxOutcome::Parsed { matches, errors }
            }
            None => {
                let outcome = parse(parser, query, &bytes, self.deadline)
                    .unwrap_or_else(|reason| SyntaxOutcome::NotParsed { reason });
                if let SyntaxOutcome::Parsed { matches, errors } = &outcome {
                    work.parsed = 1;
                    work.bytes = bytes.len() as u64;
                    let entry = Entry {
                        matches: matches.clone(),
                        errors: *errors,
                    };
                    work.unwritten = usize::from(!self.cache.put(&key, &entry));
                }
                outcome
            }
        };
        let file = SyntaxFile {
            path: path.to_string(),
            digest: Some(format!("sha256:{sha256}")),
            outcome,
        };
        (file, work)
    }
}
