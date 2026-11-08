//! A file past its parse deadline: counted in tree-sitter's progress callbacks, never a clock.

use specforge_protocol_types::{NotParsed, SyntaxOutcome};
use specforge_source::{COUNTED_DEADLINE, Deadline, Language, Loaded, SourceReader, SyntaxCache};

const QUERY: &str = "(function_item name: (identifier) @name) @definition.function";

/// About 200 KB of functions.
fn large() -> Vec<u8> {
    (0..6000)
        .map(|n| format!("fn f{n}(a: u8) -> u8 {{ a + {n} }}\n"))
        .collect::<String>()
        .into_bytes()
}

fn read(reader: &SourceReader, files: &[(&str, Vec<u8>)]) -> specforge_source::Read {
    let query = reader.query(Language::Rust, QUERY).unwrap();
    let paths: Vec<String> = files.iter().map(|(path, _)| path.to_string()).collect();
    reader.read(&query, &paths, &|path| {
        let (_, bytes) = files.iter().find(|(p, _)| *p == path).unwrap();
        Loaded::Bytes(bytes.clone())
    })
}

#[test]
fn a_file_past_its_deadline_is_not_parsed_and_not_cached() {
    let base = tempfile::tempdir().unwrap();
    let reader =
        SourceReader::new(SyntaxCache::at(base.path())).with_deadline(Deadline::Progress(0));
    let files = [("large.rs", large())];
    assert!(files[0].1.len() > 200_000);
    let stopped = read(&reader, &files);
    assert_eq!(
        stopped.files[0].outcome,
        SyntaxOutcome::NotParsed {
            reason: NotParsed::Deadline
        }
    );
    assert_eq!((stopped.work.parsed, stopped.work.cached), (0, 0));
    // Nothing was kept: a reader with room for it parses it now.
    let open = read(
        &SourceReader::new(SyntaxCache::at(base.path())).with_deadline(COUNTED_DEADLINE),
        &files,
    );
    assert_eq!((open.work.parsed, open.work.cached), (1, 0));
    let SyntaxOutcome::Parsed { matches, .. } = &open.files[0].outcome else {
        panic!("{:?}", open.files[0]);
    };
    assert_eq!(matches.len(), 6000);
}

#[test]
fn the_deadline_is_per_file() {
    let reader =
        SourceReader::new(SyntaxCache::detached()).with_deadline(Deadline::Progress(PER_FILE));
    let read = read(
        &reader,
        &[
            ("large.rs", large()),
            ("small.rs", b"fn small() {}\n".to_vec()),
        ],
    );
    assert!(matches!(
        read.files[0].outcome,
        SyntaxOutcome::NotParsed {
            reason: NotParsed::Deadline
        }
    ));
    assert!(
        matches!(&read.files[1].outcome, SyntaxOutcome::Parsed { matches, .. } if matches.len() == 1),
        "{:?}",
        read.files[1]
    );
    assert_eq!(read.work.parsed, 1);
}

/// Progress callbacks a one-line file never reaches and the large one passes.
const PER_FILE: u32 = 50;
