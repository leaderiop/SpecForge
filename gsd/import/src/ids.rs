//! The ids the import writes (ADR 0391 D4, 13-RESEARCH Part B "Id rules"), each checked before it is used in an op or
//! a file path. Every check is over ASCII bytes: a space, a `/`, `..`, a control character or a non-ASCII digit fails.

/// The grammar's `_name`, which the host's create takes as an id: runs of ASCII letters, digits and `_`, joined by
/// one `-` or `.` (`AUTH-01`, `02.1`, `v1.0`).
pub fn name(id: &str) -> bool {
    let word = |b: u8| b.is_ascii_alphanumeric() || b == b'_';
    let bytes = id.as_bytes();
    !bytes.is_empty()
        && word(bytes[0])
        && word(bytes[bytes.len() - 1])
        && bytes
            .windows(2)
            .all(|w| word(w[0]) || (word(w[1]) && (w[0] == b'-' || w[0] == b'.')))
        && bytes.iter().all(|&b| word(b) || b == b'-' || b == b'.')
}

fn digits(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit())
}

/// `<CAT>-<NN>`: an upper-case category, then a number (`TASK-01`, the minted `OOS-01`).
pub fn requirement(id: &str) -> bool {
    id.split_once('-').is_some_and(|(cat, n)| {
        cat.as_bytes().first().is_some_and(u8::is_ascii_uppercase)
            && cat
                .bytes()
                .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit())
            && digits(n)
    })
}

/// `D-<NN>`, two digits at least.
pub fn decision(id: &str) -> bool {
    id.strip_prefix("D-")
        .is_some_and(|n| n.len() >= 2 && digits(n))
}

/// `v<version>`: `v1`, `v1.0`, `v2.3.1`.
pub fn milestone(id: &str) -> bool {
    id.strip_prefix('v')
        .is_some_and(|v| v.split('.').all(digits))
}

/// A phase: its integer part on two digits at least (no leading zero past two), a decimal kept (`01`, `02.1`, `100`).
pub fn phase(id: &str) -> bool {
    let (int, dec) = match id.split_once('.') {
        Some((int, dec)) => (int, Some(dec)),
        None => (id, None),
    };
    digits(int)
        && (int.len() == 2 || int.len() > 2 && !int.starts_with('0'))
        && dec.is_none_or(digits)
}

/// A phase's plan `<NN>-<MM>` (`01-02`, `02.1-01`).
pub fn plan(id: &str) -> bool {
    id.rsplit_once('-')
        .is_some_and(|(p, m)| phase(p) && m.len() >= 2 && digits(m))
}

/// A quick plan's id: its date `YYMMDD`, `-`, then lower-case letters and digits (`261105-tcl`).
pub fn quick(id: &str) -> bool {
    id.split_once('-').is_some_and(|(day, tail)| {
        day.len() == 6
            && digits(day)
            && !tail.is_empty()
            && tail
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
    })
}

/// A minted id: `prefix` then a number from 1, with no leading zero (`T1`, `TR12`, `KL3`).
pub fn minted(prefix: &str, id: &str) -> bool {
    id.strip_prefix(prefix)
        .is_some_and(|n| digits(n) && !n.starts_with('0'))
}

/// Whether `id` is an id the import may write for `kind`: the host's `_name` and the family of the kind.
pub fn valid(kind: &str, id: &str) -> bool {
    let family = match kind {
        "requirement" => requirement(id),
        "decision" => decision(id),
        "milestone" => milestone(id),
        "phase" => phase(id),
        "plan" => plan(id) || quick(id),
        "task" => minted("T", id),
        "truth" => minted("TR", id),
        "artifact" => minted("AR", id),
        "key_link" => minted("KL", id),
        "uat_test" => minted("U", id),
        "uat_gap" => minted("G", id),
        "deferred_idea" => minted("DI", id),
        "source" => minted("SRC", id),
        "finding" => minted("F", id),
        "stack_choice" => minted("SC", id),
        "pattern" => minted("P", id),
        "pitfall" => minted("PF", id),
        "dont_hand_roll" => minted("DH", id),
        "open_question" => minted("OQ", id),
        "trust_boundary" => minted("TB", id),
        "audit_run" => minted("AU", id),
        "accepted_risk" => minted("AR", id),
        // File stems (`01-01-SUMMARY`), coverage ids (`D1`), threat ids (`T-01-SC`): the grammar alone.
        _ => true,
    };
    name(id) && family
}

/// Whether `file` is a path the import may write: `spec/gsd/`, then names joined by `/`, then `.spec`.
pub fn file(file: &str) -> bool {
    file.strip_prefix("spec/gsd/")
        .and_then(|rest| rest.strip_suffix(".spec"))
        .is_some_and(|rest| rest.split('/').all(name))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_name_is_the_grammars() {
        for ok in ["TASK-01", "02.1", "v1.0", "01-01-SUMMARY", "T_1"] {
            assert!(name(ok), "{ok}");
        }
        for bad in [
            "",
            "TASK 01",
            "A/B-01",
            "..",
            "a..b",
            "-a",
            "a-",
            "a.",
            "T\u{7}-01",
            "TASK-\u{661}",
            "é",
            "a--b",
        ] {
            assert!(!name(bad), "{bad:?}");
        }
    }

    #[test]
    fn each_family_has_its_shape() {
        assert!(valid("requirement", "TASK-01") && valid("requirement", "OOS-01"));
        assert!(!valid("requirement", "task-01") && !valid("requirement", "TASK-"));
        assert!(valid("decision", "D-01") && !valid("decision", "D-1"));
        assert!(valid("milestone", "v1.0") && !valid("milestone", "v1..0"));
        assert!(valid("phase", "01") && valid("phase", "02.1") && valid("phase", "100"));
        assert!(!valid("phase", "1") && !valid("phase", "001"));
        assert!(valid("plan", "01-02") && valid("plan", "02.1-01") && valid("plan", "261105-tcl"));
        assert!(!valid("plan", "01-2") && !valid("plan", "1-02"));
        assert!(valid("task", "T1") && !valid("task", "T01") && !valid("task", "TR1"));
        assert!(valid("truth", "TR1") && valid("key_link", "KL3"));
        assert!(valid("summary", "01-01-SUMMARY") && !valid("summary", "01 SUMMARY"));
    }

    #[test]
    fn a_file_is_built_from_names_under_spec_gsd() {
        assert!(file("spec/gsd/requirements.spec"));
        assert!(file("spec/gsd/phases/01.spec"));
        for bad in [
            "spec/requirements.spec",
            "spec/gsd/../x.spec",
            "spec/gsd/a b.spec",
            "spec/gsd/req.txt",
            "spec/gsd//x.spec",
        ] {
            assert!(!file(bad), "{bad}");
        }
    }
}
