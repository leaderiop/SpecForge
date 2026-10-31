//! Ported from gsd-core v1.19.0 hooks/lib/injection-patterns.js and hooks/gsd-read-injection-scanner.js, MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD
//!
//! The patterns of both scanners, as the text of upstream's regular expression literals (JavaScript's `source`), all
//! of the `i` flag. `injection_patterns_match_upstream` pins each against the vendored files.

/// The standard signatures both scanners share (`INJECTION_PATTERNS`).
pub const INJECTION_PATTERNS: [&str; 10] = [
    r"(?:ignore|disregard|forget|discard|override)\s+(?=(?:all|of|the|your|my|system|previous|prior|above|earlier)\s)(?:all\s+)?(?:of\s+)?(?:the\s+|your\s+|my\s+)?(?:(?:system|previous|prior|above|earlier)\s+)?(?:instructions|directives|prompts?|rules)|disregard\s+(?:all\s+)?previous|forget\s+instructions",
    r"you\s+are\s+now\s+(?:a|an|the)\s+",
    r"act\s+as\s+(?:a|an|the)\s+(?!plan|phase|wave)",
    r"pretend\s+(?:you(?:'re| are)\s+|to\s+be\s+)",
    r"from\s+now\s+on,?\s+you\s+(?:are|will|should|must)",
    r"(?:print|output|reveal|show|display|repeat)\s+(?:your\s+)?(?:system\s+)?(?:prompt|instructions)",
    r"<\/?(?:system|assistant|human)>",
    r"\[SYSTEM\]",
    r"\[INST\]",
    r"<<\s*SYS\s*>>",
];

/// Instructions written to survive context compression (the read scanner's own).
pub const SUMMARISATION_PATTERNS: [&str; 4] = [
    r"when\s+(?:summari[sz]ing|compressing|compacting),?\s+(?:retain|preserve|keep)\s+(?:this|these)",
    r"this\s+(?:instruction|directive|rule)\s+is\s+(?:permanent|persistent|immutable)",
    r"preserve\s+(?:these|this)\s+(?:rules?|instructions?|directives?)\s+(?:in|through|after|during)",
    r"(?:retain|keep)\s+(?:this|these)\s+(?:in|through|after)\s+(?:summar|compress|compact)",
];

/// A markdown link rule: the id, and the pattern of a line it flags.
pub const LINK_RULES: [(&str, &str); 4] = [
    ("MD-LINK-JS-SCHEME", r"\]\(\s*javascript:"),
    ("MD-LINK-DATA-SCHEME", r"\]\(\s*data:"),
    ("MD-LINK-USERINFO", r"\]\(\s*https?:\/\/[^/\s]+:[^/@\s]+@"),
    (
        "MD-LINK-TOKEN-IN-QUERY",
        r"[?&](token|access_token|id_token|refresh_token|api_key|apikey|secret|password|client_secret|code)=",
    ),
];

/// Where a `data:` link starts, for the rule that exempts the safe image and font types.
pub const DATA_LINK_START: &str = r"\]\(\s*(?=data:)";

/// The `data:` links `MD-LINK-DATA-SCHEME` lets through.
pub const DATA_URI_SAFE: &str =
    r"^data:(image\/(png|jpe?g|gif|webp|bmp|ico|avif|heic)|font\/(woff2?|otf|ttf))(;[^,]*)?,";

/// `describePattern`: a short single-line label of a pattern, for the advisory and the typed finding.
pub fn describe(source: &str) -> String {
    source
        .replace(r"\s+", "-")
        .chars()
        .filter(|c| !matches!(c, '(' | ')' | '\\'))
        .take(50)
        .collect()
}
