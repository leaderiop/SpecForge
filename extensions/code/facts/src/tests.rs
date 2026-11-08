use std::collections::BTreeSet;

use super::*;
use specforge_test::prelude::*;

mod mint;

fn strings(items: &[&str]) -> Vec<String> {
    items.iter().map(|s| s.to_string()).collect()
}

fn owner(self_ty: &str, trait_ty: Option<&str>) -> Owner {
    Owner {
        self_ty: self_ty.to_string(),
        trait_ty: trait_ty.map(str::to_string),
    }
}

#[test]
fn a_js_test_id_is_unchanged() {
    let id = TestId::js(
        "taskboard",
        "test/tasks.test.js",
        &strings(&["a task is added with the next id"]),
    );
    assert_eq!(
        id.to_string(),
        "js:taskboard/test/tasks.test.js#a task is added with the next id"
    );
    let nested = TestId::js("@acme/core", "test/x.test.js", &strings(&["board", "adds"]));
    assert_eq!(
        nested.to_string(),
        "js:@acme/core/test/x.test.js#board > adds"
    );
    assert_eq!(
        nested.run(),
        "run:js:@acme/core/test/x.test.js#board > adds"
    );
    assert_eq!(nested.container(), "js:@acme/core/test/x.test.js#");
    assert_eq!(
        (nested.package(), nested.suite(), nested.name()),
        ("@acme/core", &strings(&["board"])[..], "adds")
    );
}

#[test]
fn a_rust_test_id_is_its_binary_and_path() {
    let id = TestId::rust("specforge-ops::golden", "reads::a_store");
    assert_eq!(id.to_string(), "rust:specforge-ops::golden#reads::a_store");
    assert_eq!(id.container(), "rust:specforge-ops::golden#");
    assert_eq!(
        (id.package(), id.binary(), id.suite(), id.name()),
        (
            "specforge-ops",
            Some("specforge-ops::golden"),
            &strings(&["reads"])[..],
            "a_store"
        )
    );
    assert_eq!(TestId::rust("toy", "adds").package(), "toy");
}

#[specforge_test(
    behavior = "cc_mint_code_ids",
    verify = "an item id never holds a hash sign, a test id always does"
)]
fn an_item_id_never_holds_a_hash() {
    let raw = ItemId::rust("toy", &strings(&["r#match"]), None, "r#type");
    assert_eq!(raw.to_string(), "rust:toy::match::type");
    let file = ItemId::js("app", "src/a#b:c%.ts", &strings(&["Board", "#x"]));
    assert_eq!(file.to_string(), "js:app/src/a%23b%3Ac%25.ts::Board.%23x");
    let dotted = ItemId::js("app", "a.js", &strings(&["a.b"]));
    assert_eq!(dotted.to_string(), "js:app/a.js::a%2Eb");
    for id in [&raw, &file, &dotted] {
        assert!(!id.to_string().contains('#'), "{id}");
    }
    let test = TestId::js("app", "a#b.ts", &strings(&["adds"]));
    assert_eq!(test.to_string(), "js:app/a%23b.ts#adds");
    assert!("rust:toy#adds".parse::<ItemId>().is_err());
    assert!("rust:toy::adds".parse::<TestId>().is_err());
}

#[test]
fn a_trait_impl_member_drops_paths_and_generic_arguments() {
    let io = ItemId::rust(
        "toy",
        &strings(&["errors"]),
        Some(&owner("crate::a::Wrapper<T>", Some("From<io::Error>"))),
        "from",
    );
    let fmt = ItemId::rust(
        "toy",
        &strings(&["errors"]),
        Some(&owner(
            "Wrapper < T >",
            Some("std::convert::From<fmt::Error>"),
        )),
        "from",
    );
    assert_eq!(io.to_string(), "rust:toy::errors::<Wrapper as From>::from");
    assert_eq!(io, fmt);
    assert_eq!(io.name(), "from");
    let inherent = ItemId::rust("toy", &[], Some(&owner("Board", None)), "add");
    assert_eq!(inherent.to_string(), "rust:toy::Board::add");
    for (self_ty, written) in [
        ("&'a  T", "rust:toy::<&'a T as Tr>::f"),
        ("[u8]", "rust:toy::<[u8] as Tr>::f"),
        ("(A, B)", "rust:toy::<(A, B) as Tr>::f"),
        ("dyn  Fn(u8)", "rust:toy::<dyn Fn(u8) as Tr>::f"),
        ("&crate::a::T", "rust:toy::<&crate::a::T as Tr>::f"),
    ] {
        let id = ItemId::rust("toy", &[], Some(&owner(self_ty, Some("Tr"))), "f");
        assert_eq!(id.to_string(), written, "{self_ty}");
        assert_eq!(written.parse::<ItemId>().unwrap(), id, "{self_ty}");
    }
}

#[test]
fn a_target_other_than_the_lib_is_qualified() {
    let foo = PackageId::cargo("specforge-cli");
    for (kind, target, segment) in [
        ("lib", "specforge_cli", "specforge-cli"),
        ("proc-macro", "specforge_cli", "specforge-cli"),
        ("bin", "specforge", "specforge-cli/bin/specforge"),
        ("test", "golden", "specforge-cli/test/golden"),
        ("example", "demo", "specforge-cli/example/demo"),
        ("bench", "speed", "specforge-cli/bench/speed"),
        (
            "custom-build",
            "build-script-build",
            "specforge-cli/custom-build",
        ),
    ] {
        assert_eq!(crate_segment(&foo, kind, target), segment, "{kind}");
    }
    let main = ItemId::rust(&crate_segment(&foo, "bin", "specforge"), &[], None, "main");
    assert_eq!(main.to_string(), "rust:specforge-cli/bin/specforge::main");
    assert_eq!(main.container(), "rust:specforge-cli/bin/specforge::");
}

#[test]
fn a_second_package_of_one_name_takes_its_dir() {
    let twin = PackageId::cargo_in("toy", "vendor/to:y");
    assert_eq!(twin.to_string(), "pkg:cargo/toy#vendor/to:y");
    assert_eq!(crate_segment(&twin, "lib", "toy"), "toy@vendor/to%3Ay");
    assert_eq!(crate_segment(&twin, "bin", "t"), "toy/bin/t@vendor/to%3Ay");
    let item = ItemId::rust(&crate_segment(&twin, "lib", "toy"), &[], None, "adds");
    assert_eq!(item.to_string(), "rust:toy@vendor/to%3Ay::adds");
    assert_eq!(item.to_string().parse::<ItemId>().unwrap(), item);
}

#[test]
fn an_items_test_id_is_what_nextest_names_it() {
    let package = PackageId::cargo("specforge-ops");
    let module = strings(&["read", "tests"]);
    for (kind, target, binary) in [
        ("lib", "specforge_ops", "specforge-ops"),
        ("test", "golden", "specforge-ops::golden"),
        ("bin", "x", "specforge-ops::bin/x"),
        ("example", "e", "specforge-ops::example/e"),
    ] {
        let item = ItemId::rust(
            &crate_segment(&package, kind, target),
            &module,
            None,
            "adds",
        );
        assert_eq!(
            item.test_id().map(|t| t.to_string()),
            Some(format!("rust:{binary}#read::tests::adds")),
            "{kind}"
        );
    }
    let build = ItemId::rust(
        &crate_segment(&package, "custom-build", "b"),
        &[],
        None,
        "main",
    );
    let method = ItemId::rust("toy", &[], Some(&owner("S", Some("Tr"))), "f");
    let js = ItemId::js("app", "a.js", &strings(&["f"]));
    for item in [build, method, js] {
        assert_eq!(item.test_id(), None, "{item}");
    }
}

#[test]
fn a_scoped_npm_purl_encodes_the_at() {
    assert_eq!(
        PackageId::npm("@acme/core").to_string(),
        "pkg:npm/%40acme/core"
    );
    assert_eq!(PackageId::npm("taskboard").to_string(), "pkg:npm/taskboard");
    assert_eq!(PackageId::cargo("toy").to_string(), "pkg:cargo/toy");
    assert_eq!(
        "pkg:npm/%40acme/core".parse::<PackageId>().unwrap(),
        PackageId::npm("@acme/core")
    );
    for bad in ["npm/a", "pkg:pypi/a", "pkg:cargo/", "pkg:cargo/a#"] {
        assert!(bad.parse::<PackageId>().is_err(), "{bad}");
    }
}

#[specforge_test(
    behavior = "cc_mint_code_ids",
    verify = "every id parses back to itself"
)]
fn every_id_parses_back_to_itself() {
    let tests = [
        TestId::js("@acme/core", "src/a#b.ts", &strings(&["s", "t"])),
        TestId::rust("toy::golden", "a::b"),
    ];
    for id in tests {
        assert_eq!(id.to_string().parse::<TestId>().unwrap(), id, "{id}");
    }
    let items = [
        ItemId::rust("toy/bin/t", &strings(&["a", "b"]), None, "f"),
        ItemId::rust("toy", &[], Some(&owner("Vec<T>", Some("Iterator"))), "next"),
        ItemId::rust("toy", &[], None, "C"),
        ItemId::js(
            "@acme/core",
            "src/a:b.ts",
            &strings(&["Board", "add.x", "#p"]),
        ),
    ];
    for id in items {
        assert_eq!(id.to_string().parse::<ItemId>().unwrap(), id, "{id}");
    }
    let packages = [
        PackageId::cargo("toy"),
        PackageId::cargo_in("toy", "a b/c"),
        PackageId::npm("@acme/core"),
    ];
    for id in packages {
        assert_eq!(id.to_string().parse::<PackageId>().unwrap(), id, "{id}");
    }
}

/// A SCIP descriptor: its name (backtick escapes undone) and its suffix.
#[derive(Debug, PartialEq)]
enum Descriptor {
    Namespace(String),
    Type(String),
    Term(String),
    Method(String),
    Macro(String),
    TypeParameter(String),
    Parameter(String),
    Meta(String),
}

/// The descriptors of a SCIP symbol's last part (the SCIP grammar: `name/`, `name#`, `name.`, `name(…).`, `name!`,
/// `[name]`, `(name)`, `name:`; a name outside `[\w$+-]` is backticked, a backtick doubled).
fn descriptors(mut text: &str) -> Vec<Descriptor> {
    fn name(text: &str) -> (String, &str) {
        if let Some(rest) = text.strip_prefix('`') {
            let mut name = String::new();
            let mut chars = rest.char_indices();
            while let Some((at, c)) = chars.next() {
                if c == '`' {
                    if rest[at + 1..].starts_with('`') {
                        name.push('`');
                        chars.next();
                    } else {
                        return (name, &rest[at + 1..]);
                    }
                } else {
                    name.push(c);
                }
            }
            (name, "")
        } else {
            let end = text
                .find(|c: char| !(c.is_alphanumeric() || "_$+-".contains(c)))
                .unwrap_or(text.len());
            (text[..end].to_string(), &text[end..])
        }
    }
    let mut found = Vec::new();
    while !text.is_empty() {
        if let Some(rest) = text.strip_prefix('[') {
            let (n, rest) = name(rest);
            found.push(Descriptor::TypeParameter(n));
            text = rest.strip_prefix(']').unwrap();
            continue;
        }
        if let Some(rest) = text.strip_prefix('(') {
            let (n, rest) = name(rest);
            found.push(Descriptor::Parameter(n));
            text = rest.strip_prefix(')').unwrap();
            continue;
        }
        let (n, rest) = name(text);
        let (descriptor, rest) = match rest.chars().next().unwrap() {
            '/' => (Descriptor::Namespace(n), &rest[1..]),
            '#' => (Descriptor::Type(n), &rest[1..]),
            '.' => (Descriptor::Term(n), &rest[1..]),
            '!' => (Descriptor::Macro(n), &rest[1..]),
            ':' => (Descriptor::Meta(n), &rest[1..]),
            '(' => {
                let close = rest.find(").").unwrap();
                (Descriptor::Method(n), &rest[close + 2..])
            }
            other => panic!("no descriptor suffix {other:?} in {text}"),
        };
        found.push(descriptor);
        text = rest;
    }
    found
}

/// The id of `path` in `scheme_container`, or of its nearest owner that `minted` holds: a descriptor no fact is minted
/// for maps to its owner's id.
fn nearest(
    scheme_container: &str,
    mut path: Vec<String>,
    join: &str,
    minted: &BTreeSet<&str>,
) -> Option<String> {
    while !path.is_empty() {
        let id = format!("{scheme_container}{}", path.join(join));
        if minted.contains(id.as_str()) {
            return Some(id);
        }
        path.pop();
    }
    None
}

/// The id a rust-analyzer symbol maps into (ADR 0347 D4): the target descriptor makes the crate segment, an `impl#`
/// with one or two type parameters the owner, every other name a segment, a parameter or type parameter ends it.
fn rust_analyzer(symbol: &str, minted: &BTreeSet<&str>) -> Option<String> {
    let parts: Vec<&str> = symbol.splitn(5, ' ').collect();
    let ["rust-analyzer", "cargo", package, _version, descriptors_text] = parts[..] else {
        return None;
    };
    let mut found = descriptors(descriptors_text).into_iter().peekable();
    let target = match found.next()? {
        Descriptor::Namespace(t) if t == "lib" || t == "proc-macro" => {
            ("lib".to_string(), String::new())
        }
        Descriptor::Namespace(t) if t == "custom-build" => {
            ("custom-build".to_string(), String::new())
        }
        Descriptor::Namespace(kind) => match found.next()? {
            Descriptor::Namespace(target) => (kind, target),
            _ => return None,
        },
        _ => return None,
    };
    let krate = crate_segment(&PackageId::cargo(package), &target.0, &target.1);
    let mut path = Vec::new();
    while let Some(descriptor) = found.next() {
        match descriptor {
            Descriptor::Type(n) if n == "impl" => {
                let mut types = Vec::new();
                while let Some(Descriptor::TypeParameter(t)) = found.peek() {
                    types.push(t.split('<').next().unwrap().to_string());
                    found.next();
                }
                path.push(match &types[..] {
                    [self_ty] => self_ty.clone(),
                    [self_ty, trait_ty] => format!("<{self_ty} as {trait_ty}>"),
                    _ => return None,
                });
            }
            Descriptor::Namespace(n)
            | Descriptor::Type(n)
            | Descriptor::Term(n)
            | Descriptor::Method(n)
            | Descriptor::Macro(n) => path.push(n),
            Descriptor::TypeParameter(_) | Descriptor::Parameter(_) | Descriptor::Meta(_) => break,
        }
    }
    nearest(&format!("rust:{krate}::"), path, "::", minted)
}

/// The id a scip-typescript symbol maps into (ADR 0347 D4): the directories and the first namespace ending in a
/// source extension make the file, the other names are joined by `.`, `<constructor>` is `constructor` and an
/// accessor its property; a parameter, type parameter or meta ends it.
fn scip_typescript(symbol: &str, minted: &BTreeSet<&str>) -> Option<String> {
    let parts: Vec<&str> = symbol.splitn(5, ' ').collect();
    let ["scip-typescript", "npm", package, _version, descriptors_text] = parts[..] else {
        return None;
    };
    let source = |name: &str| {
        [".ts", ".tsx", ".js", ".jsx", ".mts", ".cts", ".mjs", ".cjs"]
            .iter()
            .any(|ext| name.ends_with(ext))
    };
    let mut found = descriptors(descriptors_text).into_iter();
    let mut file = Vec::new();
    for descriptor in found.by_ref() {
        let Descriptor::Namespace(n) = descriptor else {
            return None;
        };
        let last = source(&n);
        file.push(n);
        if last {
            break;
        }
    }
    let mut names = Vec::new();
    for descriptor in found {
        match descriptor {
            Descriptor::Namespace(n)
            | Descriptor::Type(n)
            | Descriptor::Term(n)
            | Descriptor::Method(n)
            | Descriptor::Macro(n) => names.push(match n.as_str() {
                "<constructor>" => "constructor".to_string(),
                accessor => accessor
                    .strip_prefix("<get>")
                    .or_else(|| accessor.strip_prefix("<set>"))
                    .unwrap_or(accessor)
                    .to_string(),
            }),
            Descriptor::TypeParameter(_) | Descriptor::Parameter(_) | Descriptor::Meta(_) => break,
        }
    }
    let container = ItemId::js(package, &file.join("/"), &strings(&["x"])).container();
    let minted: BTreeSet<String> = minted
        .iter()
        .filter_map(|id| id.parse::<ItemId>().ok())
        .map(|id| id.to_string())
        .collect();
    let minted: BTreeSet<&str> = minted.iter().map(String::as_str).collect();
    nearest(&container, names, ".", &minted)
}

#[specforge_test(
    behavior = "cc_mint_code_ids",
    verify = "a SCIP symbol of rust-analyzer or scip-typescript maps into exactly one id"
)]
fn rust_analyzer_symbols_map_into_ids() {
    let minted = BTreeSet::from([
        "rust:foo::a::b::f",
        "rust:foo::S",
        "rust:foo/bin/main::S",
        "rust:foo/test/t::S",
        "rust:foo/example/e::S",
        "rust:foo/bench/b::S",
        "rust:foo/custom-build::S",
        "rust:foo::m::S::new",
        "rust:foo::m::<S as Tr>::f",
        "rust:foo::<S as From>::from",
        "rust:foo::m::Tr::f",
        "rust:foo::m::Tr::C",
        "rust:foo::m::Tr::Ty",
        "rust:foo::m",
        "rust:foo::C",
        "rust:foo::St",
        "rust:foo::E",
    ]);
    let cargo = "rust-analyzer cargo foo 0.1.0";
    for (symbol, id) in [
        ("lib/a/b/f().", Some("rust:foo::a::b::f")),
        ("proc-macro/S#", Some("rust:foo::S")),
        ("bin/main/S#", Some("rust:foo/bin/main::S")),
        ("test/t/S#", Some("rust:foo/test/t::S")),
        ("example/e/S#", Some("rust:foo/example/e::S")),
        ("bench/b/S#", Some("rust:foo/bench/b::S")),
        ("custom-build/S#", Some("rust:foo/custom-build::S")),
        ("lib/m/impl#[S]new().", Some("rust:foo::m::S::new")),
        ("lib/m/impl#[S][Tr]f().", Some("rust:foo::m::<S as Tr>::f")),
        (
            "lib/impl#[S][`From<Error>`]from().",
            Some("rust:foo::<S as From>::from"),
        ),
        ("lib/m/Tr#f().", Some("rust:foo::m::Tr::f")),
        ("lib/m/Tr#C.", Some("rust:foo::m::Tr::C")),
        ("lib/m/Tr#Ty#", Some("rust:foo::m::Tr::Ty")),
        ("lib/m/", Some("rust:foo::m")),
        ("lib/m!", Some("rust:foo::m")),
        ("lib/C.", Some("rust:foo::C")),
        ("lib/St#a.", Some("rust:foo::St")),
        ("lib/E#V.", Some("rust:foo::E")),
        ("lib/a/b/f().(x)", Some("rust:foo::a::b::f")),
        ("lib/St#[T]", Some("rust:foo::St")),
    ] {
        let mapped = rust_analyzer(&format!("{cargo} {symbol}"), &minted);
        assert_eq!(mapped.as_deref(), id, "{symbol}");
        let parsed: ItemId = mapped.unwrap().parse().unwrap();
        assert!(minted.contains(parsed.to_string().as_str()), "{symbol}");
    }
    assert_eq!(rust_analyzer("local 4", &minted), None);
}

#[specforge_test(
    behavior = "cc_mint_code_ids",
    verify = "a SCIP symbol of rust-analyzer or scip-typescript maps into exactly one id"
)]
fn scip_typescript_symbols_map_into_ids() {
    let board =
        |path: &[&str]| ItemId::js("@acme/core", "src/board.ts", &strings(path)).to_string();
    let minted_ids = [
        board(&["Board"]),
        board(&["Board", "add"]),
        board(&["Board", "constructor"]),
        board(&["Board", "size"]),
        board(&["addTask"]),
        board(&["NS", "inner"]),
        board(&["Shape"]),
        ItemId::js("@acme/core", "src/index.ts", &strings(&["VERSION"])).to_string(),
    ];
    let minted: BTreeSet<&str> = minted_ids.iter().map(String::as_str).collect();
    let npm = "scip-typescript npm @acme/core 1.0.0";
    for (symbol, id) in [
        ("src/`board.ts`/Board#", board(&["Board"])),
        ("src/`board.ts`/Board#add().", board(&["Board", "add"])),
        (
            "src/`board.ts`/Board#`<constructor>`().",
            board(&["Board", "constructor"]),
        ),
        (
            "src/`board.ts`/Board#`<get>size`().",
            board(&["Board", "size"]),
        ),
        (
            "src/`board.ts`/Board#`<set>size`().",
            board(&["Board", "size"]),
        ),
        ("src/`board.ts`/addTask().", board(&["addTask"])),
        ("src/`board.ts`/addTask().(task)", board(&["addTask"])),
        ("src/`board.ts`/Board#[T]", board(&["Board"])),
        ("src/`board.ts`/Board#items.", board(&["Board"])),
        ("src/`board.ts`/NS/inner().", board(&["NS", "inner"])),
        ("src/`board.ts`/Shape#", board(&["Shape"])),
        (
            "src/`index.ts`/VERSION.",
            "js:@acme/core/src/index.ts::VERSION".to_string(),
        ),
    ] {
        let mapped = scip_typescript(&format!("{npm} {symbol}"), &minted);
        assert_eq!(mapped, Some(id), "{symbol}");
    }
}
