//! The CVSS v3.x base score of a vector, as the FIRST CVSS v3.1 specification computes it (section 7 and Appendix
//! A), and the severity it rates (ADR 0388). The one place a float is computed: from the Roundup on, a score is an
//! integer number of tenths, so no float boundary can flip a rating.

use crate::vulnerability::Severity;

/// The base metrics, each with its values and their weights. `PR`'s weight is the one of scope unchanged; scope
/// changed raises `L` and `H` ([`PR_CHANGED`]). `S` has no weight: it picks the formula.
const BASE: [(&str, &[(&str, f64)]); 8] = [
    ("AV", &[("N", 0.85), ("A", 0.62), ("L", 0.55), ("P", 0.2)]),
    ("AC", &[("L", 0.77), ("H", 0.44)]),
    ("PR", &[("N", 0.85), ("L", 0.62), ("H", 0.27)]),
    ("UI", &[("N", 0.85), ("R", 0.62)]),
    ("S", &[("U", 0.0), ("C", 0.0)]),
    ("C", &[("H", 0.56), ("L", 0.22), ("N", 0.0)]),
    ("I", &[("H", 0.56), ("L", 0.22), ("N", 0.0)]),
    ("A", &[("H", 0.56), ("L", 0.22), ("N", 0.0)]),
];

/// `PR`'s weights when the scope is changed.
const PR_CHANGED: [(&str, f64); 3] = [("N", 0.85), ("L", 0.68), ("H", 0.5)];

/// The temporal and environmental metrics of the v3.1 specification and their values: allowed at most once each,
/// and ignored by the base score.
const IGNORED: [(&str, &[&str]); 14] = [
    ("E", &["X", "H", "F", "P", "U"]),
    ("RL", &["X", "U", "W", "T", "O"]),
    ("RC", &["X", "C", "R", "U"]),
    ("CR", &["X", "H", "M", "L"]),
    ("IR", &["X", "H", "M", "L"]),
    ("AR", &["X", "H", "M", "L"]),
    ("MAV", &["X", "N", "A", "L", "P"]),
    ("MAC", &["X", "L", "H"]),
    ("MPR", &["X", "N", "L", "H"]),
    ("MUI", &["X", "N", "R"]),
    ("MS", &["X", "U", "C"]),
    ("MC", &["X", "H", "L", "N"]),
    ("MI", &["X", "H", "L", "N"]),
    ("MA", &["X", "H", "L", "N"]),
];

/// The base score of the CVSS v3.0 or v3.1 `vector`, in tenths (`0..=100`).
///
/// The vector is `CVSS:3.0/` or `CVSS:3.1/` then `KEY:VALUE` pairs split on `/`, in any order: each base metric
/// exactly once with a value of its table, each temporal or environmental metric at most once with a value of its
/// table. `None` for anything else: another prefix (a v2 or v4 vector), a missing or repeated metric, an unknown
/// metric or value, an empty pair.
pub fn base_score(vector: &str) -> Option<u32> {
    let pairs = vector
        .strip_prefix("CVSS:3.1/")
        .or_else(|| vector.strip_prefix("CVSS:3.0/"))?;
    let mut base: [Option<&str>; 8] = [None; 8];
    let mut ignored = [false; 14];
    for pair in pairs.split('/') {
        let (key, value) = pair.split_once(':')?;
        if let Some(at) = BASE.iter().position(|(name, _)| *name == key) {
            let known = BASE[at].1.iter().any(|(name, _)| *name == value);
            if !known || base[at].replace(value).is_some() {
                return None;
            }
        } else {
            let at = IGNORED.iter().position(|(name, _)| *name == key)?;
            if !IGNORED[at].1.contains(&value) || std::mem::replace(&mut ignored[at], true) {
                return None;
            }
        }
    }
    let [av, ac, pr, ui, s, c, i, a] = base;
    let changed = s? == "C";
    let weight = |at: usize, value: Option<&str>| -> Option<f64> {
        let table = if at == 2 && changed {
            &PR_CHANGED[..]
        } else {
            BASE[at].1
        };
        let value = value?;
        table
            .iter()
            .find(|(name, _)| *name == value)
            .map(|(_, w)| *w)
    };
    let (av, ac, pr, ui) = (
        weight(0, av)?,
        weight(1, ac)?,
        weight(2, pr)?,
        weight(3, ui)?,
    );
    let (c, i, a) = (weight(5, c)?, weight(6, i)?, weight(7, a)?);
    let iss = 1.0 - (1.0 - c) * (1.0 - i) * (1.0 - a);
    let impact = if changed {
        7.52 * (iss - 0.029) - 3.25 * (iss - 0.02).powi(15)
    } else {
        6.42 * iss
    };
    if impact <= 0.0 {
        return Some(0);
    }
    let exploitability = 8.22 * av * ac * pr * ui;
    let sum = if changed {
        1.08 * (impact + exploitability)
    } else {
        impact + exploitability
    };
    Some(roundup(sum.min(10.0)))
}

/// The v3.1 Appendix A Roundup of `x`, in tenths: the smallest number of tenths not below `x`, computed on the
/// integer `round(x * 100000)` so a float a hair above a tenth does not round up to the next.
fn roundup(x: f64) -> u32 {
    let i = (x * 100_000.0).round() as u32;
    if i.is_multiple_of(10_000) {
        i / 10_000
    } else {
        i / 10_000 + 1
    }
}

/// The severity a base score of `tenths` rates: 9.0 and above critical, 7.0 high, 4.0 medium, below low. 0.0, which
/// the specification calls None, reads low: the vocabulary has no `none`.
pub fn rating(tenths: u32) -> Severity {
    match tenths {
        90.. => Severity::Critical,
        70..=89 => Severity::High,
        40..=69 => Severity::Medium,
        _ => Severity::Low,
    }
}
