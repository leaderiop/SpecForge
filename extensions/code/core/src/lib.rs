//! @code/core — the neutral base of code facts (ADR 0329, ADR 0347).
//!
//! It declares the facts the code packs collect, and judges the references a spec writes to them:
//!
//! - `test` (schemes `js`, `rust`): a test a runner ran, `js:<package>/<file>#<s1 > … > name>` or
//!   `rust:<nextest binary id>#<test path>`, with its package, file, suite path, name, `binary` and `status`, its
//!   result (`result("status")`), and the entities it says it proves (`names`, edge `names`: each `[<ns>:<id>]` tag
//!   in its JUnit name, or its source's idiom);
//! - `test_run` (scheme `run`): the run of a test, `run:<test id>`, with the `test` it ran (edge `run_of`), the same
//!   `status` and its `duration_ms`;
//! - `item` (schemes `rust`, `js`): a definition path of a source pack, `rust:<crate>::<path>` or
//!   `js:<package>/<file>::<name>`, with its `package` (edge `in_package`), file, kinds, visibility, signature digest,
//!   doc, attrs, `names` and the `test_id` it runs as (edge `runs_as`);
//! - `package` (scheme `pkg`): a cargo or npm package, a purl without version, with its manifest's values and the
//!   project packages it `depends_on` (edge `uses_package`).
//!
//! Every kind is collector-written and none is scoped: a scheme reference names the fact whole. `@code/core` declares
//! no collector: a pack collects, minting the facts with the linked `specforge-code-facts` library, so every pack
//! names a fact the same way. It is a builtin outside `@specforge`, so it catalogues its codes and they print
//! namespaced: `code/E101` (ADR 0328 D2).
//!
//! Its check pass `refs` judges a reference no fact holds by the collected kinds its field admits (E101 when a store
//! holds one, W102 naming the pack otherwise), unless the citing entity is pending (ADR 0329 D4, ADR 0347 D9); its
//! check pass `proof` reports a complete entity linking a failed test (W401), a test or item naming no entity (I402)
//! (ADR 0329 D6, ADR 0347 D11) and a fact's own reference that answers nothing (E101, ADR 0346 D4). `refs` is sent
//! the `test` and `test_run` facts, `proof` the facts of every kind (ADR 0353 D6).

use specforge_code_facts::{ItemKind, Scheme, Visibility};
use specforge_extension_sdk::prelude::*;

mod proof;
mod refs;

/// The kind of a test fact.
const TEST_KIND: &str = "test";
/// The kind of the run of a test.
const RUN_KIND: &str = "test_run";
/// The kind of a definition a source pack read.
const ITEM_KIND: &str = "item";
/// The kind of a package a manifest pack read.
const PACKAGE_KIND: &str = "package";
/// Every kind `@code/core` declares.
const KINDS: [&str; 4] = [TEST_KIND, RUN_KIND, ITEM_KIND, PACKAGE_KIND];
/// The schemes of a `test`'s id.
const TEST_SCHEMES: [&str; 2] = [Scheme::Js.as_str(), Scheme::Rust.as_str()];
/// The scheme of a `test_run`'s id.
const RUN_SCHEME: &str = "run";
/// The schemes of an `item`'s id.
const ITEM_SCHEMES: [&str; 2] = [Scheme::Rust.as_str(), Scheme::Js.as_str()];
/// The scheme of a `package`'s id.
const PACKAGE_SCHEME: &str = "pkg";

/// The field holding a `test`'s result (`result`), and a `test_run`'s status.
const RESULT: &str = "status";
/// The field holding the ids of the entities a `test` or an `item` says it proves.
const NAMES: &str = "names";

/// A reference field `@code/core` declares with a target kind: a row of [`REFERENCES`].
pub(crate) struct Reference {
    pub kind: &'static str,
    pub field: &'static str,
    pub target: &'static str,
    pub edge: &'static str,
    /// Whether one collector answers the holder and its target together: an unheld one is then its disagreement.
    pub together: bool,
}

/// Every reference field `@code/core` declares with a target kind: the one table its declarations read for the
/// target and the edge, and `refs` for what a mistyped reference should have named and whose an unheld one is.
pub(crate) const REFERENCES: [Reference; 4] = [
    Reference {
        kind: ITEM_KIND,
        field: "package",
        target: PACKAGE_KIND,
        edge: "in_package",
        together: false,
    },
    Reference {
        kind: ITEM_KIND,
        field: "test_id",
        target: TEST_KIND,
        edge: "runs_as",
        together: false,
    },
    Reference {
        kind: PACKAGE_KIND,
        field: "depends_on",
        target: PACKAGE_KIND,
        edge: "uses_package",
        together: true,
    },
    Reference {
        kind: RUN_KIND,
        field: "test",
        target: TEST_KIND,
        edge: "run_of",
        together: true,
    },
];

/// The row of [`REFERENCES`] for the field `field` of `kind`.
pub(crate) fn reference(kind: &str, field: &str) -> Option<&'static Reference> {
    REFERENCES
        .iter()
        .find(|r| r.kind == kind && r.field == field)
}

/// `f` targeting and labelled as [`REFERENCES`] declares the field `field` of `kind`.
fn referencing<'f>(f: &'f mut FieldBuilder, kind: &str, field: &str) -> &'f mut FieldBuilder {
    let row = reference(kind, field).expect("a row of REFERENCES");
    f.target_kind(row.target).edge(row.edge)
}

/// A test's status: the `TestStatus` names, the one vocabulary of the collect wire.
const STATUS: Vocabulary = Vocabulary::new(&["passed", "failed", "skipped"]);
/// What a definition is: the library's `ItemKind` names.
const ITEM_KINDS: Vocabulary = Vocabulary::new(&ItemKind::NAMES);
/// Who may name a definition: the library's `Visibility` names, widest first.
const VISIBILITY: Vocabulary = Vocabulary::new(&Visibility::NAMES);

#[specforge_extension_sdk::extension(
    name = "@code/core",
    version = "1.1.0",
    description = "The neutral base of code facts: the test, test_run, item and package kinds the code packs collect, and the checks of the references a spec writes to them"
)]
struct CodeCore;

impl Contributions for CodeCore {
    fn contribute(c: &mut ContributionsBuilder) {
        c.kind(TEST_KIND, |k| {
            k.description(
                "A test a runner ran, written by its runner pack's collector (ADR 0329): cite it by its id, \
                 js:<package>/<file>#<suite > … > name> or rust:<nextest binary id>#<test path>",
            )
            .collected(&TEST_SCHEMES)
            .result(RESULT)
            .field("package", |f| {
                f.field_type(FieldType::String)
                    .description("The package the runner pack assigns the test to");
            })
            .field("file", |f| {
                f.field_type(FieldType::String)
                    .description("The test file, project-relative, with / separators");
            })
            .field("suite", |f| {
                f.field_type(FieldType::StringList)
                    .description("The suites the test is nested in, outermost first");
            })
            .field("name", |f| {
                f.field_type(FieldType::String)
                    .description("The test's own name");
            })
            .field("binary", |f| {
                f.field_type(FieldType::String)
                    .description("The nextest binary id of a Rust test");
            })
            .field(RESULT, |f| {
                f.vocabulary(&STATUS)
                    .description("What its last run said: passed, failed or skipped");
            })
            .field(NAMES, |f| {
                f.field_type(FieldType::ReferenceList).edge(NAMES).description(
                    "The entities the test says it proves: each [<ns>:<id>] tag in its JUnit name, or its \
                     source's idiom (I402 when one is no entity)",
                );
            });
        });
        c.kind(RUN_KIND, |k| {
            k.description(
                "The run of a test (ADR 0329): run:<test id>, with the status the run gave it",
            )
            .collected(&[RUN_SCHEME])
            .field("test", |f| {
                referencing(f, RUN_KIND, "test")
                    .field_type(FieldType::Reference)
                    .description("The test this is the run of");
            })
            .field(RESULT, |f| {
                f.vocabulary(&STATUS)
                    .description("What the run said: the test's own status");
            })
            .field("duration_ms", |f| {
                f.field_type(FieldType::Integer)
                    .description("How long the run took, in milliseconds");
            });
        });
        c.kind(ITEM_KIND, |k| {
            k.description(
                "A definition a source pack read (ADR 0347): every definition one path reaches is one item, cited by \
                 its id, rust:<crate>::<module path>::<name> or js:<package>/<file>::<name>[.<member>]",
            )
            .collected(&ITEM_SCHEMES)
            .field("package", |f| {
                referencing(f, ITEM_KIND, "package")
                    .field_type(FieldType::Reference)
                    .description("The package the item is defined in");
            })
            .field("file", |f| {
                f.field_type(FieldType::String)
                    .description("The file of its first definition, project-relative, with / separators");
            })
            .field("kind", |f| {
                f.enum_list(&ITEM_KINDS)
                    .description("What each of its definitions is, in vocabulary order");
            })
            .field("visibility", |f| {
                f.vocabulary(&VISIBILITY)
                    .description("Who may name it: the widest of its definitions'");
            })
            .field("signature", |f| {
                f.field_type(FieldType::String).description(
                    "sha256:<hex> over its definitions' signature texts, whitespace-collapsed: it changes when a \
                     signature does",
                );
            })
            .field("doc", |f| {
                f.field_type(FieldType::String)
                    .description("Its first definition's doc comment");
            })
            .field("attrs", |f| {
                f.field_type(FieldType::StringList)
                    .description("Its definitions' attributes: test, ignore, cfg(<predicate>), export, …");
            })
            .field(NAMES, |f| {
                f.field_type(FieldType::ReferenceList).edge(NAMES).description(
                    "The entities the item says it implements, in its source's idiom (I402 when one is no entity)",
                );
            })
            .field("test_id", |f| {
                referencing(f, ITEM_KIND, "test_id")
                    .field_type(FieldType::Reference)
                    .description("The test this function runs as, when it is one");
            });
        });
        c.kind(PACKAGE_KIND, |k| {
            k.description(
                "A package a manifest pack read (ADR 0347): pkg:cargo/<name>, pkg:cargo/<name>#<dir> or \
                 pkg:npm/<name>, a purl without its version",
            )
            .collected(&[PACKAGE_SCHEME])
            .field("name", |f| {
                f.field_type(FieldType::String)
                    .description("The name its manifest gives it");
            })
            .field("version", |f| {
                f.field_type(FieldType::String)
                    .description("The version its manifest gives it");
            })
            .field("edition", |f| {
                f.field_type(FieldType::String)
                    .description("A cargo package's Rust edition");
            })
            .field("rust_version", |f| {
                f.field_type(FieldType::String)
                    .description("A cargo package's minimum supported Rust version");
            })
            .field("engines", |f| {
                f.field_type(FieldType::StringList)
                    .description("An npm package's engines, each <engine> <range>");
            })
            .field("features", |f| {
                f.field_type(FieldType::StringList)
                    .description("A cargo package's features");
            })
            .field("targets", |f| {
                f.field_type(FieldType::StringList)
                    .description("A cargo package's targets, each <kind>:<name>:<root>");
            })
            .field("path", |f| {
                f.field_type(FieldType::String)
                    .description("Its directory, project-relative, empty at the root");
            })
            .field("depends_on", |f| {
                referencing(f, PACKAGE_KIND, "depends_on")
                    .field_type(FieldType::ReferenceList)
                    .description("The project's own packages it depends on");
            });
        });
        c.edge("run_of", |e| {
            e.description("A run of a test")
                .source_kind(RUN_KIND)
                .target_kind(TEST_KIND);
        });
        c.edge(NAMES, |e| {
            e.description("An entity a test or an item says it proves or implements");
        });
        c.edge("in_package", |e| {
            e.description("The package an item is defined in")
                .source_kind(ITEM_KIND)
                .target_kind(PACKAGE_KIND);
        });
        c.edge("uses_package", |e| {
            e.description("A project package a package depends on")
                .source_kind(PACKAGE_KIND)
                .target_kind(PACKAGE_KIND);
        });
        c.edge("runs_as", |e| {
            e.description("The test a function runs as")
                .source_kind(ITEM_KIND)
                .target_kind(TEST_KIND);
        });

        // A pass is sent the facts of the kinds it declares (ADR 0353): `refs` names the closest held test or run,
        // and `proof` judges what the facts of every kind say, so item facts cross into one guest.
        c.pass("refs", |p| {
            p.phase("check")
                .input(Input::facts(&[TEST_KIND, RUN_KIND]))
                .run(refs::refs);
        });
        c.pass("proof", |p| {
            p.phase("check")
                .input(Input::facts(&KINDS))
                .run(proof::proof);
        });

        c.code("E101", |d| {
            d.title("A fact reference names no fact").explanation(
                "A reference names no collected fact of the kinds its field admits, narrowed to those its id is \
                 shaped as (a test id holds `#`, an item id never does), while a fact store the session served holds \
                 facts of one of those kinds in its scheme: the test, item or package was renamed, moved or \
                 deleted, or the reference is misspelled. A test or run reference is told the closest one of the \
                 same file or binary. It is not reported while the citing entity's owner is pending (ADR 0329 D4): \
                 a test not written yet is TDD. Fix the reference, or run `specforge collect` if what it names is \
                 new. A reference one of its own facts holds (a test run's test) that no fact or entity answers, or \
                 that names an entity of another kind, is reported at the place its collector read it, addressed \
                 to that collector: its facts disagree, and running `specforge collect` again, or fixing the pack, \
                 is the remedy (ADR 0346).",
            );
        });
        c.code("W102", |d| {
            d.title("No facts of the reference's kinds were collected").explanation(
                "A reference names no collected fact, and no fact store the session served holds facts of the kinds \
                 its field admits (narrowed to those its id is shaped as) in its scheme: on a fresh clone, or before \
                 the pack that writes them ran, there is nothing to check it against. It is reported once per cited \
                 reference, and not while the citing entity's owner is pending. Run `specforge collect` with the \
                 pack the finding names: `@javascript/node-test` for `js` tests, `@rust/test` for `rust` tests, \
                 `@rust/source` for `rust` items, `@javascript/source` for `js` items, `@rust/cargo` for \
                 `pkg:cargo` packages, `@javascript/source` for `pkg:npm` packages.",
            );
        });
        c.code("W401", |d| {
            d.title("A linked test fails").explanation(
                "An entity whose owner is complete links a test whose last collected status is `failed`: the proof \
                 it cites does not hold. A pending owner's failing test is not reported (TDD). Fix the code or the \
                 test, and run `specforge collect` again.",
            );
        });
        c.code("I402", |d| {
            d.title("A test or item names an entity that does not exist").explanation(
                "A test or item fact's `names` (the entities its source's idiom says it proves or implements) \
                 holds a name that is no entity of the project, or the short name of several: the entity was \
                 renamed or deleted, or the name is misspelled. Fix the name in the source, then collect again.",
            );
        });
    }
}

specforge_extension_sdk::component_guest!(build = specforge_extension_build);

#[cfg(test)]
mod tests;
