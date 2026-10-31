//! Ported from gsd-core v1.19.0 hooks/gsd-secret-read-guard.js and hooks/lib/filename-classification.js, MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD
//!
//! Which names are secret files (`.env`, `.env.<suffix>`, `.secrets`; not a template), and which Grep globs can select
//! one. A glob is judged per brace alternative, against the names a secret file may take.

const NON_SECRET_ENV_SUFFIXES: [&str; 4] = ["example", "sample", "template", "dist"];
const MAX_GLOB_ALTERNATIVES: usize = 64;
const GLOB_PROBES: [&str; 10] = [
    ".env",
    ".secrets",
    ".env.local",
    ".env.development",
    ".env.production",
    ".env.staging",
    ".env.test",
    ".env.development.local",
    ".env.production.local",
    ".env.zzq",
];

/// The last path segment, trailing slashes dropped.
pub(super) fn last_segment(tok: &str) -> &str {
    let s = tok.trim_end_matches(['/', '\\']);
    s.rsplit(['/', '\\']).next().unwrap_or(s)
}

/// The text after the last dot, or the whole name when there is none.
fn final_extension(name: &str) -> &str {
    name.rsplit_once('.').map_or(name, |(_, ext)| ext)
}

/// Windows ignores trailing dots and spaces: `.env.` is `.env`.
fn normalize_windows_basename(name: &str) -> &str {
    name.trim_end_matches(['.', ' '])
}

fn is_secret_basename(name: &str) -> bool {
    let n = normalize_windows_basename(name);
    if n == ".env" || n == ".secrets" {
        return true;
    }
    n.strip_prefix(".env.").is_some_and(|suffix| {
        !suffix.is_empty()
            && !NON_SECRET_ENV_SUFFIXES.contains(&final_extension(suffix).to_lowercase().as_str())
    })
}

/// Whether `tok` names a secret file, alone or as `ref:path`.
pub(super) fn names_secret(tok: &str) -> bool {
    if tok.is_empty() {
        return false;
    }
    let lower = tok.to_lowercase();
    if is_secret_basename(last_segment(&lower)) {
        return true;
    }
    lower
        .rfind(':')
        .is_some_and(|colon| is_secret_basename(last_segment(&lower[colon + 1..])))
}

/// `glob` with its braces expanded; none when there are more than 64 alternatives.
fn expand_braces(glob: &str) -> Option<Vec<String>> {
    let chars: Vec<char> = glob.chars().collect();
    let Some(open) = chars.iter().position(|&c| c == '{') else {
        return Some(vec![glob.to_string()]);
    };
    let (mut depth, mut close, mut commas) = (0, None, Vec::new());
    for (i, &ch) in chars.iter().enumerate().skip(open) {
        match ch {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    close = Some(i);
                    break;
                }
            }
            ',' if depth == 1 => commas.push(i),
            _ => {}
        }
    }
    let Some(close) = close else {
        return Some(vec![glob.to_string()]);
    };
    let text = |a: usize, b: usize| chars[a..b].iter().collect::<String>();
    let (pre, post) = (text(0, open), text(close + 1, chars.len()));
    let mut bounds = vec![open];
    bounds.extend(&commas);
    bounds.push(close);
    let mut out = Vec::new();
    for w in bounds.windows(2) {
        for alt in expand_braces(&format!("{pre}{}{post}", text(w[0] + 1, w[1])))? {
            out.push(alt);
            if out.len() > MAX_GLOB_ALTERNATIVES {
                return None;
            }
        }
    }
    Some(out)
}

enum Piece {
    Star,
    Any,
    Class(bool, Vec<(char, char)>),
    Lit(char),
}

/// A glob alternative as pieces: `*` and `?` stay in one segment, `[a-z]` and `[!a]` are classes. None for a class
/// no regular expression accepts (`[z-a]`), which upstream counts as selecting a secret.
fn pieces(alt: &str) -> Option<Vec<Piece>> {
    let cs: Vec<char> = alt.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < cs.len() {
        match cs[i] {
            '*' => out.push(Piece::Star),
            '?' => out.push(Piece::Any),
            '[' => match cs[i + 1..].iter().position(|&c| c == ']') {
                None => out.push(Piece::Lit('[')),
                Some(len) => {
                    let body = &cs[i + 1..i + 1 + len];
                    let (negated, body) = match body.first() {
                        Some('!' | '^') => (true, &body[1..]),
                        _ => (false, body),
                    };
                    let mut ranges = Vec::new();
                    let mut k = 0;
                    while k < body.len() {
                        if k + 2 < body.len() && body[k + 1] == '-' {
                            if body[k] > body[k + 2] {
                                return None;
                            }
                            ranges.push((body[k], body[k + 2]));
                            k += 3;
                        } else {
                            ranges.push((body[k], body[k]));
                            k += 1;
                        }
                    }
                    out.push(Piece::Class(negated, ranges));
                    i += len + 1;
                }
            },
            c => out.push(Piece::Lit(c)),
        }
        i += 1;
    }
    Some(out)
}

fn matches(ps: &[Piece], s: &[char]) -> bool {
    match ps.split_first() {
        None => s.is_empty(),
        Some((Piece::Star, rest)) => (0..=s.len())
            .take_while(|&n| s[..n].iter().all(|&c| c != '/'))
            .any(|n| matches(rest, &s[n..])),
        Some((p, rest)) => s.split_first().is_some_and(|(&c, tail)| {
            c != '/'
                && match p {
                    Piece::Any => true,
                    Piece::Lit(l) => *l == c,
                    Piece::Class(neg, ranges) => {
                        ranges.iter().any(|&(a, b)| a <= c && c <= b) != *neg
                    }
                    Piece::Star => false,
                }
                && matches(rest, tail)
        }),
    }
}

fn alt_selects_secret(alt: &str) -> bool {
    if alt.is_empty() || alt.chars().all(|c| c == '*' || c == '?') {
        return false;
    }
    let wild = alt.find(['*', '?', '[']);
    let lit = wild.map_or(alt, |w| &alt[..w]);
    if wild.is_none() {
        if is_secret_basename(lit) {
            return true;
        }
    } else if lit.starts_with(".env.") {
        return true;
    }
    if !lit.is_empty() && (".env.".starts_with(lit) || ".secrets".starts_with(lit)) {
        return true;
    }
    let Some(ps) = pieces(alt) else {
        return true;
    };
    GLOB_PROBES
        .iter()
        .any(|probe| matches(&ps, &probe.chars().collect::<Vec<_>>()))
}

/// `Some("secret-read")` when a Grep glob can select a secret file, `Some("glob-too-complex")` when it cannot be
/// judged, none otherwise.
pub(super) fn classify_glob(glob: &str) -> Option<&'static str> {
    match expand_braces(&last_segment(glob).to_lowercase()) {
        None => Some("glob-too-complex"),
        Some(alts) => alts
            .iter()
            .any(|a| alt_selects_secret(a))
            .then_some("secret-read"),
    }
}
