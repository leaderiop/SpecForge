use super::*;

/// A span of line 3 from column `start` to `end`.
fn span(start: usize, end: usize) -> SourceSpan {
    SourceSpan {
        file: Sym::new("p.spec"),
        start_line: 3,
        start_col: start,
        end_line: 3,
        end_col: end,
    }
}

/// Each name `text` holds, and where it is placed, written at `at` with `lexemes`.
fn placed(
    text: &str,
    at: &SourceSpan,
    lexemes: &[SourceSpan],
) -> Vec<(String, Option<SourceSpan>)> {
    names(TypeExpression {
        text,
        owner: "m",
        typed: true,
        at: Some(at),
        lexemes,
    })
    .map(|(name, at, _)| (name.to_string(), at))
    .collect()
}

#[test]
fn a_spelling_cut_as_written_places_each_name_at_its_lexeme() {
    // `Map<K, V>` written `Map<K,\n V>`: five lexemes, cut alike.
    let lexemes = [
        span(1, 4),
        span(4, 5),
        span(5, 6),
        span(6, 7),
        span(9, 10),
        span(10, 11),
    ];
    let at = span(1, 11);
    assert_eq!(
        placed("Map<K, V>", &at, &lexemes),
        [
            ("Map".to_string(), Some(span(1, 4))),
            ("K".to_string(), Some(span(5, 6))),
            ("V".to_string(), Some(span(9, 10))),
        ]
    );
}

#[test]
fn a_spelling_cut_otherwise_places_each_name_on_the_whole_type() {
    // `Map<K V>` (an error-recovered parse) spelled `Map<KV>`: five lexemes written, four cut from the spelling.
    let lexemes = [span(1, 4), span(4, 5), span(5, 6), span(7, 8), span(8, 9)];
    let at = span(1, 9);
    assert_eq!(
        placed("Map<KV>", &at, &lexemes),
        [
            ("Map".to_string(), Some(at.clone())),
            ("KV".to_string(), Some(at.clone())),
        ]
    );
}
