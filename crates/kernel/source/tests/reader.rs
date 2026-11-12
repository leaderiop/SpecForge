//! Many files read through the cache: what each read parsed, answered from the cache and could not keep.

use std::collections::BTreeMap;
use std::time::{Duration, SystemTime};

use specforge_protocol_types::{NotParsed, SyntaxOutcome};
use specforge_source::{
    CACHE_DIR, CACHE_MAX_AGE, COUNTED_DEADLINE, Language, Loaded, Read, SourceReader, SyntaxCache,
    SyntaxWork,
};
use specforge_test_macros::test as specforge_test;

const QUERY: &str = "(function_item name: (identifier) @name) @definition.function";

/// Files held in memory, by path.
struct Files(BTreeMap<String, Vec<u8>>);

impl Files {
    fn of(files: &[(&str, &str)]) -> Self {
        Files(
            files
                .iter()
                .map(|(path, text)| (path.to_string(), text.as_bytes().to_vec()))
                .collect(),
        )
    }

    fn paths(&self) -> Vec<String> {
        self.0.keys().cloned().collect()
    }

    fn read(&self, reader: &SourceReader, query: &str) -> Read {
        let query = reader.query(Language::Rust, query).unwrap();
        reader.read(&query, &self.paths(), &|path| match self.0.get(path) {
            Some(bytes) => Loaded::Bytes(bytes.clone()),
            None => Loaded::NotParsed(NotParsed::Missing),
        })
    }
}

/// A reader over `cache`, each file held to the counted deadline.
fn counted(cache: SyntaxCache) -> SourceReader {
    SourceReader::new(cache).with_deadline(COUNTED_DEADLINE)
}

fn three() -> Files {
    Files::of(&[
        ("a.rs", "fn a() {}\n"),
        ("b.rs", "fn b() {}\n"),
        ("c.rs", "fn c() {}\n"),
    ])
}

fn names(read: &Read) -> Vec<String> {
    read.files
        .iter()
        .flat_map(|file| match &file.outcome {
            SyntaxOutcome::Parsed { matches, .. } => matches
                .iter()
                .flat_map(|m| m.captures.iter().map(|c| c.text.clone()))
                .collect(),
            SyntaxOutcome::NotParsed { reason } => vec![reason.name().to_string()],
        })
        .collect()
}

#[specforge_test(
    behavior = "read_source_syntax",
    verify = "a second read parses nothing and answers every file from the cache"
)]
fn a_second_read_parses_nothing_and_answers_every_file_from_the_cache() {
    let base = tempfile::tempdir().unwrap();
    let files = three();
    let first = files.read(&counted(SyntaxCache::at(base.path())), QUERY);
    assert_eq!(
        first.work,
        SyntaxWork {
            parsed: 3,
            cached: 0,
            bytes: 30,
            unwritten: 0,
            unwritable: None
        }
    );
    assert!(base.path().join(CACHE_DIR).is_dir());
    // Another reader on the same cache: nothing compiled or parsed is shared but the entries.
    let second = files.read(&counted(SyntaxCache::at(base.path())), QUERY);
    assert_eq!((second.work.parsed, second.work.cached), (0, 3));
    assert_eq!(second.files, first.files);
    assert_eq!(
        first.files[0]
            .digest
            .as_deref()
            .map(|d| d.starts_with("sha256:")),
        Some(true)
    );
}

#[specforge_test(
    behavior = "read_source_syntax",
    verify = "an edited file is the only one parsed again"
)]
fn an_edited_file_is_the_only_one_parsed_again() {
    let base = tempfile::tempdir().unwrap();
    let reader = counted(SyntaxCache::at(base.path()));
    three().read(&reader, QUERY);
    let edited = Files::of(&[
        ("a.rs", "fn a() {}\n"),
        ("b.rs", "fn b2() {}\n"),
        ("c.rs", "fn c() {}\n"),
    ]);
    let read = edited.read(&reader, QUERY);
    assert_eq!((read.work.parsed, read.work.cached), (1, 2));
    assert_eq!(names(&read), ["a", "b2", "c"]);
}

#[specforge_test(
    behavior = "read_source_syntax",
    verify = "another query misses the cache"
)]
fn another_query_misses_the_cache() {
    let base = tempfile::tempdir().unwrap();
    let reader = counted(SyntaxCache::at(base.path()));
    three().read(&reader, QUERY);
    let other = three().read(&reader, "(identifier) @reference.name");
    assert_eq!((other.work.parsed, other.work.cached), (3, 0));
}

#[specforge_test(
    behavior = "read_source_syntax",
    verify = "a detached cache parses every time"
)]
fn a_detached_cache_parses_every_time() {
    let reader = counted(SyntaxCache::detached());
    three().read(&reader, QUERY);
    let again = three().read(&reader, QUERY);
    assert_eq!(
        again.work,
        SyntaxWork {
            parsed: 3,
            cached: 0,
            bytes: 30,
            unwritten: 0,
            unwritable: None
        }
    );
}

/// Every entry file under the cache.
fn entries(base: &std::path::Path) -> Vec<std::path::PathBuf> {
    let mut found = Vec::new();
    for shard in std::fs::read_dir(base.join(CACHE_DIR)).unwrap().flatten() {
        found.extend(
            std::fs::read_dir(shard.path())
                .unwrap()
                .flatten()
                .map(|e| e.path()),
        );
    }
    found.sort();
    found
}

#[specforge_test(
    behavior = "read_source_syntax",
    verify = "an unreadable cache entry is a miss and is rewritten"
)]
fn an_unreadable_entry_is_a_miss_and_is_rewritten() {
    let base = tempfile::tempdir().unwrap();
    let files = Files::of(&[("a.rs", "fn a() {}\n")]);
    files.read(&counted(SyntaxCache::at(base.path())), QUERY);
    let [entry] = entries(base.path()).try_into().unwrap();
    std::fs::write(&entry, "{not json").unwrap();
    let read = files.read(&counted(SyntaxCache::at(base.path())), QUERY);
    assert_eq!((read.work.parsed, read.work.cached), (1, 0));
    assert_eq!(names(&read), ["a"]);
    let again = files.read(&counted(SyntaxCache::at(base.path())), QUERY);
    assert_eq!(again.work.cached, 1);
}

#[specforge_test(
    behavior = "read_source_syntax",
    verify = "a cache that cannot be written counts unwritten and still answers"
)]
fn a_cache_that_cannot_be_written_counts_unwritten_and_still_answers() {
    let base = tempfile::tempdir().unwrap();
    // The cache's directory is a file: no entry can go under it.
    std::fs::create_dir_all(base.path().join(".specforge/cache")).unwrap();
    std::fs::write(base.path().join(CACHE_DIR), "a file").unwrap();
    let read = three().read(&counted(SyntaxCache::at(base.path())), QUERY);
    assert_eq!((read.work.parsed, read.work.unwritten), (3, 3));
    // The first failed write's error is kept: a shard directory cannot be made under a file.
    let why = std::fs::create_dir_all(base.path().join(CACHE_DIR).join("00"))
        .unwrap_err()
        .to_string();
    assert_eq!(read.work.unwritable, Some(why));
    assert_eq!(names(&read), ["a", "b", "c"]);
}

#[specforge_test(
    behavior = "read_source_syntax",
    verify = "files of one content read at once are all cached and none is unwritten"
)]
fn files_of_one_content_read_at_once_are_all_cached() {
    let base = tempfile::tempdir().unwrap();
    // One key for every file: the workers parse and write it at once.
    let paths: Vec<String> = (0..64).map(|n| format!("f{n}.rs")).collect();
    let same: Vec<(&str, &str)> = paths
        .iter()
        .map(|p| (p.as_str(), "fn same() {}\n"))
        .collect();
    let read = Files::of(&same).read(&counted(SyntaxCache::at(base.path())), QUERY);
    assert_eq!(read.work.parsed + read.work.cached, 64);
    assert_eq!(
        (read.work.unwritten, read.work.unwritable),
        (0, None),
        "a worker failed on another's write of the same entry"
    );
    assert_eq!(
        entries(base.path()).len(),
        1,
        "one entry, no temporary left"
    );
}

#[specforge_test(
    behavior = "read_source_syntax",
    verify = "a prune removes the entries untouched past the age"
)]
fn prune_removes_entries_untouched_past_the_age() {
    let base = tempfile::tempdir().unwrap();
    let cache = SyntaxCache::at(base.path());
    three().read(&counted(cache.clone()), QUERY);
    let now = SystemTime::now();
    assert_eq!(cache.prune(now, CACHE_MAX_AGE), 0);
    assert_eq!(entries(base.path()).len(), 3);
    let later = now + CACHE_MAX_AGE + Duration::from_secs(60);
    assert_eq!(cache.prune(later, CACHE_MAX_AGE), 3);
    assert!(entries(base.path()).is_empty());
    assert_eq!(SyntaxCache::detached().prune(later, CACHE_MAX_AGE), 0);
}

#[specforge_test(
    behavior = "read_source_syntax",
    verify = "a read answers in the order asked whatever the workers"
)]
fn the_answer_keeps_the_order_asked_whatever_the_workers() {
    let texts: Vec<(String, String)> = (0..64)
        .map(|n| (format!("f{n:02}.rs"), format!("fn f{n}() {{}}\n")))
        .collect();
    let files = Files(
        texts
            .iter()
            .map(|(path, text)| (path.clone(), text.as_bytes().to_vec()))
            .collect(),
    );
    let reader = counted(SyntaxCache::detached());
    let query = reader.query(Language::Rust, QUERY).unwrap();
    // Asked in reverse, with one missing file in the middle.
    let mut paths: Vec<String> = files.paths().into_iter().rev().collect();
    paths.insert(32, "missing.rs".to_string());
    let read = reader.read(&query, &paths, &|path| match files.0.get(path) {
        Some(bytes) => Loaded::Bytes(bytes.clone()),
        None => Loaded::NotParsed(NotParsed::Missing),
    });
    let answered: Vec<&str> = read.files.iter().map(|f| f.path.as_str()).collect();
    assert_eq!(answered, paths);
    assert_eq!(read.work.parsed, 64);
    assert_eq!(names(&read)[0], "f63");
    assert_eq!(names(&read)[32], "missing");
}
