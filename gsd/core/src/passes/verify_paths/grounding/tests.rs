use super::*;

fn at(command: &str) -> Target {
    match resolve(command) {
        Verdict::At(target) => target,
        other => panic!("{command}: {other:?}"),
    }
}

#[test]
fn a_cd_chain_folds_and_an_absolute_segment_starts_over() {
    assert_eq!(at("cd web && cd ./app && ls").dir, "web/app");
    assert_eq!(resolve("cd web && cd /abs && ls"), Verdict::Outside);
}

#[test]
fn the_prefix_flag_names_the_directory_with_or_without_quotes() {
    assert_eq!(at("npm --prefix ./web run lint").dir, "web");
    assert_eq!(at("npm --prefix=\"my dir\" test").dir, "my dir");
}

#[test]
fn the_tool_decides_the_manifest_and_the_script() {
    let target = at("cd web && npm --prefix x run lint:fix");
    assert_eq!(target.need, Need::Package(Some("lint:fix".into())));
    assert_eq!(target.paths(), ["web", "web/package.json"]);
    assert_eq!(
        at("cd web; make test").manifest().as_deref(),
        Some("web/Makefile")
    );
    assert_eq!(at("cd web && ls").need, Need::Nothing);
}

#[test]
fn an_escaped_ampersand_chains_like_a_real_one() {
    assert_eq!(at("cd src &amp;&amp; ls").dir, "src");
}

#[test]
fn what_cannot_be_checked_is_named() {
    assert_eq!(resolve("cd $WEB && ls"), Verdict::Dynamic);
    assert_eq!(resolve("cd ~/web"), Verdict::Dynamic);
    assert_eq!(resolve("cd ../.."), Verdict::Outside);
    assert_eq!(resolve("MISSING - Wave 0 creates it"), Verdict::Skip);
    assert_eq!(resolve("MISSING=1 npm test"), Verdict::Skip);
    assert_eq!(resolve("npm test"), Verdict::Skip);
}
