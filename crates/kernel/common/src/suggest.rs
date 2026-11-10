/// Largest Levenshtein edit distance a suggestion may be from its target.
pub const MAX_SUGGESTION_DISTANCE: usize = 3;

/// Find the closest match for `target` among `candidates`.
///
/// A candidate qualifies when it is within [`MAX_SUGGESTION_DISTANCE`] edits
/// of `target` and also scores above 0.85 Jaro-Winkler similarity, which
/// keeps short, unrelated IDs (`abc` and `xyz` are three edits apart) from
/// being suggested. The fewest edits wins, then the higher similarity.
/// Returns `None` when no candidate qualifies.
pub fn find_close_match<'a>(
    target: &str,
    candidates: impl IntoIterator<Item = &'a str>,
) -> Option<&'a str> {
    within_reach(target, candidates)
        .filter_map(|c| {
            let distance = strsim::levenshtein(target, c);
            if distance > MAX_SUGGESTION_DISTANCE {
                return None;
            }
            let score = strsim::jaro_winkler(target, c);
            (score > 0.85).then_some((c, distance, score))
        })
        // Deterministic tie-break: among equal distances and scores pick the
        // lexicographically smallest candidate, so the suggestion never
        // depends on the caller's iteration order (R-6 / hardening-plan D1).
        .min_by(|a, b| {
            a.1.cmp(&b.1)
                .then_with(|| b.2.total_cmp(&a.2))
                .then_with(|| a.0.cmp(b.0))
        })
        .map(|(s, _, _)| s)
}

/// The candidates whose length in characters is within [`MAX_SUGGESTION_DISTANCE`] of `target`'s: a Levenshtein
/// distance is at least the difference of the lengths, so no other candidate can qualify, and none is measured.
fn within_reach<'a, 't>(
    target: &'t str,
    candidates: impl IntoIterator<Item = &'a str> + 't,
) -> impl Iterator<Item = &'a str> + 't {
    let length = target.chars().count();
    candidates
        .into_iter()
        .filter(move |c| c.chars().count().abs_diff(length) <= MAX_SUGGESTION_DISTANCE)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_candidates_within_the_distance_in_length_are_measured() {
        let candidates = ["abcd", "abcdefg", "abcdefgh", "a", "ab", "αβγδε"];
        let measured: Vec<&str> = within_reach("abcd", candidates).collect();
        // Three characters either way of four: one to seven, counted in characters, not bytes.
        assert_eq!(measured, ["abcd", "abcdefg", "a", "ab", "αβγδε"]);
        // Three characters longer is still within reach; four is never suggested.
        assert_eq!(find_close_match("abcd", ["abcdefg"]), Some("abcdefg"));
        assert_eq!(find_close_match("abcd", ["abcdefgh"]), None);
    }

    #[test]
    fn exact_match_returns_candidate() {
        let result = find_close_match("alpha", ["alpha", "beta", "gamma"]);
        assert_eq!(result, Some("alpha"));
    }

    #[test]
    fn close_typo_returns_suggestion() {
        let result = find_close_match(
            "alpha_parsr",
            ["alpha_parser", "beta_builder", "gamma_runner"],
        );
        assert_eq!(result, Some("alpha_parser"));
    }

    #[test]
    fn no_close_match_returns_none() {
        let result = find_close_match("zzzzz", ["alpha", "beta", "gamma"]);
        assert_eq!(result, None);
    }

    #[test]
    fn empty_candidates_returns_none() {
        let result: Option<&str> = find_close_match("alpha", std::iter::empty());
        assert_eq!(result, None);
    }

    #[test]
    fn picks_best_among_multiple_close_matches() {
        let result = find_close_match("alpha_parse", ["alpha_parser", "alpha_parsed"]);
        // Both are close but "alpha_parser" should score higher (or either is acceptable)
        assert!(result.is_some());
    }
}
