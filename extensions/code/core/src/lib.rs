//! @code/core — the neutral base of test facts (ADR 0329).
//!
//! It declares the facts a runner pack collects, and judges the references a spec writes to them:
//!
//! - `test` (scheme `js`): a test a runner ran, `js:<package>/<file>#<s1 > … > name>`, with its package, file, suite
//!   path, name and `status`, its result (`result("status")`), and the entities it says it proves (`names`);
//! - `test_run` (scheme `run`): the run of a test, `run:<test id>`, with the `test` it ran (edge `run_of`), the same
//!   `status` and its `duration_ms`.
//!
//! Both are collector-written and neither is scoped: a scheme reference names the fact whole. `@code/core` declares
//! no collector: a runner pack (`@javascript/node-test`) collects, minting the facts with the linked
//! `specforge-code-junit` library, so every pack names a test the same way. It is a builtin outside `@specforge`, so
//! it catalogues its codes and they print namespaced: `code/E101` (ADR 0328 D2).
//!
//! Its check pass `refs` judges a reference in its schemes that no fact holds (E101, or W102 when no store holds
//! facts of that scheme), unless the citing entity is pending (ADR 0329 D4, D5); its check pass `proof` reports a
//! complete entity linking a failed test (W401) and a test naming no entity (I402) (D6).

use specforge_extension_sdk::prelude::*;

mod proof;
mod refs;

/// The kind of a test fact.
const TEST_KIND: &str = "test";
/// The kind of the run of a test.
const RUN_KIND: &str = "test_run";
/// The scheme of a `test`'s id.
const TEST_SCHEME: &str = "js";
/// The scheme of a `test_run`'s id.
const RUN_SCHEME: &str = "run";

/// The field holding a `test`'s result (`result`), and a `test_run`'s status.
const RESULT: &str = "status";
/// The field holding the ids of the entities a `test` says it proves.
const NAMES: &str = "names";

/// A test's status: the `TestStatus` names, the one vocabulary of the collect wire.
const STATUS: Vocabulary = Vocabulary::new(&["passed", "failed", "skipped"]);

#[specforge_extension_sdk::extension(
    name = "@code/core",
    version = "1.0.0",
    description = "The neutral base of test facts: the test and test_run kinds a runner pack collects, and the checks of the references a spec writes to them"
)]
struct CodeCore;

impl Contributions for CodeCore {
    fn contribute(c: &mut ContributionsBuilder) {
        c.kind(TEST_KIND, |k| {
            k.description(
                "A test a runner ran, written by its runner pack's collector (ADR 0329): cite it by its id, \
                 js:<package>/<file>#<suite > … > name>",
            )
            .collected(&[TEST_SCHEME])
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
            .field(RESULT, |f| {
                f.vocabulary(&STATUS)
                    .description("What its last run said: passed, failed or skipped");
            })
            .field(NAMES, |f| {
                f.field_type(FieldType::StringList).description(
                    "The ids of the entities the test says it proves, in its runner's idiom (I402 when one is \
                     no entity)",
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
        c.edge("run_of", |e| {
            e.description("A run of a test")
                .source_kind(RUN_KIND)
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
                 a test not written yet is TDD. Fix the reference, or run `specforge collect` if the test is new.",
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
            d.title("A test names an entity that does not exist").explanation(
                "A test fact's `names` (the entities its runner's idiom says it proves) holds an id no entity of the \
                 project has: the entity was renamed or deleted, or the name is misspelled. Fix the name in the \
                 test, then collect again.",
            );
        });
    }
}

specforge_extension_sdk::component_guest!(build = specforge_extension_build);

#[cfg(test)]
mod tests;
