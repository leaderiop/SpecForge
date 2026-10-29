//! What `specforge install claude-code` writes for the 15 core agents: a golden per file (its frontmatter, its judged
//! section and the digest of its body), and the body is the asset.

use sha2::{Digest, Sha256};
use std::path::Path;

use super::agents::{claude_agent, installed, split};
use super::harness::project;
use gsd_core::agents::roster::AGENTS;

const JUDGED: &str = "\n## Judged rules\n";

/// The body of an installed file: after its frontmatter, before the judged section and the attribution notice.
fn body(file: &str) -> String {
    let (_, rest) = split(file);
    let end = rest
        .find(JUDGED)
        .or_else(|| rest.find("\n<!-- Ported from gsd-core agents/"));
    rest[..end.unwrap_or(rest.len())].trim().to_string()
}

/// The golden of one installed file.
fn golden_of(file: &str) -> String {
    let (front, _) = split(file);
    let mut out = String::from("---\n");
    for (k, v) in front {
        out += &format!("{k}: {v}\n");
    }
    out += "---\n";
    match file.split(JUDGED).nth(1) {
        Some(section) => {
            let section = section.split("\n<!-- ").next().unwrap_or(section).trim();
            out += &format!("\n## Judged rules\n\n{section}\n");
        }
        None => out += "\n(no judged rules)\n",
    }
    let body = body(file);
    let digest = Sha256::digest(body.as_bytes());
    out += &format!("\nbody sha256: {digest:x}\nbody bytes: {}\n", body.len());
    out
}

fn check_golden(id: &str, actual: &str) {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/golden/install/claude-code/agents")
        .join(format!("{id}.md"));
    if std::env::var_os("GSD_BLESS").is_some() {
        std::fs::create_dir_all(path.parent().expect("a folder")).expect("the folder");
        std::fs::write(&path, actual).expect("write the golden");
        return;
    }
    let expected = std::fs::read_to_string(&path).unwrap_or_else(|_| {
        panic!(
            "{} is missing: GSD_BLESS=1 cargo test writes it",
            path.display()
        )
    });
    assert!(
        expected == actual,
        "{id} differs from its golden: review the change, then GSD_BLESS=1 cargo test\n--- expected\n{expected}\n--- actual\n{actual}"
    );
}

#[test]
fn install_agents_golden() {
    let p = project("empty");
    installed(&p, "claude-code");
    for agent in AGENTS {
        check_golden(agent.id, &golden_of(&claude_agent(&p, agent.id)));
    }
}

#[test]
fn a_second_install_changes_nothing() {
    let p = project("empty");
    installed(&p, "claude-code");
    let first: Vec<String> = AGENTS.iter().map(|a| claude_agent(&p, a.id)).collect();
    installed(&p, "claude-code");
    let second: Vec<String> = AGENTS.iter().map(|a| claude_agent(&p, a.id)).collect();
    assert_eq!(first, second);
}

#[test]
fn the_body_is_the_asset() {
    let p = project("empty");
    installed(&p, "claude-code");
    for agent in AGENTS {
        // The asset's header line is a comment about the port; the host's own notice follows the body.
        let asset = agent.full.trim();
        let installed = body(&claude_agent(&p, agent.id));
        assert_eq!(installed, asset, "{}", agent.id);
    }
}
