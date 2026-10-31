//! The port rule (D-r40-5, D-r42-4), shared by the agent bodies and the workflow assets: an asset is upstream's text
//! line for line, and a line may differ only where upstream's names a tool this port replaced.
#![allow(
    dead_code,
    reason = "each test crate that includes this module uses some of it"
)]

use regex::Regex;

/// Checks one asset against the body lines of its upstream file: the header line, then every line. `header` is the
/// upstream path the attribution names; `rewritten` says which body lines (0-based) differ although upstream's names
/// no ported tool. Returns how many lines differ.
pub fn check_lines(
    label: &str,
    header: &str,
    asset: &str,
    body: &[String],
    rx: &Regex,
    rewritten: &dyn Fn(usize) -> bool,
) -> usize {
    let lines: Vec<&str> = asset.split('\n').collect();
    assert!(
        lines[0].starts_with(&format!(
            "<!-- Ported from gsd-core v{} {header}, MIT",
            gsd_kit::tracked_version!()
        )),
        "{label}: header {}",
        lines[0]
    );
    assert_eq!(lines.len() - 1, body.len(), "{label}: line count");
    let mut changed = 0;
    for (n, (mine, theirs)) in lines[1..].iter().zip(body).enumerate() {
        if rx.is_match(theirs) {
            assert_ne!(
                mine, theirs,
                "{label}:{n}: a ported line is left as upstream's"
            );
            assert!(
                !rx.is_match(mine),
                "{label}:{n}: still names upstream's tools: {mine}"
            );
            changed += 1;
        } else if rewritten(n) {
            assert_ne!(
                mine, theirs,
                "{label}:{n}: listed as rewritten, but is upstream's"
            );
            changed += 1;
        } else {
            assert_eq!(mine, theirs, "{label}:{n}: differs outside a ported line");
        }
    }
    changed
}
