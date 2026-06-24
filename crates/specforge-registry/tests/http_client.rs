use specforge_registry::client::http_client::parse_specifier;

#[test]
fn parse_specifier_scoped_with_version() {
    let (name, version) = parse_specifier("@specforge/product@1.2.3");
    assert_eq!(name, "@specforge/product");
    assert_eq!(version, "1.2.3");
}

#[test]
fn parse_specifier_scoped_without_version() {
    let (name, version) = parse_specifier("@specforge/product");
    assert_eq!(name, "@specforge/product");
    assert_eq!(version, "latest");
}

#[test]
fn parse_specifier_scoped_with_range() {
    let (name, version) = parse_specifier("@myorg/my-ext@^2.0");
    assert_eq!(name, "@myorg/my-ext");
    assert_eq!(version, "^2.0");
}

#[test]
fn parse_specifier_unscoped_with_version() {
    let (name, version) = parse_specifier("simple-ext@0.5.0");
    assert_eq!(name, "simple-ext");
    assert_eq!(version, "0.5.0");
}

#[test]
fn parse_specifier_unscoped_without_version() {
    let (name, version) = parse_specifier("simple-ext");
    assert_eq!(name, "simple-ext");
    assert_eq!(version, "latest");
}
