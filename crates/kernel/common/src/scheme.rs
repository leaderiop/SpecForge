//! The scheme an id is written in, `<scheme>:<rest>` (`js:a.test.js#adds`): a collector's fact's (ADR 0327).

/// The scheme of `id`: what it writes before its first `:`; `None` when it writes no `:`.
pub fn of(id: &str) -> Option<&str> {
    id.split_once(':').map(|(scheme, _)| scheme)
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_scheme_is_before_the_first_colon() {
        assert_eq!(super::of("js:a.test.js#adds"), Some("js"));
        assert_eq!(super::of("rust:a::b"), Some("rust"));
        assert_eq!(super::of("plain"), None);
    }
}
