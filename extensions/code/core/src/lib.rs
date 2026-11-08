//! @code/core — the neutral base of code facts (ADR 0329, ADR 0347).
//!
//! It declares the facts the code packs collect, and judges the references a spec writes to them:
//!
//! - `test` (schemes `js`, `rust`): a test a runner ran, `js:<package>/<file>#<s1 > … > name>` or
//!   `rust:<nextest binary id>#<test path>`, with its package, file, suite path, name, `binary` and `status`, its
//!   result (`result("status")`), and the entities it says it proves (`names`, edge `names`);
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
//! check pass `proof` reports a complete entity linking a failed test (W401) and a test or item naming no entity
//! (I402) (ADR 0329 D6, ADR 0347 D11).

use specforge_code_facts::{ItemKind, Visibility};
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
const TEST_SCHEMES: [&str; 2] = ["js", "rust"];
/// The scheme of a `test_run`'s id.
const RUN_SCHEME: &str = "run";
/// The schemes of an `item`'s id.
const ITEM_SCHEMES: [&str; 2] = ["rust", "js"];
/// The scheme of a `package`'s id.
const PACKAGE_SCHEME: &str = "pkg";

/// The field holding a `test`'s result (`result`), and a `test_run`'s status.
const RESULT: &str = "status";
/// The field holding the ids of the entities a `test` or an `item` says it proves.
const NAMES: &str = "names";

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
                    "The entities the test says it proves, in its runner's idiom (I402 when one is no entity)",
                );
            });
        });
        c.kind(RUN_KIND, |k| {
            k.description(
                "The run of a test (ADR 0329): run:<test id>, with the status the run gave it",
            )
            .collected(&[RUN_SCHEME])
            .field("test", |f| {
                f.field_type(FieldType::Reference)
                    .target_kind(TEST_KIND)
                    .edge("run_of")
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
                f.field_type(FieldType::Reference)
                    .target_kind(PACKAGE_KIND)
                    .edge("in_package")
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
                f.field_type(FieldType::Reference)
                    .target_kind(TEST_KIND)
                    .edge("runs_as")
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
                f.field_type(FieldType::ReferenceList)
                    .target_kind(PACKAGE_KIND)
                    .edge("uses_package")
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

        c.pass("refs", |p| {
            p.phase("check").run(refs::refs);
        });
        c.pass("proof", |p| {
            p.phase("check").run(proof::proof);
        });

        c.code("E101", |d| {
            d.title("A fact reference names no fact").explanation(
                "A reference written in a test fact's scheme (`js:<package>/<file>#<name>`) names a test that the \
                 fact stores the session served do not hold, while a store does hold facts of that scheme: the test \
                 was renamed, moved or deleted, or the reference is misspelled. The finding suggests the closest \
                 test of the same file. It is not reported while the citing entity's owner is pending (ADR 0329 D4): \
                 a test not written yet is TDD. Fix the reference, or run `specforge collect` if the test is new. \
                 A reference one of its own facts holds (a test run's test) that no fact or entity answers, or \
                 that names an entity of another kind, is reported at the place its collector read it, addressed \
                 to that collector: its facts disagree, and running `specforge collect` again, or fixing the runner \
                 pack, is the remedy (ADR 0346).",
            );
        });
        c.code("W102", |d| {
            d.title("No facts of the reference's scheme were collected").explanation(
                "A reference written in a test fact's scheme (`js:…`) names no fact, and no fact store the session \
                 served holds facts of that scheme: on a fresh clone, or before the runner pack's collector ran, \
                 there is nothing to check it against. It is reported once per cited reference, and not while the \
                 citing entity's owner is pending. Run `specforge collect` with the runner pack that writes that \
                 scheme (`@javascript/node-test` for `js`).",
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
