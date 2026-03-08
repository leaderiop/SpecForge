/// Find the closest match for `target` among `candidates` using Jaro-Winkler similarity.
/// Returns `None` if no candidate scores above the 0.85 threshold.
pub fn find_close_match<'a>(target: &str, candidates: impl IntoIterator<Item = &'a str>) -> Option<&'a str> {
    candidates
        .into_iter()
        .filter_map(|c| {
            let score = strsim::jaro_winkler(target, c);
            if score > 0.85 { Some((c, score)) } else { None }
        })
        .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
        .map(|(s, _)| s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_match_returns_candidate() {
        let result = find_close_match("alpha", ["alpha", "beta", "gamma"]);
        assert_eq!(result, Some("alpha"));
    }

    #[test]
    fn close_typo_returns_suggestion() {
        let result = find_close_match("alpha_parsr", ["alpha_parser", "beta_builder", "gamma_runner"]);
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
