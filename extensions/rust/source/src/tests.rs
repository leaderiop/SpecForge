//! The pack over small projects, its query run by the host's own reader (`specforge-source`) behind the in-process
//! syntax import, under a counted deadline.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use serde_json::json;
use specforge_code_facts::{Package, PackageId, Target};
use specforge_extension_sdk::host::in_process::{serve, InProcessHost};
use specforge_extension_sdk::host::{
    HostError, NotParsed, SyntaxAnswer, SyntaxFile, SyntaxFiles, SyntaxOutcome, SyntaxRequest,
};
use specforge_source::{Language, SourceQuery, COUNTED_DEADLINE};
use specforge_test::prelude::*;

use super::*;

/// The project's files in memory, parsed as the host parses them; with a cap, only the first `cap` files of an
/// answer are parsed and the rest `capped`, as the host's answer cap leaves them.
struct Host {
    files: BTreeMap<String, String>,
    cap: Option<usize>,
    asked: RefCell<Vec<Vec<String>>>,
}

impl InProcessHost for Host {
    fn syntax_parse(&self, request: &SyntaxRequest) -> Result<SyntaxAnswer, HostError> {
        let SyntaxFiles::Paths(paths) = &request.files else {
            panic!("rust-source asks named files: {request:?}");
        };
        assert_eq!(request.language, "rust");
        self.asked.borrow_mut().push(paths.clone());
        let query =
            SourceQuery::compile(Language::Rust, &request.query).expect("the query compiles");
        let capped = |at: usize| self.cap.is_some_and(|cap| at >= cap);
        let files = paths
            .iter()
            .enumerate()
            .map(|(at, path)| SyntaxFile {
                path: path.clone(),
                digest: None,
                outcome: match self.files.get(path) {
                    _ if capped(at) => SyntaxOutcome::NotParsed {
                        reason: NotParsed::Capped,
                    },
                    Some(text) => query.parse(text.as_bytes(), COUNTED_DEADLINE),
                    None => SyntaxOutcome::NotParsed {
                        reason: NotParsed::Missing,
                    },
                },
            })
            .collect();
        Ok(SyntaxAnswer {
            files,
            truncated: capped(paths.len().saturating_sub(1)),
            parsed: 0,
            cached: 0,
        })
    }
}

/// The package `toy` (edition 2021) with these written targets, as `@rust/cargo` answers it.
fn package(targets: &[&str]) -> Fact {
    Package {
        id: PackageId::cargo("toy"),
        version: Some("0.1.0".to_string()),
        edition: Some("2021".to_string()),
        rust_version: None,
        engines: Vec::new(),
        features: Vec::new(),
        targets: targets
            .iter()
            .map(|t| Target::read(t).expect("a target"))
            .collect(),
        path: String::new(),
        depends_on: Vec::new(),
        at: None,
    }
    .fact()
}

/// What rust-source answers over `files` for the lib `src/lib.rs` (and `targets`), and the files each request asked.
fn collected_with(
    files: &[(&str, &str)],
    targets: &[&str],
    cap: Option<usize>,
) -> (Result<Vec<Fact>, String>, Vec<Vec<String>>) {
    let host = Rc::new(Host {
        files: files
            .iter()
            .map(|(path, text)| (path.to_string(), text.to_string()))
            .collect(),
        cap,
        asked: RefCell::new(Vec::new()),
    });
    let input = CollectInput {
        facts: vec![package(targets)],
        ..CollectInput::default()
    };
    let out = serve(host.clone(), || collect(&input)).map(|out| out.facts);
    let asked = host.asked.borrow().clone();
    (out, asked)
}

/// The item facts over `files`, the lib `src/lib.rs` the one target.
fn collected(files: &[(&str, &str)]) -> Vec<Fact> {
    collected_with(files, &["lib:toy:src/lib.rs"], None)
        .0
        .expect("rust-source collects")
}

fn ids(facts: &[Fact]) -> Vec<&str> {
    facts.iter().map(|f| f.id.as_str()).collect()
}

fn item<'f>(facts: &'f [Fact], id: &str) -> &'f Fact {
    facts
        .iter()
        .find(|f| f.id == id)
        .unwrap_or_else(|| panic!("no item {id} among {:?}", ids(facts)))
}

/// Each place of `fact`: `(file, start line, start col, end line, end col)`.
fn places(fact: &Fact) -> Vec<(&str, usize, usize, usize, usize)> {
    fact.at
        .iter()
        .map(|at| {
            (
                at.file.as_str(),
                at.start_line,
                at.start_col,
                at.end_line,
                at.end_col,
            )
        })
        .collect()
}

#[specforge_test(
    behavior = "rs_declare_source_collector",
    verify = "rust-source declares its collector, its input, its capability and its peers"
)]
fn rust_source_declares_its_collector_its_input_and_its_peers() {
    let declaration = crate::specforge_extension_build().declaration();
    assert_eq!(declaration.handshake.name, "@rust/source");
    let peers: Vec<(&str, &str, bool)> = declaration
        .handshake
        .peer_dependencies
        .iter()
        .map(|p| (p.name.as_str(), p.version.as_str(), p.optional))
        .collect();
    assert_eq!(
        peers,
        [
            ("@code/core", "^1.1", false),
            ("@rust/cargo", "^1.0", false)
        ]
    );
    let [collector] = &declaration.collectors[..] else {
        panic!("one collector: {:?}", declaration.collectors);
    };
    assert_eq!(collector.name, COLLECTOR);
    assert_eq!(collector.answers, ["item"]);
    assert_eq!(collector.inputs, [Input::facts(&["package"])]);
    assert_eq!(
        (&collector.capability, &collector.report),
        (&None, &None),
        "no command, no report"
    );
    let capabilities = serde_json::to_value(&declaration.capabilities).unwrap();
    assert_eq!(
        capabilities,
        json!([{
            "name": COLLECTOR,
            "port": "fs",
            "roots": ["project"],
            "ops": ["read"],
            "call_kinds": ["collect"],
            "why": "parse the project's Rust source with the host's reader",
        }])
    );
    assert!(SourceQuery::compile(Language::Rust, QUERY).is_ok());
}

#[specforge_test(
    behavior = "rs_resolve_module_tree",
    verify = "the tree follows mod declarations from each target's root"
)]
fn the_tree_follows_mod_declarations_from_each_root() {
    let facts = collected(&[
        ("src/lib.rs", "mod a;\npub mod b;\n"),
        ("src/a.rs", "pub fn f() {}\n"),
        ("src/b/mod.rs", "pub struct S;\n"),
        ("src/unreached.rs", "pub fn never() {}\n"),
    ]);
    assert_eq!(
        ids(&facts),
        [
            "rust:toy::a",
            "rust:toy::a::f",
            "rust:toy::b",
            "rust:toy::b::S"
        ]
    );
    assert_eq!(
        item(&facts, "rust:toy::b::S").fields["file"],
        "src/b/mod.rs"
    );
}

#[specforge_test(
    behavior = "rs_resolve_module_tree",
    verify = "a non-mod-rs file keeps its children in a directory of its stem"
)]
fn a_non_mod_rs_file_keeps_its_children_in_its_own_directory() {
    let facts = collected(&[
        ("src/lib.rs", "mod a;\nmod b;\n"),
        ("src/a.rs", "mod c;\n"),
        ("src/a/c.rs", "fn g() {}\n"),
        ("src/c.rs", "fn wrong() {}\n"),
        ("src/b/mod.rs", "mod d;\n"),
        ("src/b/d.rs", "fn h() {}\n"),
    ]);
    assert_eq!(
        ids(&facts),
        [
            "rust:toy::a",
            "rust:toy::a::c",
            "rust:toy::a::c::g",
            "rust:toy::b",
            "rust:toy::b::d",
            "rust:toy::b::d::h",
        ]
    );
}

#[specforge_test(
    behavior = "rs_resolve_module_tree",
    verify = "a path attribute is relative to the declaring file's directory"
)]
fn a_path_attribute_is_relative_to_the_declaring_file() {
    let facts = collected(&[
        ("src/lib.rs", "mod a;\n"),
        ("src/a.rs", "#[path = \"other.rs\"]\nmod p;\n"),
        ("src/other.rs", "mod q;\nfn in_other() {}\n"),
        ("src/q.rs", "fn in_q() {}\n"),
    ]);
    assert_eq!(
        ids(&facts),
        [
            "rust:toy::a",
            "rust:toy::a::p",
            "rust:toy::a::p::in_other",
            "rust:toy::a::p::q",
            "rust:toy::a::p::q::in_q",
        ],
        "a file a path attribute reaches keeps its children beside it"
    );
}

#[specforge_test(
    behavior = "rs_resolve_module_tree",
    verify = "a path inside an inline module is relative to the inline module's directory"
)]
fn a_path_inside_an_inline_module_is_relative_to_its_directory() {
    let facts = collected(&[
        (
            "src/lib.rs",
            "mod outer {\n    #[path = \"x.rs\"]\n    mod inner;\n    mod plain;\n}\n",
        ),
        ("src/outer/x.rs", "fn in_x() {}\n"),
        ("src/outer/plain.rs", "fn in_plain() {}\n"),
    ]);
    assert_eq!(
        ids(&facts),
        [
            "rust:toy::outer",
            "rust:toy::outer::inner",
            "rust:toy::outer::inner::in_x",
            "rust:toy::outer::plain",
            "rust:toy::outer::plain::in_plain",
        ]
    );
}

#[specforge_test(
    behavior = "rs_resolve_module_tree",
    verify = "every path a cfg_attr names is followed and its twins merge into one item"
)]
fn a_cfg_attr_path_is_followed_and_its_twins_merge() {
    let facts = collected(&[
        (
            "src/lib.rs",
            "#[cfg_attr(unix, path = \"sys/unix.rs\")]\n#[cfg_attr(windows, path = \"sys/windows.rs\")]\nmod sys;\n",
        ),
        ("src/sys/unix.rs", "pub fn open() {}\n"),
        ("src/sys/windows.rs", "\npub fn open() {}\n"),
    ]);
    assert_eq!(ids(&facts), ["rust:toy::sys", "rust:toy::sys::open"]);
    assert_eq!(
        places(item(&facts, "rust:toy::sys::open")),
        [
            ("src/sys/unix.rs", 1, 1, 1, 17),
            ("src/sys/windows.rs", 2, 1, 2, 17)
        ]
    );
}

#[specforge_test(
    behavior = "rs_resolve_module_tree",
    verify = "a file two targets reach mints items in each crate"
)]
fn a_file_two_targets_reach_mints_items_in_each() {
    let (facts, asked) = collected_with(
        &[
            ("src/lib.rs", "mod shared;\n"),
            ("src/main.rs", "mod shared;\nfn main() {}\n"),
            ("src/shared.rs", "pub fn f() {}\n"),
        ],
        &["lib:toy:src/lib.rs", "bin:toy:src/main.rs"],
        None,
    );
    assert_eq!(
        ids(&facts.unwrap()),
        [
            "rust:toy/bin/toy::main",
            "rust:toy/bin/toy::shared",
            "rust:toy/bin/toy::shared::f",
            "rust:toy::shared",
            "rust:toy::shared::f",
        ]
    );
    let shared = asked
        .iter()
        .flatten()
        .filter(|path| *path == "src/shared.rs")
        .count();
    assert_eq!(shared, 1, "a file is parsed once: {asked:?}");
}

#[specforge_test(
    behavior = "rs_mint_item_facts",
    verify = "an item carries its kind, visibility, signature, doc and location"
)]
fn items_carry_kind_visibility_signature_and_location() {
    let facts = collected(&[(
        "src/lib.rs",
        "/// Adds them.\npub fn add<T>(a: T, b: T) -> T { a }\npub(crate) struct Board;\nconst MAX: u8 = 3;\n\
         pub(super) static NAME: &str = \"x\";\nmacro_rules! m { () => {} }\npub type Id = u32;\n\
         pub enum E { A }\npub union U { a: u8 }\npub trait T {}\n",
    )]);
    let summary: Vec<(&str, &str, &str)> = facts
        .iter()
        .map(|f| {
            (
                f.id.as_str(),
                f.fields["kind"][0].as_str().unwrap(),
                f.fields["visibility"].as_str().unwrap(),
            )
        })
        .collect();
    assert_eq!(
        summary,
        [
            ("rust:toy::Board", "struct", "crate"),
            ("rust:toy::E", "enum", "public"),
            ("rust:toy::Id", "type", "public"),
            ("rust:toy::MAX", "const", "private"),
            ("rust:toy::NAME", "static", "restricted"),
            ("rust:toy::T", "trait", "public"),
            ("rust:toy::U", "union", "public"),
            ("rust:toy::add", "function", "public"),
            ("rust:toy::m", "macro", "private"),
        ]
    );
    let add = item(&facts, "rust:toy::add");
    assert_eq!(add.fields["doc"], "Adds them.");
    assert_eq!(add.fields["package"], "pkg:cargo/toy");
    assert!(add.fields["signature"]
        .as_str()
        .unwrap()
        .starts_with("sha256:"));
    assert_eq!(places(add), [("src/lib.rs", 2, 1, 2, 37)]);
    assert!(!item(&facts, "rust:toy::Board")
        .fields
        .contains_key("signature"));
}

#[specforge_test(
    behavior = "rs_mint_item_facts",
    verify = "impl and trait items are named by their owner"
)]
fn impl_and_trait_items_are_named_by_their_owner() {
    let facts = collected(&[(
        "src/lib.rs",
        "pub struct Board;\nimpl Board {\n    pub fn add(&mut self) {}\n    const SIZE: u8 = 9;\n}\n\
         impl std::fmt::Display for Board {\n    fn fmt(&self, f: &mut Formatter) -> Result { Ok(()) }\n}\n\
         pub trait Shape {\n    fn area(&self) -> f64;\n    type Unit;\n}\n",
    )]);
    let summary: Vec<(&str, &str)> = facts
        .iter()
        .map(|f| (f.id.as_str(), f.fields["kind"][0].as_str().unwrap()))
        .collect();
    assert_eq!(
        summary,
        [
            ("rust:toy::<Board as Display>::fmt", "method"),
            ("rust:toy::Board", "struct"),
            ("rust:toy::Board::SIZE", "const"),
            ("rust:toy::Board::add", "method"),
            ("rust:toy::Shape", "trait"),
            ("rust:toy::Shape::Unit", "type"),
            ("rust:toy::Shape::area", "method"),
        ]
    );
}

#[specforge_test(
    behavior = "rs_mint_item_facts",
    verify = "an item inside a function body or another item's block is not minted"
)]
fn items_inside_a_function_body_are_not_minted() {
    let facts = collected(&[(
        "src/lib.rs",
        "fn outer() {\n    fn inner() {}\n    struct Local;\n    impl Local { fn m(&self) {} }\n}\n\
         const C: () = { fn hidden() {} };\n",
    )]);
    assert_eq!(ids(&facts), ["rust:toy::C", "rust:toy::outer"]);
}

#[specforge_test(
    behavior = "rs_resolve_module_tree",
    verify = "a module whose file is absent or outside the project keeps its mod item only"
)]
fn an_absent_module_file_keeps_the_mod_item_only() {
    let (facts, asked) = collected_with(
        &[(
            "src/lib.rs",
            "mod gone;\n#[path = \"../../outside.rs\"]\nmod out;\n",
        )],
        &["lib:toy:src/lib.rs"],
        None,
    );
    assert_eq!(ids(&facts.unwrap()), ["rust:toy::gone", "rust:toy::out"]);
    assert_eq!(
        asked,
        [
            vec!["src/lib.rs".to_string()],
            vec!["src/gone.rs".to_string(), "src/gone/mod.rs".to_string()],
        ],
        "a path out of the project is never asked"
    );
}

#[specforge_test(
    behavior = "rs_resolve_module_tree",
    verify = "the files an answer's cap left unparsed are asked again"
)]
fn a_capped_answer_is_asked_again() {
    let files = [
        ("src/lib.rs", "mod a;\nmod b;\nmod c;\n"),
        ("src/a.rs", "fn fa() {}\n"),
        ("src/b.rs", "fn fb() {}\n"),
        ("src/c.rs", "fn fc() {}\n"),
    ];
    let (capped, asked) = collected_with(&files, &["lib:toy:src/lib.rs"], Some(2));
    let (whole, _) = collected_with(&files, &["lib:toy:src/lib.rs"], None);
    assert_eq!(capped, whole);
    assert!(
        asked.len() > 2,
        "the capped files are asked again: {asked:?}"
    );
    let (stuck, _) = collected_with(&files, &["lib:toy:src/lib.rs"], Some(0));
    assert!(
        stuck.as_ref().unwrap_err().contains("answer cap"),
        "{stuck:?}"
    );
}

#[specforge_test(
    behavior = "rs_mint_item_facts",
    verify = "attributes and doc comments belong to the item directly below them"
)]
fn attributes_belong_to_the_item_directly_below_them() {
    let facts = collected(&[(
        "src/lib.rs",
        "#[cfg(unix)]\n/// Docs of a.\n#[inline]\nfn a() {}\n\n#[cfg(windows)]\n\nfn b() {}\n#[cfg(test)] fn c() {}\n\
         #[cfg(all(unix,  test))]\nmod m {\n    #[cfg(x)]\n    fn d() {}\n}\n",
    )]);
    let attrs = |id: &str| item(&facts, id).fields.get("attrs").cloned();
    assert_eq!(attrs("rust:toy::a"), Some(json!(["cfg(unix)"])));
    assert_eq!(item(&facts, "rust:toy::a").fields["doc"], "Docs of a.");
    assert_eq!(attrs("rust:toy::b"), None, "a blank line ends the run");
    assert_eq!(attrs("rust:toy::c"), Some(json!(["cfg(test)"])));
    assert_eq!(attrs("rust:toy::m"), Some(json!(["cfg(all(unix, test))"])));
    assert_eq!(attrs("rust:toy::m::d"), Some(json!(["cfg(x)"])));
}

/// `(attrs, test_id)` of the item `id`.
fn marks(facts: &[Fact], id: &str) -> (Option<serde_json::Value>, Option<serde_json::Value>) {
    let fact = item(facts, id);
    (
        fact.fields.get("attrs").cloned(),
        fact.fields.get("test_id").cloned(),
    )
}

#[specforge_test(
    behavior = "rs_mark_tests",
    verify = "a test function carries the id nextest gives it"
)]
fn a_test_function_carries_the_id_nextest_gives_it() {
    let facts = collected(&[(
        "src/lib.rs",
        "#[cfg(test)]\nmod tests {\n    #[test]\n    fn adds() {}\n    fn helper() {}\n}\n",
    )]);
    assert_eq!(
        marks(&facts, "rust:toy::tests::adds"),
        (Some(json!(["test"])), Some(json!("rust:toy#tests::adds")))
    );
    assert_eq!(marks(&facts, "rust:toy::tests::helper"), (None, None));
    assert_eq!(
        marks(&facts, "rust:toy::tests"),
        (Some(json!(["cfg(test)"])), None)
    );
}

#[specforge_test(
    behavior = "rs_mark_tests",
    verify = "the same test path in three binaries is three ids"
)]
fn the_same_test_path_in_three_binaries_is_three_ids() {
    let tests = "#[cfg(test)]\nmod tests {\n    #[test]\n    fn adds() {}\n}\n";
    let (facts, _) = collected_with(
        &[
            ("src/lib.rs", tests),
            ("src/main.rs", tests),
            ("tests/api.rs", tests),
        ],
        &[
            "lib:toy:src/lib.rs",
            "bin:toy:src/main.rs",
            "test:api:tests/api.rs",
        ],
        None,
    );
    let test_ids: Vec<&str> = facts
        .as_ref()
        .unwrap()
        .iter()
        .filter_map(|f| f.fields.get("test_id"))
        .filter_map(|id| id.as_str())
        .collect();
    assert_eq!(
        test_ids,
        [
            "rust:toy::bin/toy#tests::adds",
            "rust:toy::api#tests::adds",
            "rust:toy#tests::adds",
        ],
        "by item id: the bin's, the integration test's, the lib's"
    );
}

#[specforge_test(
    behavior = "rs_mark_tests",
    verify = "an ignored test is marked ignore"
)]
fn an_ignored_test_is_marked() {
    let facts = collected(&[(
        "src/lib.rs",
        "#[test]\n#[ignore = \"slow\"]\nfn slow() {}\n#[ignore]\nfn not_a_test() {}\n",
    )]);
    assert_eq!(
        marks(&facts, "rust:toy::slow").0,
        Some(json!(["ignore", "test"]))
    );
    assert_eq!(marks(&facts, "rust:toy::not_a_test"), (None, None));
}

#[specforge_test(
    behavior = "rs_mark_tests",
    verify = "a spec-link attribute names its entity at the id's string"
)]
fn a_specforge_test_attribute_names_its_entity() {
    let facts = collected(&[(
        "src/lib.rs",
        "#[specforge_test(behavior = \"rc_read\", verify = \"it reads\")]\nfn one() {}\n\
         #[specforge_test_macros::test(\n    feature = \"rc_feature\",\n)]\nfn two() {}\n\
         #[tokio::test(flavor = \"current_thread\")]\nasync fn three() {}\n",
    )]);
    let one = item(&facts, "rust:toy::one");
    assert_eq!(one.fields["names"], json!(["rc_read"]));
    let at: Vec<(usize, usize, usize, usize)> = one.named_at["names"]["rc_read"]
        .iter()
        .map(|at| (at.start_line, at.start_col, at.end_line, at.end_col))
        .collect();
    assert_eq!(at, [(1, 30, 1, 37)], "the id inside its quotes");
    assert_eq!(
        item(&facts, "rust:toy::two").fields["names"],
        json!(["rc_feature"])
    );
    let three = item(&facts, "rust:toy::three");
    assert!(
        !three.fields.contains_key("names"),
        "tokio::test names no entity"
    );
    assert_eq!(three.fields["attrs"], json!(["test"]));
}

#[specforge_test(
    behavior = "rs_mark_tests",
    verify = "a runner attribute whose path ends in ::test marks a test"
)]
fn a_runner_attribute_ending_in_test_marks_a_test() {
    let facts = collected(&[(
        "src/lib.rs",
        "#[tokio::test]\nasync fn a() {}\n#[spec(behavior = \"x\")]\nfn b() {}\n#[rstest]\nfn c() {}\n\
         #[test_case(1)]\nfn d() {}\n",
    )]);
    let tests: Vec<&str> = facts
        .iter()
        .filter(|f| f.fields.contains_key("test_id"))
        .map(|f| f.id.as_str())
        .collect();
    assert_eq!(tests, ["rust:toy::a", "rust:toy::b"]);
}

#[specforge_test(
    behavior = "rs_mark_tests",
    verify = "a test in an inline module keeps its module path"
)]
fn a_test_in_an_inline_module_keeps_its_module_path() {
    let facts = collected(&[
        ("src/lib.rs", "mod x;\n"),
        (
            "src/x.rs",
            "mod a {\n    mod b {\n        #[test]\n        fn r#match() {}\n    }\n}\n",
        ),
    ]);
    assert_eq!(
        marks(&facts, "rust:toy::x::a::b::match").1,
        Some(json!("rust:toy#x::a::b::r#match")),
        "libtest writes a reserved word raw"
    );
}
