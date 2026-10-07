use specforge_registry_client::http_client::parse_specifier;

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

/// What P3 makes of every input of plan 12's table (§2.2) today, including
/// the two rows that disagree with the ops reading (I2, I9); the function
/// goes with plan 12's T4.
// B:resolve_registry_source — verify unit "a fetch requests the name and version it was given, from the registry it was given"
#[test]
fn parse_specifier_reads_each_input_as_today() {
    let cases: &[(&str, &str, &str)] = &[
        ("@acme/tool", "@acme/tool", "latest"),              // I1
        ("@acme/tool@", "@acme/tool@", "latest"),            // I2: the whole string is the name
        ("@acme/tool@1.2.0", "@acme/tool", "1.2.0"),         // I3
        ("@acme/tool@^1.2", "@acme/tool", "^1.2"),           // I4
        ("@acme/tool@1.x", "@acme/tool", "1.x"),             // I5
        ("@acme/tool@1.2", "@acme/tool", "1.2"),             // I6
        ("@acme/tool@1.0.0/x", "@acme/tool", "1.0.0/x"),     // I7
        ("@acme/tool@1.0.0?x=1", "@acme/tool", "1.0.0?x=1"), // I8
        ("foo@/bar", "foo@/bar", "latest"),                  // I9: ops asked for `foo` at `/bar`
        ("tool@1.0.0", "tool", "1.0.0"),                     // I10
        ("tool", "tool", "latest"),                          // I11
        ("@acme/..", "@acme/..", "latest"),                  // I12
        ("@acme/T ool", "@acme/T ool", "latest"),            // I16
        ("Acme@1", "Acme", "1"),                             // I17
        ("@acme/tool@latest", "@acme/tool", "latest"),       // I18
        ("@acme/tool@>=1, <2", "@acme/tool", ">=1, <2"),     // I19
        ("@acme/tool@^bogus", "@acme/tool", "^bogus"),       // I20
        ("@acme/tool@2.0.0+build.1", "@acme/tool", "2.0.0+build.1"), // I21
        ("@scope", "@scope", "latest"),                      // I22
    ];
    for (input, name, version) in cases {
        assert_eq!(
            parse_specifier(input),
            (name.to_string(), version.to_string()),
            "{input:?}"
        );
    }
}
